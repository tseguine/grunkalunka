import assert from "node:assert/strict";
import test from "node:test";
import { probeDiagnosticIsolation, verifyDiagnosticIsolation } from "../../tools/browser-oracles/diagnostic-isolation.mjs";

const viewport = { width: 800, height: 600 };
function measurement() {
  const original = { viewport: [800, 600, 1], root: [800, 600, 800, 600], body: [800, 90],
    scroll: [0, 0], boxes: [["block", 19, 19, 119, 34]] };
  return { originalHidden: true, outputRects: 0, visibleRects: 1, payloadVersion: 1, textPreserved: true,
    original, empty: structuredClone(original), enlarged: structuredClone(original),
    restored: structuredClone(original), rehidden: structuredClone(original),
    visibleControl: { ...structuredClone(original), root: [800, 600, 800, 65536] } };
}

test("isolated diagnostics preserve live geometry and require a sensitive control", () => {
  const result = measurement();
  assert.doesNotThrow(() => verifyDiagnosticIsolation(result, viewport));
  result.visibleControl.root[3] = result.original.root[3];
  assert.throws(() => verifyDiagnosticIsolation(result, viewport), /increase scroll height/);
});

test("each diagnostic write phase must leave the live layout unchanged", () => {
  for (const phase of ["empty", "enlarged", "restored", "rehidden"]) {
    const result = measurement(); result[phase].root[2] += 1;
    assert.throws(() => verifyDiagnosticIsolation(result, viewport), /changed layout/);
  }
  const result = measurement(); result.enlarged.boxes[0][3] += 1;
  assert.throws(() => verifyDiagnosticIsolation(result, viewport), /changed layout/);
});

test("transport, viewport and nonrendering conditions cannot silently disappear", () => {
  for (const change of [
    r => { r.originalHidden = false; }, r => { r.outputRects = 1; },
    r => { r.payloadVersion = 2; }, r => { r.textPreserved = false; },
    r => { r.visibleRects = 0; }, r => { delete r.restored; },
    r => { r.original.viewport[0] = 900; }
  ]) {
    const result = measurement(); change(result);
    assert.throws(() => verifyDiagnosticIsolation(result, viewport));
  }
});

test("probe closes pages and browser even when navigation fails", async () => {
  const events = [];
  const page = { goto: async () => { throw new Error("navigation failed"); }, close: async () => events.push("page-close") };
  const browserType = { launch: async () => ({ newPage: async () => page, close: async () => events.push("browser-close") }) };
  await assert.rejects(probeDiagnosticIsolation(browserType, "fixture.html"), /navigation failed/);
  assert.deepEqual(events, ["page-close", "browser-close"]);
});

test("probe collects both requested viewports without claiming verification", async () => {
  const configs = [], events = [];
  const browserType = { launch: async () => ({
    newPage: async config => {
      configs.push(config);
      return { goto: async () => {}, evaluate: async () => measurement(), close: async () => events.push("page-close") };
    }, close: async () => events.push("browser-close")
  }) };
  const results = await probeDiagnosticIsolation(browserType, "fixture.html");
  assert.deepEqual(configs.map(c => c.viewport), [{ width: 800, height: 600 }, { width: 320, height: 64 }]);
  assert.equal(results.length, 2);
  assert.deepEqual(events, ["page-close", "page-close", "browser-close"]);
});
