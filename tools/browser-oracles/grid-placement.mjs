import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// CSS Grid definite track sizes, row-major auto-placement in order-modified order.
// This independent model is not injected into the browser under test.
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const states = [
  ["initial", [40, 70], 10, 5, 0, "", "", "", ""],
  ["widen-first-track", [50, 60], 10, 5, 0, "50px 60px", "", "", ""],
  ["restore-tracks", [40, 70], 10, 5, 0, "40px 70px", "", "", ""],
  ["increase-column-gap", [40, 70], 25, 5, 0, "40px 70px", "25px", "", ""],
  ["increase-row-gap", [40, 70], 25, 15, 0, "40px 70px", "25px", "15px", ""],
  ["order-delta-first", [40, 70], 25, 15, -1, "40px 70px", "25px", "15px", "-1"],
  ["restore-order", [40, 70], 25, 15, 0, "40px 70px", "25px", "15px", ""],
  ["restore-gaps", [40, 70], 10, 5, 0, "40px 70px", "10px", "5px", ""],
  ["three-columns", [40, 50, 30], 10, 5, 0, "40px 50px 30px", "10px", "5px", ""],
  ["restore-two-columns", [40, 70], 10, 5, 0, "40px 70px", "10px", "5px", ""]
];
const mutations = {
  "skip-track-change": ["widen-first-track", '  grid.style.setProperty("grid-template-columns", "50px 60px");',
    '  // Intentionally skip the first-track change.'],
  "skip-row-gap": ["increase-row-gap", '  grid.style.setProperty("row-gap", "15px");',
    '  // Intentionally skip the row-gap increase.'],
  "skip-order": ["order-delta-first", '  delta.style.setProperty("order", "-1");',
    '  // Intentionally skip the order mutation.']
};
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");

export function gridExpected(defect = null) {
  assert.ok(defect === null || Object.hasOwn(mutations, defect), "known mutation");
  return states.map(([id, baselineTracks, baselineColumnGap, baselineRowGap, baselineOrder,
    baselineInlineColumns, baselineInlineColumnGap, baselineInlineRowGap, baselineInlineOrder]) => {
    let tracks = baselineTracks, columnGap = baselineColumnGap, rowGap = baselineRowGap;
    let deltaOrder = baselineOrder, inlineColumns = baselineInlineColumns;
    let inlineRowGap = baselineInlineRowGap, inlineOrder = baselineInlineOrder;
    if (defect === "skip-track-change" && id === "widen-first-track") {
      tracks = [40, 70]; inlineColumns = "";
    }
    if (defect === "skip-row-gap" && ["increase-row-gap", "order-delta-first", "restore-order"].includes(id)) {
      rowGap = 5; inlineRowGap = "";
    }
    if (defect === "skip-order" && id === "order-delta-first") {
      deltaOrder = 0; inlineOrder = "";
    }
    const ids = ["alpha", "beta", "gamma", "delta"];
    const ordered = deltaOrder < 0 ? ["delta", "alpha", "beta", "gamma"] : ids;
    const boxes = new Map();
    ordered.forEach((name, index) => {
      const col = index % tracks.length, row = Math.floor(index / tracks.length);
      const x = tracks.slice(0, col).reduce((sum, size) => sum + size, 0) + col * columnGap;
      boxes.set(name, { x, y: row * (20 + rowGap), width: tracks[col], height: 20 });
    });
    return { id, container: {
      computed: { display: "grid", width: "160px", height: "120px",
        gridTemplateColumns: tracks.map(x => `${x}px`).join(" "), gridAutoRows: "20px",
        gridAutoFlow: "row", columnGap: `${columnGap}px`, rowGap: `${rowGap}px`,
        justifyItems: "stretch", alignItems: "stretch" },
      inlineColumns, inlineColumnGap: baselineInlineColumnGap, inlineRowGap,
      rect: { x: 0, y: 0, width: 160, height: 120 }
    }, items: ids.map(name => {
      const rect = boxes.get(name);
      return { id: name, text: "",
        computed: { display: "block", width: `${rect.width}px`, height: "20px",
          order: String(name === "delta" ? deltaOrder : 0), gridColumnStart: "auto", gridRowStart: "auto" },
        inlineOrder: name === "delta" ? inlineOrder : "", rect };
    }) };
  });
}

export function verifyGrid(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "source hash format");
  assert.equal(capture?.schemaVersion, 1, "schema");
  assert.equal(capture.fixtureId, "grid-placement-v1", "fixture");
  assert.equal(capture.observationKind, "grid-placement", "kind");
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
  assert.ok(Array.isArray(capture.observations), "complete observations");
  const expected = gridExpected(defect);
  assert.deepEqual(capture.observations.map(x => x?.id), expected.map(x => x.id), "phase sequence");
  for (let i = 0; i < expected.length; i++)
    assert.deepEqual(capture.observations[i], expected[i], `${expected[i].id}: complete grid placement and CSSOM`);
}

export function mutateGrid(source, kind) {
  assert.ok(Object.hasOwn(mutations, kind), "known mutation");
  const [, before, after] = mutations[kind];
  assert.equal(source.split(before).length, 2, `${kind}: unique source site`);
  return source.replace(before, after);
}

export async function runGrid(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/grid-placement.html", import.meta.url));
  await mkdir(outputDir); // Exclusive; never mix old and new receipts.
  const take = async (bytes, name) => {
    const path = join(outputDir, `${name}.html`);
    await writeFile(path, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(path);
    await writeFile(join(outputDir, `${name}.json`), JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
    return capture;
  };
  const sourceHash = sha256(source);
  const first = await take(source, "first"); verifyGrid(first, engine, sourceHash);
  const repeat = await take(source, "repeat"); verifyGrid(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "repeat provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh context stability");
  const controls = [];
  for (const [kind, [phase]] of Object.entries(mutations)) {
    const changed = mutateGrid(source.toString("utf8"), kind), changedHash = sha256(changed);
    const capture = await take(changed, kind);
    verifyGrid(capture, engine, changedHash, kind); // All alternate phases, not one changed coordinate.
    assert.throws(() => verifyGrid(capture, engine, changedHash), error =>
      error.code === "ERR_ASSERTION" && error.message.includes(`${phase}: complete grid placement`));
    controls.push({ kind, phase, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash, phases: states.length, repeat: "equal", mutations: controls };
  await writeFile(join(outputDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n", { flag: "wx" });
  return summary;
}
