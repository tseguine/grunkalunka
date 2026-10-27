import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Spec-derived fixed-size flex model, independent of the HTML fixture's capture.
// The row has 300px content size; two gaps; 50/80 and 30px fixed flex bases.
// The middle item's zero basis absorbs all positive free space when grow=1.
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const states = [
  ["initial", 50, 1, 10, "row", "", "", "", ""],
  ["no-growth", 50, 0, 10, "row", "", "0", "", ""],
  ["restore-growth", 50, 1, 10, "row", "", "", "", ""],
  ["increase-basis", 80, 1, 10, "row", "80px", "", "", ""],
  ["increase-gap", 80, 1, 20, "row", "80px", "", "20px", ""],
  ["reverse-axis", 80, 1, 20, "row-reverse", "80px", "", "20px", "row-reverse"],
  ["restore-gap", 80, 1, 10, "row-reverse", "80px", "", "10px", "row-reverse"],
  ["restore-axis", 80, 1, 10, "row", "80px", "", "10px", "row"],
  ["restore-basis", 50, 1, 10, "row", "", "", "10px", "row"]
];
const mutations = {
  "skip-grow": ["no-growth", '  beta.style.setProperty("flex-grow", "0");', '  // Intentionally skip flex-grow update.'],
  "skip-gap": ["increase-gap", '  row.style.setProperty("gap", "20px");', '  // Intentionally skip gap update.'],
  "skip-reverse": ["reverse-axis", '  row.style.setProperty("flex-direction", "row-reverse");', '  // Intentionally skip axis reversal.']
};
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");

export function flexExpected(defect = null) {
  assert.ok(defect === null || Object.hasOwn(mutations, defect));
  return states.map(([id, alphaBasis0, betaGrow0, gap0, direction0, aInline, bInline0, rowGap0, rowDir0]) => {
    let alphaBasis = alphaBasis0, betaGrow = betaGrow0, gap = gap0, direction = direction0;
    let bInline = bInline0, rowGap = rowGap0, rowDir = rowDir0;
    if (defect === "skip-grow" && id === "no-growth") { betaGrow = 1; bInline = ""; }
    if (defect === "skip-gap" && ["increase-gap", "reverse-axis"].includes(id)) {
      gap = 10; rowGap = "";
    }
    if (defect === "skip-reverse" && ["reverse-axis", "restore-gap"].includes(id)) {
      direction = "row"; rowDir = "";
    }
    const widths = [alphaBasis, betaGrow ? 300 - alphaBasis - 30 - 2 * gap : 0, 30];
    const xs = direction === "row" ?
      [0, widths[0] + gap, widths[0] + gap + widths[1] + gap] :
      [300 - widths[0], 300 - widths[0] - gap - widths[1], 0];
    const items = ["alpha", "beta", "gamma"].map((name, index) => ({
      id: name, text: "", computed: { display: "block", width: `${widths[index]}px`,
        height: "20px", flexGrow: String(index === 1 ? betaGrow : 0),
        flexBasis: index === 0 ? `${alphaBasis}px` : index === 1 ? "0px" : "30px",
        minWidth: "0px" },
      rect: { x: xs[index], y: 0, width: widths[index], height: 20 },
      inlineBasis: index === 0 ? aInline : "", inlineGrow: index === 1 ? bInline : ""
    }));
    return { id, container: {
      computed: { display: "flex", width: "300px", height: "40px", flexDirection: direction, gap: `${gap}px` },
      inlineGap: rowGap, inlineDirection: rowDir, rect: { x: 0, y: 0, width: 300, height: 40 }
    }, items };
  });
}

export function verifyFlex(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "source hash format");
  assert.equal(capture?.schemaVersion, 1);
  assert.equal(capture.fixtureId, "flex-mutations-v1");
  assert.equal(capture.observationKind, "flex-layout");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.collector, "playwright");
  assert.equal(p.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.engine, engine); assert.equal(p.engineFamily, families[engine]);
  for (const key of ["playwrightVersion", "browserVersion", "nodeVersion", "hostPlatform", "hostArchitecture"])
    assert.ok(typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded", key);
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(e?.viewportWidth, 800); assert.equal(e.viewportHeight, 600); assert.equal(e.devicePixelRatio, 1);
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)));
  for (const key of ["userAgent", "url"]) assert.ok(typeof e[key] === "string" && e[key].length, key);
  assert.ok(Array.isArray(capture.observations), "complete observation array");
  const expected = flexExpected(defect);
  assert.deepEqual(capture.observations.map(x => x?.id), expected.map(x => x.id), "phase sequence");
  for (let i = 0; i < expected.length; i++) {
    assert.deepEqual(capture.observations[i], expected[i], `${expected[i].id}: expected flex geometry and DOM state`);
  }
}

export function mutateFlex(source, kind) {
  assert.ok(Object.hasOwn(mutations, kind), "known mutation");
  const [, before, replacement] = mutations[kind];
  assert.equal(source.split(before).length, 2, `${kind}: exactly one source site`);
  return source.replace(before, replacement);
}

export async function runFlex(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/flex-mutations.html", import.meta.url));
  await mkdir(outputDir);
  const take = async (bytes, name) => {
    const file = join(outputDir, `${name}.html`);
    await writeFile(file, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(file);
    await writeFile(join(outputDir, `${name}.json`), JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
    return capture;
  };
  const sourceHash = sha256(source);
  const first = await take(source, "first"); verifyFlex(first, engine, sourceHash);
  const repeat = await take(source, "repeat"); verifyFlex(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "repeat provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-context stability");
  const defects = [];
  for (const [kind, [phase]] of Object.entries(mutations)) {
    const changed = mutateFlex(source.toString("utf8"), kind), changedHash = sha256(changed);
    const capture = await take(changed, kind);
    verifyFlex(capture, engine, changedHash, kind); // Full alternate trace, not merely one changed width.
    assert.throws(() => verifyFlex(capture, engine, changedHash), error =>
      error.code === "ERR_ASSERTION" && error.message.includes(`${phase}: expected flex`));
    defects.push({ kind, phase, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash, phases: states.length, repeat: "equal", mutations: defects };
  await writeFile(join(outputDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n", { flag: "wx" });
  return summary;
}
