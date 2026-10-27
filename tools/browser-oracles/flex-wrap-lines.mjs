import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Normative fixed-size flex line collection; the oracle is never sent into the page.
// Lines have 150px available main-size; flex:0 0 60/90px prevents shrinking.
// With align-items/content:flex-start, each line is 20px high and separated by row-gap.
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const phases = [
  ["initial-nowrap", "nowrap", 60, 10, 10, "", "", "", ""],
  ["enable-wrap", "wrap", 60, 10, 10, "wrap", "", "", ""],
  ["widen-beta", "wrap", 90, 10, 10, "wrap", "90px", "", ""],
  ["restore-beta", "wrap", 60, 10, 10, "wrap", "", "", ""],
  ["increase-column-gap", "wrap", 60, 40, 10, "wrap", "", "40px", ""],
  ["increase-row-gap", "wrap", 60, 40, 25, "wrap", "", "40px", "25px"],
  ["restore-column-gap", "wrap", 60, 10, 25, "wrap", "", "10px", "25px"],
  ["disable-wrap", "nowrap", 60, 10, 25, "nowrap", "", "10px", "25px"],
  ["restore-wrap", "wrap", 60, 10, 25, "wrap", "", "10px", "25px"]
];
const mutations = {
  "skip-enable-wrap": ["enable-wrap", '  row.style.setProperty("flex-wrap", "wrap");\n  observe("enable-wrap");',
    '  // Intentionally leave wrapping disabled.\n  observe("enable-wrap");'],
  "skip-beta-basis": ["widen-beta", '  beta.style.setProperty("flex-basis", "90px");',
    '  // Intentionally skip the second item basis change.'],
  "skip-row-gap": ["increase-row-gap", '  row.style.setProperty("row-gap", "25px");',
    '  // Intentionally skip row-gap change.']
};
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");

export function wrapExpected(defect = null) {
  assert.ok(defect === null || Object.hasOwn(mutations, defect), "known defect");
  return phases.map(([id, baselineWrap, baselineBeta, columnGap, baselineRowGap,
    baselineInlineWrap, baselineInlineBasis, inlineColumnGap, baselineInlineRowGap]) => {
    let flexWrap = baselineWrap, betaBasis = baselineBeta, rowGap = baselineRowGap;
    let inlineWrap = baselineInlineWrap, inlineBasis = baselineInlineBasis, inlineRowGap = baselineInlineRowGap;
    if (defect === "skip-enable-wrap" && !["disable-wrap", "restore-wrap"].includes(id) && id !== "initial-nowrap") {
      flexWrap = "nowrap"; inlineWrap = "";
    }
    if (defect === "skip-beta-basis" && id === "widen-beta") {
      betaBasis = 60; inlineBasis = "";
    }
    if (defect === "skip-row-gap" && ["increase-row-gap", "restore-column-gap", "disable-wrap", "restore-wrap"].includes(id)) {
      rowGap = 10; inlineRowGap = "";
    }
    const widths = [60, betaBasis, 60];
    const positions = [];
    let x = 0, y = 0;
    for (const width of widths) {
      if (positions.length > 0 && flexWrap === "wrap" && x + columnGap + width > 150) {
        x = 0; y += 20 + rowGap;
      } else if (positions.length > 0) x += columnGap;
      positions.push([x, y]);
      x += width;
    }
    const items = ["alpha", "beta", "gamma"].map((name, index) => ({
      id: name, text: "", computed: { display: "block", width: `${widths[index]}px`,
        height: "20px", flexGrow: "0", flexShrink: "0", flexBasis: `${widths[index]}px` },
      inlineBasis: index === 1 ? inlineBasis : "",
      rect: { x: positions[index][0], y: positions[index][1], width: widths[index], height: 20 }
    }));
    return { id, container: {
      computed: { display: "flex", width: "150px", height: "120px", flexDirection: "row",
        flexWrap, alignContent: "flex-start", alignItems: "flex-start",
        columnGap: `${columnGap}px`, rowGap: `${rowGap}px` },
      inlineWrap, inlineColumnGap, inlineRowGap,
      rect: { x: 0, y: 0, width: 150, height: 120 }
    }, items };
  });
}

export function verifyWrap(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "source hash format");
  assert.equal(capture?.schemaVersion, 1, "schema");
  assert.equal(capture.fixtureId, "flex-wrap-lines-v1", "fixture");
  assert.equal(capture.observationKind, "flex-wrap-lines", "kind");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.collector, "playwright");
  assert.equal(p.engine, engine); assert.equal(p.engineFamily, families[engine]);
  for (const key of ["playwrightVersion", "browserVersion", "nodeVersion", "hostPlatform", "hostArchitecture"])
    assert.ok(typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded", key);
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(e?.viewportWidth, 800); assert.equal(e.viewportHeight, 600); assert.equal(e.devicePixelRatio, 1);
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)), "timestamp");
  for (const key of ["userAgent", "url"]) assert.ok(typeof e[key] === "string" && e[key].length, key);
  assert.ok(Array.isArray(capture.observations), "observations array required");
  const expected = wrapExpected(defect);
  assert.deepEqual(capture.observations.map(o => o?.id), expected.map(o => o.id), "phase sequence");
  for (let i = 0; i < expected.length; i++)
    assert.deepEqual(capture.observations[i], expected[i], `${expected[i].id}: complete flex line geometry and DOM state`);
}

export function mutateWrap(source, kind) {
  assert.ok(Object.hasOwn(mutations, kind), "known mutation");
  const [, before, after] = mutations[kind];
  assert.equal(source.split(before).length, 2, `${kind}: unique source site`);
  return source.replace(before, after);
}

export async function runWrap(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/flex-wrap-lines.html", import.meta.url));
  await mkdir(outputDir); // Exclusive: stale evidence must not contaminate a run.
  const take = async (bytes, name) => {
    const path = join(outputDir, `${name}.html`);
    await writeFile(path, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(path);
    await writeFile(join(outputDir, `${name}.json`), JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
    return capture;
  };
  const sourceHash = sha256(source);
  const first = await take(source, "first"); verifyWrap(first, engine, sourceHash);
  const repeat = await take(source, "repeat"); verifyWrap(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "repeat provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-context repeat");
  const controls = [];
  for (const [kind, [phase]] of Object.entries(mutations)) {
    const changed = mutateWrap(source.toString("utf8"), kind), changedHash = sha256(changed);
    const capture = await take(changed, kind);
    verifyWrap(capture, engine, changedHash, kind); // Validate full alternate trace first.
    assert.throws(() => verifyWrap(capture, engine, changedHash), error =>
      error.code === "ERR_ASSERTION" && error.message.includes(`${phase}: complete flex line`));
    controls.push({ kind, phase, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash, phases: phases.length, repeat: "equal", mutations: controls };
  await writeFile(join(outputDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n", { flag: "wx" });
  return summary;
}
