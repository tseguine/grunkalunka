import assert from "node:assert/strict";
import test from "node:test";
import { verifyBaseline } from "./verify-baseline.mjs";

const hash = "a".repeat(64);
function receipt() {
  const computed = {
    display: "block", boxSizing: "content-box", width: "105px", height: "20px",
    marginTop: "4px", paddingLeft: "5px", borderLeftWidth: "2px",
    fontSize: "16px", lineHeight: "normal", whiteSpace: "normal"
  };
  return {
    schemaVersion: 1, fixtureId: "layout-baseline-v1",
    environment: { userAgent: "Test/1", url: "file:///snapshot.html",
      timestamp: "2026-10-27T09:00:00Z", viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1 },
    provenance: { collector: "playwright", playwrightVersion: "test-version", engine: "chromium",
      engineFamily: "blink", browserVersion: "test-browser", fixtureSha256: hash,
      hostPlatform: "test-os", hostArchitecture: "test-arch", nodeVersion: "test-node",
      viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations: [
      { id: "frame", text: "abcA B", computed: { ...computed, boxSizing: "border-box" },
        rect: { x: 0, y: 0, width: 240, height: 90 } },
      { id: "block", text: "abc", computed: { ...computed },
        rect: { x: 19, y: 19, width: 119, height: 34 } },
      { id: "inline", text: "A B", computed: { ...computed },
        rect: { x: 15, y: 60, width: 30, height: 18 } }
    ]
  };
}

test("independent selected baseline accepts a complete receipt", () => {
  for (const [engine, family] of Object.entries({ chromium: "blink", firefox: "gecko", webkit: "webkit" })) {
    const value = receipt();
    value.provenance.engine = engine;
    value.provenance.engineFamily = family;
    assert.doesNotThrow(() => verifyBaseline(value, engine, hash));
  }
});

test("every required property and observation must exist", () => {
  for (let i = 0; i < 3; i++) {
    for (const key of Object.keys(receipt().observations[i].computed)) {
      const missing = receipt();
      delete missing.observations[i].computed[key];
      assert.throws(() => verifyBaseline(missing, "chromium", hash), /missing/);
    }
    const missing = receipt();
    missing.observations.splice(i, 1);
    assert.throws(() => verifyBaseline(missing, "chromium", hash));
  }
});

test("wrong inputs, provenance and independently incorrect geometry fail", () => {
  const mutations = [
    r => { r.provenance.fixtureSha256 = "b".repeat(64); },
    r => { delete r.provenance; },
    r => { r.provenance.engine = "firefox"; },
    r => { r.provenance.engineFamily = "gecko"; },
    r => { r.provenance.browserVersion = ""; },
    r => { r.provenance.playwrightVersion = "unrecorded"; },
    r => { r.environment.devicePixelRatio = 2; },
    r => { r.provenance.viewport.width = 801; },
    r => { r.observations[1].rect.width += 1; },
    r => { r.observations[1].computed.width = "104px"; },
    r => { r.observations[0].rect.height = NaN; },
    r => { r.observations[2].text = "wrong"; }
  ];
  for (const mutate of mutations) {
    const value = receipt();
    mutate(value);
    assert.throws(() => verifyBaseline(value, "chromium", hash));
  }
  assert.throws(() => verifyBaseline(receipt(), "unknown", hash));
  assert.throws(() => verifyBaseline(receipt(), "chromium", "bad"));
});
