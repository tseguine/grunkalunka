import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Independent expectations based on DOM listener registration/invocation rules.
// Never import an observation or oracle table from the HTML under test.
const scenarios = ["remove-during-dispatch", "add-during-dispatch", "nested-once", "passive-cancel"];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const defects = {
  "skip-remove": ["remove-during-dispatch", 1,
    'target.removeEventListener("probe", later);', 'void 0;'],
  "skip-add": ["add-during-dispatch", 2,
    'target.addEventListener("probe", added);', 'void 0;'],
  "skip-once": ["nested-once", 1, '{ once: true }', '{ once: false }'],
  "disable-passive": ["passive-cancel", 1, '{ passive: true }', '{ passive: false }']
};
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const after = canceled => ({ target: "target", currentTarget: null, phase: 0,
  bubbles: true, cancelable: true, defaultPrevented: canceled, cancelBubble: false,
  isTrusted: false, path: [] });

function expectedDispatch(id, dispatch, defect) {
  const canceled = id === "passive-cancel" && defect === "disable-passive";
  const order = id === "remove-during-dispatch" ?
    ["remover", ...(defect === "skip-remove" ? ["later"] : [])] :
    id === "add-during-dispatch" ?
      ["adder", "stable", ...(dispatch === 2 && defect !== "skip-add" ? ["added"] : [])] :
      id === "passive-cancel" ? ["passive", "observer"] :
        dispatch === 1 || defect === "skip-once" ?
          ["once", ...(defect === "skip-once" ? ["once:nested"] : []), "peer:nested", "peer"] : ["peer"];
  const trace = order.map(key => {
    const [listener, label = "outer"] = key.split(":");
    return { listener, invocation: label, target: "target", currentTarget: "target", phase: 2,
      bubbles: true, cancelable: true, defaultPrevented: canceled,
      cancelBubble: false, isTrusted: false, path: ["target"] };
  });
  const hasNested = id === "nested-once" && (dispatch === 1 || defect === "skip-once");
  return { dispatch, trace, returned: !canceled, after: after(canceled),
    nested: hasNested ? [{ returned: true, after: after(false) }] : [] };
}

export function verifyLifecycle(capture, engine, sourceHash, defect = null) {
  assert.ok(Object.hasOwn(families, engine), "known engine");
  assert.ok(defect === null || Object.hasOwn(defects, defect), "known defect");
  assert.match(sourceHash, /^[a-f0-9]{64}$/, "expected source SHA-256");
  assert.equal(capture?.schemaVersion, 1);
  assert.equal(capture.fixtureId, "listener-lifecycle-v1");
  assert.equal(capture.observationKind, "listener-lifecycle");
  const p = capture.provenance, e = capture.environment;
  assert.equal(p?.collector, "playwright");
  assert.equal(p.fixtureSha256, sourceHash, "source hash mismatch");
  assert.equal(p.engine, engine); assert.equal(p.engineFamily, families[engine]);
  for (const key of ["playwrightVersion", "browserVersion", "nodeVersion", "hostPlatform", "hostArchitecture"]) {
    assert.ok(typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded", key);
  }
  assert.deepEqual(p.viewport, { width: 800, height: 600, devicePixelRatio: 1 });
  assert.equal(e?.viewportWidth, 800); assert.equal(e.viewportHeight, 600); assert.equal(e.devicePixelRatio, 1);
  assert.ok(typeof e.timestamp === "string" && Number.isFinite(Date.parse(e.timestamp)));
  for (const key of ["userAgent", "url"]) assert.ok(typeof e[key] === "string" && e[key].length, key);
  assert.ok(Array.isArray(capture.observations));
  assert.deepEqual(capture.observations.map(o => o?.id), scenarios, "complete ordered scenarios");
  for (const { id, dispatches } of capture.observations) {
    assert.ok(Array.isArray(dispatches) && dispatches.length === 2, `${id}: two dispatches`);
    for (let i = 0; i < 2; i++) {
      assert.deepEqual(dispatches[i], expectedDispatch(id, i + 1, defect), `${id}/dispatch-${i + 1}`);
    }
  }
}

export function mutateLifecycle(source, kind) {
  assert.ok(Object.hasOwn(defects, kind), "known mutation");
  const [, , before, replacement] = defects[kind];
  assert.equal(source.split(before).length, 2, `${kind}: exactly one guarded mutation site`);
  return source.replace(before, replacement);
}

export async function runLifecycle(engine, { collect, outputDir }) {
  const source = await readFile(new URL("../../tests/browser-oracles/listener-lifecycle.html", import.meta.url));
  await mkdir(outputDir); // Exclusive directory; never mix executions.
  const take = async (bytes, name) => {
    const file = join(outputDir, `${name}.html`);
    await writeFile(file, bytes, { flag: "wx", mode: 0o444 });
    const capture = await collect(file);
    await writeFile(join(outputDir, `${name}.json`), JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });
    return capture; // Preserve raw evidence before verification, even if rejected.
  };
  const sourceHash = hash(source);
  const first = await take(source, "first"); verifyLifecycle(first, engine, sourceHash);
  const repeat = await take(source, "repeat"); verifyLifecycle(repeat, engine, sourceHash);
  assert.deepEqual(repeat.provenance, first.provenance, "fresh-instance provenance");
  assert.deepEqual(repeat.observations, first.observations, "fresh-instance trace stability");
  const mutations = [];
  for (const [kind, [scenario, dispatch]] of Object.entries(defects)) {
    const changed = mutateLifecycle(source.toString("utf8"), kind), changedHash = hash(changed);
    const capture = await take(changed, kind);
    verifyLifecycle(capture, engine, changedHash, kind); // Verify complete alternate expectation.
    assert.throws(() => verifyLifecycle(capture, engine, changedHash), error =>
      error.code === "ERR_ASSERTION" && error.message.includes(`${scenario}/dispatch-${dispatch}`));
    mutations.push({ kind, scenario, dispatch, sourceHash: changedHash, detected: true });
  }
  const summary = { engine, sourceHash, scenarios: scenarios.length, dispatchesPerScenario: 2,
    repeat: "equal", mutations };
  await writeFile(join(outputDir, "summary.json"), JSON.stringify(summary, null, 2) + "\n", { flag: "wx" });
  return summary;
}
