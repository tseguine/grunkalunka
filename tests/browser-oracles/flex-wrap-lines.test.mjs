import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { mutateWrap, runWrap, verifyWrap, wrapExpected } from "../../tools/browser-oracles/flex-wrap-lines.mjs";

const source = await readFile(new URL("./flex-wrap-lines.html", import.meta.url), "utf8");
const hash = text => createHash("sha256").update(text).digest("hex");
// Independent handwritten geometry for every phase, not imported from wrapExpected.
// Columns: id, wrapping, second basis, column/row gap, inline wrap/basis/gaps, x and y.
const rows = [
  ["initial-nowrap", "nowrap", 60, 10, 10, "", "", "", "", [0, 70, 140], [0, 0, 0]],
  ["enable-wrap", "wrap", 60, 10, 10, "wrap", "", "", "", [0, 70, 0], [0, 0, 30]],
  ["widen-beta", "wrap", 90, 10, 10, "wrap", "90px", "", "", [0, 0, 0], [0, 30, 60]],
  ["restore-beta", "wrap", 60, 10, 10, "wrap", "", "", "", [0, 70, 0], [0, 0, 30]],
  ["increase-column-gap", "wrap", 60, 40, 10, "wrap", "", "40px", "", [0, 0, 0], [0, 30, 60]],
  ["increase-row-gap", "wrap", 60, 40, 25, "wrap", "", "40px", "25px", [0, 0, 0], [0, 45, 90]],
  ["restore-column-gap", "wrap", 60, 10, 25, "wrap", "", "10px", "25px", [0, 70, 0], [0, 0, 45]],
  ["disable-wrap", "nowrap", 60, 10, 25, "nowrap", "", "10px", "25px", [0, 70, 140], [0, 0, 0]],
  ["restore-wrap", "wrap", 60, 10, 25, "wrap", "", "10px", "25px", [0, 70, 0], [0, 0, 45]]
];
function handwritten(defect = null) {
  return rows.map(original => {
    const [id] = original;
    const cell = [...original];
    if (defect === "skip-enable-wrap" && ["enable-wrap", "widen-beta", "restore-beta",
      "increase-column-gap", "increase-row-gap", "restore-column-gap"].includes(id)) {
      cell[1] = "nowrap"; cell[5] = ""; cell[10] = [0, 0, 0];
      cell[9] = id === "widen-beta" ? [0, 70, 170] :
        ["increase-column-gap", "increase-row-gap"].includes(id) ? [0, 100, 200] : [0, 70, 140];
    }
    if (defect === "skip-beta-basis" && id === "widen-beta") {
      cell[2] = 60; cell[6] = ""; cell[9] = [0, 70, 0]; cell[10] = [0, 0, 30];
    }
    if (defect === "skip-row-gap" && ["increase-row-gap", "restore-column-gap", "disable-wrap", "restore-wrap"].includes(id)) {
      cell[4] = 10; cell[8] = "";
      if (id === "increase-row-gap") cell[10] = [0, 30, 60];
      if (id === "restore-column-gap" || id === "restore-wrap") cell[10] = [0, 0, 30];
    }
    const [_, flexWrap, betaBasis, columnGap, rowGap, inlineWrap, inlineBasis, inlineColumnGap,
      inlineRowGap, xs, ys] = cell;
    const widths = [60, betaBasis, 60];
    return { id, container: {
      computed: { display: "flex", width: "150px", height: "120px", flexDirection: "row", flexWrap,
        alignContent: "flex-start", alignItems: "flex-start", columnGap: `${columnGap}px`, rowGap: `${rowGap}px` },
      inlineWrap, inlineColumnGap, inlineRowGap, rect: { x: 0, y: 0, width: 150, height: 120 }
    }, items: ["alpha", "beta", "gamma"].map((name, i) => ({
      id: name, text: "", computed: { display: "block", width: `${widths[i]}px`,
        height: "20px", flexGrow: "0", flexShrink: "0", flexBasis: `${widths[i]}px` },
      inlineBasis: i === 1 ? inlineBasis : "", rect: { x: xs[i], y: ys[i], width: widths[i], height: 20 }
    })) };
  });
}
function receipt(bytes = source, defect = null, engine = "chromium") {
  return {
    schemaVersion: 1, fixtureId: "flex-wrap-lines-v1", observationKind: "flex-wrap-lines",
    provenance: { collector: "playwright", fixtureSha256: hash(bytes), engine,
      engineFamily: { chromium: "blink", firefox: "gecko", webkit: "webkit" }[engine],
      browserVersion: "mock-not-executed", playwrightVersion: "mock-not-executed",
      nodeVersion: "v22.16.0", hostPlatform: "linux", hostArchitecture: "x64",
      viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    environment: { userAgent: "synthetic unit fixture only", viewportWidth: 800,
      viewportHeight: 600, devicePixelRatio: 1, url: "file:///synthetic-only",
      timestamp: "2026-10-27T00:00:00.000Z" },
    observations: handwritten(defect)
  };
}

test("complete independent handwritten phase table matches normative model for all controls", () => {
  for (const defect of [null, "skip-enable-wrap", "skip-beta-basis", "skip-row-gap"]) {
    assert.deepEqual(wrapExpected(defect), handwritten(defect));
    const bytes = defect ? mutateWrap(source, defect) : source;
    assert.notEqual(defect ? hash(bytes) : "", defect ? hash(source) : "x");
    for (const engine of ["chromium", "firefox", "webkit"])
      verifyWrap(receipt(bytes, defect, engine), engine, hash(bytes), defect);
    if (defect) assert.throws(() => verifyWrap(receipt(bytes, defect), "chromium", hash(bytes)),
      error => error.code === "ERR_ASSERTION" && error.message.includes(({
        "skip-enable-wrap": "enable-wrap", "skip-beta-basis": "widen-beta", "skip-row-gap": "increase-row-gap"
      })[defect]));
  }
});

test("reject missing phases, wrong geometry, wrong provenance and malformed alternate captures", () => {
  const original = receipt();
  original.observations.pop();
  assert.throws(() => verifyWrap(original, "chromium", hash(source)), /phase sequence/);
  const geometry = receipt(); geometry.observations[1].items[2].rect.y = 999;
  assert.throws(() => verifyWrap(geometry, "chromium", hash(source)), /enable-wrap: complete flex line/);
  const provenance = receipt(); provenance.provenance.fixtureSha256 = "0".repeat(64);
  assert.throws(() => verifyWrap(provenance, "chromium", hash(source)), /source hash mismatch/);
  const defect = "skip-row-gap", bytes = mutateWrap(source, defect);
  const bogus = receipt(bytes, defect); bogus.observations[8].items[2].rect.y = 999;
  assert.throws(() => verifyWrap(bogus, "chromium", hash(bytes), defect), /restore-wrap: complete flex line/);
  assert.throws(() => mutateWrap(source, "not-a-defect"), /known mutation/);
  assert.throws(() => mutateWrap(source.replace('  row.style.setProperty("row-gap", "25px");', ''), defect), /unique source site/);
});

test("runner retains all sources, provenance and captures; requires real alternate whole-trace checks", async () => {
  const dir = await mkdtemp(join(tmpdir(), "wrap-oracle-"));
  const outputDir = join(dir, "evidence");
  try {
    const seen = [];
    const summary = await runWrap("chromium", { outputDir, collect: async file => {
      const bytes = await readFile(file, "utf8");
      const defect = ["skip-enable-wrap", "skip-beta-basis", "skip-row-gap"]
        .find(name => bytes.includes(`Intentionally ${({
          "skip-enable-wrap": "leave wrapping disabled", "skip-beta-basis": "skip the second item basis change",
          "skip-row-gap": "skip row-gap change"
        })[name]}`)) ?? null;
      seen.push({ bytes, defect });
      return receipt(bytes, defect);
    } });
    assert.equal(summary.phases, 9); assert.equal(summary.mutations.length, 3);
    assert.equal(summary.repeat, "equal"); assert.equal(seen.length, 5);
    assert.deepEqual(await readdir(outputDir), ["first.html", "first.json", "repeat.html", "repeat.json",
      "skip-beta-basis.html", "skip-beta-basis.json", "skip-enable-wrap.html",
      "skip-enable-wrap.json", "skip-row-gap.html", "skip-row-gap.json", "summary.json"]);
    for (const kind of ["first", "repeat", "skip-enable-wrap", "skip-beta-basis", "skip-row-gap"]) {
      const bytes = await readFile(join(outputDir, `${kind}.html`));
      const captured = JSON.parse(await readFile(join(outputDir, `${kind}.json`), "utf8"));
      assert.equal(captured.provenance.fixtureSha256, hash(bytes));
      verifyWrap(captured, "chromium", hash(bytes), kind.startsWith("skip-") ? kind : null);
    }
    await assert.rejects(() => runWrap("chromium", { outputDir, collect: async () => receipt() }),
      error => error.code === "EEXIST");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("collector failure preserves its source snapshot without claiming missing evidence passed", async () => {
  const dir = await mkdtemp(join(tmpdir(), "wrap-oracle-fail-"));
  try {
    const evidence = join(dir, "evidence");
    await assert.rejects(() => runWrap("chromium", { outputDir: evidence,
      collect: async () => { throw new Error("no browser was launched"); } }), /no browser was launched/);
    assert.deepEqual(await readdir(evidence), ["first.html"]);
    assert.equal(hash(await readFile(join(evidence, "first.html"))), hash(source));
  } finally { await rm(dir, { recursive: true, force: true }); }
});
