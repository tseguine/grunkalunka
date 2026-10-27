import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Independent expected observations, never imported by the browser fixture.
// Rules and derivations: ../../docs/browser-oracle-cascade.md
const phases = [
  ["initial", 40, "box", "", ""],
  ["normal-inline", 40, "box", "100px", ""],
  ["important-inline", 64, "box", "64px", "important"],
  ["remove-inline", 40, "box", "", ""],
  ["selected-class", 52, "box selected", "", ""],
  ["remove-selected", 40, "box", "", ""],
  ["remove-box", 80, "", "", ""],
  ["restore-box", 40, "box", "", ""]
];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const hash = bytes => createHash("sha256").update(bytes).digest("hex");

export function verifyCascade(capture, engine, sourceHash) {
  assert.ok(Object.hasOwn(families, engine), "known engine required");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "expected source hash");
  assert.equal(capture?.schemaVersion, 1, "capture schema");
  assert.equal(capture.fixtureId, "cascade-mutations-v1");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.collector, "playwright");
  assert.equal(p.engine, engine, "engine identity");
  assert.equal(p.engineFamily, families[engine], "engine family");
  for (const key of ["playwrightVersion", "browserVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]) {
    assert.equal(typeof p[key], "string", key);
    assert.ok(p[key].trim().length > 0 && p[key] !== "unrecorded", key);
  }
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(e?.viewportWidth, 800);
  assert.equal(e.viewportHeight, 600);
  assert.equal(e.devicePixelRatio, 1);
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)));
  for (const key of ["userAgent", "url"]) assert.ok(typeof e[key] === "string" && e[key].length > 0, key);
  assert.ok(Array.isArray(capture.observations), "observations array required");
  assert.deepEqual(capture.observations.map(o => o?.id), phases.map(p => p[0]), "complete ordered phases");
  for (const [i, [id, width, className, inlineWidth, inlinePriority]] of phases.entries()) {
    const o = capture.observations[i];
    assert.equal(o.text, "", `${id}: empty target`);
    assert.deepEqual(o.computed, { display: "block", boxSizing: "content-box", width: `${width}px`,
      height: "10px", paddingLeft: "2px", paddingRight: "2px", paddingTop: "2px", paddingBottom: "2px",
      borderLeftWidth: "1px", borderRightWidth: "1px", borderTopWidth: "1px", borderBottomWidth: "1px" }, `${id}: computed`);
    assert.deepEqual(o.rect, { x: 0, y: 0, width: width + 6, height: 16 }, `${id}: border rectangle`);
    assert.deepEqual(o.dom, { className, inlineWidth, inlinePriority }, `${id}: DOM state`);
  }
}

// Mutate source, not returned measurements. Guards make fixture drift explicit.
export function mutateCascade(source, kind) {
  const earlier = "  .box { width: 24px !important; }";
  const later = "  .box { width: 40px !important; }";
  const selected = '  target.classList.add("selected");';
  let before, after;
  if (kind === "reverse-order") [before, after] = [`${earlier}\n${later}`, `${later}\n${earlier}`];
  else if (kind === "drop-important") [before, after] = [`${earlier}\n${later}`, "  .box { width: 24px; }\n  .box { width: 40px; }"];
  else if (kind === "skip-class") [before, after] = [selected, "  // Deliberately skip the class mutation."];
  else throw new Error(`unknown cascade mutation: ${kind}`);
  assert.equal(source.split(before).length, 2, `${kind}: exactly one mutation site required`);
  return source.replace(before, after);
}

export async function runCascade(engine, { collect, outputDir }) {
  // collect uses the existing real-engine collector, supplied by smoke.mjs.
  const source = await readFile(new URL("../../tests/browser-oracles/cascade-mutations.html", import.meta.url));
  await mkdir(outputDir); // Exclusive run directory, never combine old and new evidence.
  const save = (name, value) => writeFile(join(outputDir, name), JSON.stringify(value, null, 2) + "\n", { flag: "wx" });
  const take = async (text, name) => {
    const path = join(outputDir, `${name}.html`);
    await writeFile(path, text, { flag: "wx", mode: 0o444 });
    const capture = await collect(path);
    await save(`${name}.json`, capture); // Preserve incorrect observations too.
    return capture;
  };
  const sourceHash = hash(source);
  const first = await take(source, "first");
  verifyCascade(first, engine, sourceHash);
  const repeat = await take(source, "repeat");
  verifyCascade(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "repeat provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-instance phase repeat");
  const mutations = [];
  for (const [kind, index, width] of [["reverse-order", 0, 24], ["drop-important", 0, 80], ["skip-class", 4, 40]]) {
    const changed = mutateCascade(source.toString("utf8"), kind);
    const captured = await take(changed, kind);
    assert.equal(captured.provenance?.fixtureSha256, hash(changed), "mutated source hash");
    // Match the semantic failure, not just a different source hash or malformed receipt.
    assert.throws(() => verifyCascade(captured, engine, hash(changed)),
      error => error.code === "ERR_ASSERTION" && error.message.includes(`${phases[index][0]}: computed`));
    assert.equal(captured.observations[index].computed.width, `${width}px`);
    assert.equal(captured.observations[index].rect.width, width + 6);
    mutations.push({ kind, phase: phases[index][0], width, sourceHash: hash(changed), detected: true });
  }
  const summary = { engine, sourceHash, phases: phases.length, repeat: "equal", mutations };
  await save("summary.json", summary);
  return summary;
}
