import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { gridExpected, mutateGrid, runGrid, verifyGrid } from "../../tools/browser-oracles/grid-placement.mjs";

const source = await readFile(new URL("./grid-placement.html", import.meta.url), "utf8");
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");
const first = [[0, 0, 40], [50, 0, 70], [0, 25, 40], [50, 25, 70]];
const widened = [[0, 0, 50], [60, 0, 60], [0, 25, 50], [60, 25, 60]];
const widerGap = [[0, 0, 40], [65, 0, 70], [0, 25, 40], [65, 25, 70]];
const tallerGap = [[0, 0, 40], [65, 0, 70], [0, 35, 40], [65, 35, 70]];
const reordered = [[65, 0, 70], [0, 35, 40], [65, 35, 70], [0, 0, 40]];
const threeCols = [[0, 0, 40], [50, 0, 50], [110, 0, 30], [0, 25, 40]];
// These expected rectangles are independently handwritten, not computed using gridExpected.
const rows = [
  ["initial", [40, 70], 10, 5, 0, "", "", "", "", first],
  ["widen-first-track", [50, 60], 10, 5, 0, "50px 60px", "", "", "", widened],
  ["restore-tracks", [40, 70], 10, 5, 0, "40px 70px", "", "", "", first],
  ["increase-column-gap", [40, 70], 25, 5, 0, "40px 70px", "25px", "", "", widerGap],
  ["increase-row-gap", [40, 70], 25, 15, 0, "40px 70px", "25px", "15px", "", tallerGap],
  ["order-delta-first", [40, 70], 25, 15, -1, "40px 70px", "25px", "15px", "-1", reordered],
  ["restore-order", [40, 70], 25, 15, 0, "40px 70px", "25px", "15px", "", tallerGap],
  ["restore-gaps", [40, 70], 10, 5, 0, "40px 70px", "10px", "5px", "", first],
  ["three-columns", [40, 50, 30], 10, 5, 0, "40px 50px 30px", "10px", "5px", "", threeCols],
  ["restore-two-columns", [40, 70], 10, 5, 0, "40px 70px", "10px", "5px", "", first]
];
function handwritten(defect = null) {
  return rows.map(original => {
    let [id, tracks, columnGap, rowGap, order, inlineColumns, inlineColumnGap,
      inlineRowGap, inlineOrder, boxes] = original;
    if (defect === "skip-track-change" && id === "widen-first-track") {
      tracks = [40, 70]; inlineColumns = ""; boxes = first;
    }
    if (defect === "skip-row-gap" && ["increase-row-gap", "order-delta-first", "restore-order"].includes(id)) {
      rowGap = 5; inlineRowGap = "";
      boxes = id === "order-delta-first" ?
        [[65, 0, 70], [0, 25, 40], [65, 25, 70], [0, 0, 40]] : widerGap;
    }
    if (defect === "skip-order" && id === "order-delta-first") {
      order = 0; inlineOrder = ""; boxes = tallerGap;
    }
    return { id, container: {
      computed: { display: "grid", width: "160px", height: "120px",
        gridTemplateColumns: tracks.map(n => `${n}px`).join(" "), gridAutoRows: "20px",
        gridAutoFlow: "row", columnGap: `${columnGap}px`, rowGap: `${rowGap}px`,
        justifyItems: "stretch", alignItems: "stretch" },
      inlineColumns, inlineColumnGap, inlineRowGap,
      rect: { x: 0, y: 0, width: 160, height: 120 }
    }, items: ["alpha", "beta", "gamma", "delta"].map((name, index) => {
      const [x, y, width] = boxes[index];
      return { id: name, text: "", computed: { display: "block", width: `${width}px`,
        height: "20px", order: String(name === "delta" ? order : 0),
        gridColumnStart: "auto", gridRowStart: "auto" },
      inlineOrder: name === "delta" ? inlineOrder : "", rect: { x, y, width, height: 20 } };
    }) };
  });
}
function receipt(bytes = source, defect = null, engine = "chromium") {
  return { schemaVersion: 1, fixtureId: "grid-placement-v1", observationKind: "grid-placement",
    provenance: { collector: "playwright", fixtureSha256: sha256(bytes), engine,
      engineFamily: { chromium: "blink", firefox: "gecko", webkit: "webkit" }[engine],
      browserVersion: "synthetic-not-executed", playwrightVersion: "synthetic-not-executed",
      nodeVersion: "v22.16.0", hostPlatform: "linux", hostArchitecture: "x64",
      viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    environment: { userAgent: "synthetic unit test only", viewportWidth: 800,
      viewportHeight: 600, devicePixelRatio: 1, url: "file:///synthetic-only",
      timestamp: "2026-10-27T00:00:00.000Z" }, observations: handwritten(defect) };
}

test("independently handwritten complete grid traces for every phase and source control", () => {
  for (const defect of [null, "skip-track-change", "skip-row-gap", "skip-order"]) {
    assert.deepEqual(gridExpected(defect), handwritten(defect));
    const bytes = defect ? mutateGrid(source, defect) : source;
    if (defect) assert.notEqual(sha256(bytes), sha256(source));
    for (const engine of ["chromium", "firefox", "webkit"])
      verifyGrid(receipt(bytes, defect, engine), engine, sha256(bytes), defect);
    if (defect) assert.throws(() => verifyGrid(receipt(bytes, defect), "chromium", sha256(bytes)),
      error => error.code === "ERR_ASSERTION" && error.message.includes(({
        "skip-track-change": "widen-first-track", "skip-row-gap": "increase-row-gap",
        "skip-order": "order-delta-first"
      })[defect]));
  }
});

test("reject partial phase records, altered coordinates, wrong source and incomplete mutated traces", () => {
  const truncated = receipt(); truncated.observations.pop();
  assert.throws(() => verifyGrid(truncated, "chromium", sha256(source)), /phase sequence/);
  const changed = receipt(); changed.observations[5].items[3].rect.x = 777;
  assert.throws(() => verifyGrid(changed, "chromium", sha256(source)), /order-delta-first: complete grid/);
  const wrongHash = receipt(); wrongHash.provenance.fixtureSha256 = "0".repeat(64);
  assert.throws(() => verifyGrid(wrongHash, "chromium", sha256(source)), /source hash mismatch/);
  const bytes = mutateGrid(source, "skip-row-gap"), alternate = receipt(bytes, "skip-row-gap");
  alternate.observations[6].items[2].rect.y = 777;
  assert.throws(() => verifyGrid(alternate, "chromium", sha256(bytes), "skip-row-gap"),
    /restore-order: complete grid/);
  assert.throws(() => mutateGrid(source, "invalid"), /known mutation/);
  assert.throws(() => mutateGrid(source.replace('  delta.style.setProperty("order", "-1");', ""),
    "skip-order"), /unique source site/);
});

test("runner retains sources, original and alternate captures and refuses evidence overwrite", async () => {
  const dir = await mkdtemp(join(tmpdir(), "grid-oracle-"));
  const outputDir = join(dir, "evidence");
  try {
    const visited = [];
    const summary = await runGrid("chromium", { outputDir, collect: async path => {
      const bytes = await readFile(path, "utf8");
      const defect = ["skip-track-change", "skip-row-gap", "skip-order"]
        .find(name => bytes.includes(`Intentionally ${({
          "skip-track-change": "skip the first-track change",
          "skip-row-gap": "skip the row-gap increase",
          "skip-order": "skip the order mutation"
        })[name]}`)) ?? null;
      visited.push(defect);
      return receipt(bytes, defect);
    } });
    assert.equal(summary.phases, 10);
    assert.deepEqual(summary.mutations.map(x => x.kind),
      ["skip-track-change", "skip-row-gap", "skip-order"]);
    assert.equal(summary.repeat, "equal"); assert.equal(visited.length, 5);
    assert.deepEqual(await readdir(outputDir), ["first.html", "first.json", "repeat.html", "repeat.json",
      "skip-order.html", "skip-order.json", "skip-row-gap.html", "skip-row-gap.json",
      "skip-track-change.html", "skip-track-change.json", "summary.json"]);
    for (const name of ["first", "repeat", "skip-track-change", "skip-row-gap", "skip-order"]) {
      const bytes = await readFile(join(outputDir, `${name}.html`));
      const capture = JSON.parse(await readFile(join(outputDir, `${name}.json`), "utf8"));
      assert.equal(capture.provenance.fixtureSha256, sha256(bytes));
      verifyGrid(capture, "chromium", sha256(bytes), name.startsWith("skip-") ? name : null);
    }
    await assert.rejects(() => runGrid("chromium", { outputDir, collect: async () => receipt() }),
      error => error.code === "EEXIST");
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("collector error retains the source, never reports nonexistent observations as passing", async () => {
  const dir = await mkdtemp(join(tmpdir(), "grid-fail-"));
  try {
    const outputDir = join(dir, "evidence");
    await assert.rejects(() => runGrid("chromium", { outputDir,
      collect: async () => { throw new Error("browser not launched"); } }), /browser not launched/);
    assert.deepEqual(await readdir(outputDir), ["first.html"]);
    assert.equal(sha256(await readFile(join(outputDir, "first.html"))), sha256(source));
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test("runner rejects a source defect whose non-target alternate phase has wrong geometry", async () => {
  const dir = await mkdtemp(join(tmpdir(), "grid-adversarial-"));
  try {
    const outputDir = join(dir, "evidence");
    await assert.rejects(() => runGrid("chromium", { outputDir, collect: async path => {
      const bytes = await readFile(path, "utf8");
      const defect = ["skip-track-change", "skip-row-gap", "skip-order"]
        .find(name => bytes.includes(`Intentionally ${({
          "skip-track-change": "skip the first-track change",
          "skip-row-gap": "skip the row-gap increase",
          "skip-order": "skip the order mutation"
        })[name]}`)) ?? null;
      const result = receipt(bytes, defect);
      if (defect === "skip-row-gap") result.observations[6].items[1].rect.x = 999;
      return result;
    } }), /restore-order: complete grid placement/);
    assert.ok((await readdir(outputDir)).includes("skip-row-gap.json"), "failed evidence retained");
    assert.ok(!(await readdir(outputDir)).includes("summary.json"), "failed case cannot pass");
  } finally { await rm(dir, { recursive: true, force: true }); }
});
