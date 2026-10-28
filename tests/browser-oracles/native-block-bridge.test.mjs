import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { compareNativeBlockBridge } from "./native-block-bridge.mjs";

const fixture = readFileSync(new URL("./native-block-bridge.html", import.meta.url));
const sha = createHash("sha256").update(fixture).digest("hex");
const geometry = [
  { x: 0, y: 0, width: 284, height: 86 },
  { x: 16, y: 16, width: 118, height: 42 },
  { x: 12, y: 58, width: 86, height: 16 },
];
const native = () => ({
  schemaVersion: 1, kind: "grunkalunka-native-block-geometry",
  fixtureId: "native-block-bridge-v1",
  viewport: { width: 800, height: 600, devicePixelRatio: 1 },
  geometry: Object.fromEntries(["outer", "first", "second"].map((id, i) => [id, { ...geometry[i] }])),
});
function browser() {
  return {
    schemaVersion: 1, fixtureId: "native-block-bridge-v1",
    environment: {
      userAgent: "MockBrowser/1", viewportWidth: 800, viewportHeight: 600,
      devicePixelRatio: 1, url: "file:///test.html", timestamp: "2026-10-28T10:00:00Z",
    },
    observations: ["outer", "first", "second"].map((id, i) => ({
      id, text: "",
      computed: {
        display: "block", boxSizing: "content-box", width: "260px", height: "24px",
        marginTop: "0px", paddingLeft: "10px", borderLeftWidth: "2px",
      },
      rect: { ...geometry[i] },
    })),
    provenance: {
      collector: "playwright", engine: "chromium", engineFamily: "blink",
      browserVersion: "1.2", playwrightVersion: "1.64.1",
      hostPlatform: "linux", hostArchitecture: "x64", nodeVersion: "v24.21.0",
      fixtureSha256: sha, viewport: { width: 800, height: 600, devicePixelRatio: 1 },
    },
  };
}

test("self-contained source keeps the manually projected dimensions and IDs", () => {
  const source = fixture.toString("utf8");
  for (const rule of [
    "#outer { width: 260px; padding: 10px; border: 2px",
    "#first { width: 100px; height: 24px; margin: 4px 0 0 4px;",
    "#second { width: 80px; height: 10px; margin: 0;",
    '<div id="outer"><div id="first"></div><div id="second"></div></div>',
  ]) {
    assert.ok(source.includes(rule), "fixture drift: " + rule);
  }
  assert.ok(!/\b(?:src|href)=["']https?:/i.test(source), "external assets are not modeled");
});

test("compares only independently resolved native border boxes, not all CSS", () => {
  const left = native(), right = browser();
  const report = compareNativeBlockBridge(left, right, fixture);
  assert.equal(report.status, "equal");
  assert.equal(report.conformance, "not-assessed");
  assert.equal(report.sourceSha256, sha);
  assert.equal(report.native, left);
  assert.equal(report.browser, right);
});

test("geometric changes remain differences, never normalized to browser consensus", () => {
  const right = browser();
  right.observations[1].rect.width += 0.25;
  const report = compareNativeBlockBridge(native(), right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences.map(d => [d.id, d.field, d.left, d.right]), [
    ["first", "width", 118, 118.25],
  ]);
});

test("source hashes, identities and viewport mismatches cannot be compared", () => {
  for (const change of [
    c => { c.provenance.fixtureSha256 = "0".repeat(64); },
    c => { c.environment.viewportWidth = 801; },
    c => { c.observations.reverse(); },
    c => { c.fixtureId = "other"; },
    c => { c.provenance.engineFamily = "wrong"; },
  ]) {
    const right = browser();
    change(right);
    const report = compareNativeBlockBridge(native(), right, fixture);
    assert.equal(report.status, "incompatible", JSON.stringify(report));
  }
});

test("malformed native evidence and incomplete browser fields fail", () => {
  const bad = native();
  bad.geometry.first.width = -1;
  assert.equal(compareNativeBlockBridge(bad, browser(), fixture).status, "invalid");
  const mismatched = native();
  delete mismatched.geometry.second;
  assert.equal(compareNativeBlockBridge(mismatched, browser(), fixture).status, "invalid");
  const c = browser();
  delete c.observations[0].computed.width;
  assert.equal(compareNativeBlockBridge(native(), c, fixture).status, "invalid");
  assert.throws(() => compareNativeBlockBridge(native(), browser(), "not bytes"), TypeError);
});

test("CLI distinguishes equal, different and invalid source receipts", () => {
  const temp = mkdtempSync(join(tmpdir(), "grunkalunka-native-bridge-"));
  const a = join(temp, "native.json"), b = join(temp, "browser.json");
  const cli = fileURLToPath(new URL("./native-block-bridge.mjs", import.meta.url));
  try {
    writeFileSync(a, JSON.stringify(native()));
    writeFileSync(b, JSON.stringify(browser()));
    const run = () => spawnSync(process.execPath, [cli, a, b], { encoding: "utf8" });
    let output = run();
    assert.equal(output.status, 0, output.stderr);
    assert.equal(JSON.parse(output.stdout).status, "equal");
    const altered = browser();
    altered.observations[2].rect.height++;
    writeFileSync(b, JSON.stringify(altered));
    output = run();
    assert.equal(output.status, 1, output.stderr);
    assert.equal(JSON.parse(output.stdout).status, "different");
    altered.provenance.fixtureSha256 = "f".repeat(64);
    writeFileSync(b, JSON.stringify(altered));
    output = run();
    assert.equal(output.status, 2, output.stderr);
    assert.equal(JSON.parse(output.stdout).status, "incompatible");
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
});
