import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Specification-derived expectations, never imported into the observed fixture.
// See ../../docs/browser-oracle-events.md; no layout rectangles or browser voting.
const scenarios = ["ordinary", "once", "stop", "stop-immediate", "cancel", "noncancelable", "nonbubbling"];
const listeners = ["root-capture", "parent-capture", "parent-second", "target-capture",
  "target-first", "target-second", "parent-bubble", "root-bubble"];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const defects = {
  "skip-stop": ["stop", 1, 'if (id === "stop") event.stopPropagation();', 'if (id === "stop") void 0;'],
  "skip-cancel": ["cancel", 1, 'if (id === "cancel" || id === "noncancelable") event.preventDefault();',
    'if (id === "cancel" || id === "noncancelable") void 0;'],
  "skip-once": ["once", 2, '{ once: id === "once" }', '{ once: false }']
};
const hash = bytes => createHash("sha256").update(bytes).digest("hex");

function expectedDispatch(id, dispatch, defect) {
  const stopped = id === "stop" && defect !== "skip-stop";
  const immediate = id === "stop-immediate";
  const canceled = id === "cancel" && defect !== "skip-cancel";
  let order = listeners;
  if (stopped || immediate) order = listeners.slice(0, immediate ? 2 : 3);
  else if (id === "nonbubbling") order = listeners.slice(0, 6);
  else if (id === "once" && dispatch === 2 && defect !== "skip-once") {
    order = listeners.filter(name => name !== "target-first");
  }
  const trace = order.map(listener => {
    const index = listeners.indexOf(listener), currentTarget = listener.split("-")[0];
    return { listener, target: "target", currentTarget, phase: index < 3 ? 1 : index < 6 ? 2 : 3,
      defaultPrevented: canceled && index >= 4, cancelBubble: (stopped || immediate) && index >= 1,
      isTrusted: false, path: ["target", "parent", "root"] };
  });
  return { dispatch, bubbles: id !== "nonbubbling", cancelable: id !== "noncancelable", trace,
    returned: !canceled, after: { target: "target", currentTarget: null, phase: 0,
      defaultPrevented: canceled, cancelBubble: false, isTrusted: false, path: [] } };
}

export function verifyEvents(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine");
  assert.ok(defect === null || Object.hasOwn(defects, defect), "known defect control");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "expected source hash");
  assert.equal(capture?.schemaVersion, 1, "capture schema");
  assert.equal(capture.fixtureId, "event-dispatch-v1", "fixture identity");
  assert.equal(capture.observationKind, "event-dispatch", "event trace contract");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.collector, "playwright");
  assert.equal(p.engine, engine, "engine identity");
  assert.equal(p.engineFamily, families[engine], "engine family");
  for (const key of ["playwrightVersion", "browserVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]) {
    assert.ok(typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded", key);
  }
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(e?.viewportWidth, 800); assert.equal(e.viewportHeight, 600); assert.equal(e.devicePixelRatio, 1);
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)), "timestamp");
  for (const key of ["userAgent", "url"]) assert.ok(typeof e[key] === "string" && e[key].length, key);
  assert.ok(Array.isArray(capture.observations), "observations array");
  assert.deepEqual(capture.observations.map(o => o?.id), scenarios, "complete ordered scenarios");
  for (const { id, dispatches } of capture.observations) {
    assert.ok(Array.isArray(dispatches) && dispatches.length === 2, `${id}: two dispatches required`);
    for (let i = 0; i < 2; i++) {
      assert.deepEqual(dispatches[i], expectedDispatch(id, i + 1, defect), `${id}/dispatch-${i + 1}`);
    }
  }
}

export function mutateEvents(source, kind) {
  assert.ok(Object.hasOwn(defects, kind), "known source mutation");
  const [, , before, after] = defects[kind];
  assert.equal(source.split(before).length, 2, `${kind}: exactly one mutation site required`);
  return source.replace(before, after);
}

export async function runEvents(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/event-dispatch.html", import.meta.url));
  await mkdir(outputDir); // Exclusive directory: never mix evidence from different runs.
  const save = (name, data) => writeFile(join(outputDir, name), JSON.stringify(data, null, 2) + "\n", { flag: "wx" });
  const take = async (bytes, name) => {
    const path = join(outputDir, `${name}.html`);
    await writeFile(path, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(path);
    await save(`${name}.json`, capture); // Preserve evidence before any assertion.
    return capture;
  };
  const sourceHash = hash(source), first = await take(source, "first");
  verifyEvents(first, engine, sourceHash);
  const repeat = await take(source, "repeat");
  verifyEvents(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "repeat provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-instance event repeat");
  const mutations = [];
  for (const [kind, [id, dispatch]] of Object.entries(defects)) {
    const changed = mutateEvents(source.toString("utf8"), kind), changedHash = hash(changed);
    const captured = await take(changed, kind);
    // Verify the ENTIRE alternate trace before accepting a specific baseline failure.
    verifyEvents(captured, engine, changedHash, kind);
    assert.throws(() => verifyEvents(captured, engine, changedHash),
      error => error.code === "ERR_ASSERTION" && error.message.includes(`${id}/dispatch-${dispatch}`));
    mutations.push({ kind, scenario: id, dispatch, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash, scenarios: scenarios.length, dispatchesPerScenario: 2, repeat: "equal", mutations };
  await save("summary.json", summary);
  return summary;
}
