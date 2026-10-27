import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { crossEngineReport, reportExitCode } from "./cross-engine.mjs";

const source = "<!doctype html><p>independent test input</p>";
const hash = createHash("sha256").update(source).digest("hex");
function captures() {
  return Object.fromEntries(Object.entries({ chromium: "blink", firefox: "gecko", webkit: "webkit" }).map(([engine, engineFamily]) => [engine, {
    schemaVersion: 1, fixtureId: "test-v1",
    environment: { userAgent: engine, viewportWidth: 800, viewportHeight: 600,
      devicePixelRatio: 1, url: `file:///${engine}.html`, timestamp: "2026-10-27T10:00:00Z" },
    provenance: { collector: "playwright", engine, engineFamily, fixtureSha256: hash,
      browserVersion: `${engine}-test`, playwrightVersion: "test-pw", hostPlatform: "linux",
      hostArchitecture: "x64", nodeVersion: "test-node", viewport: { width: 800, height: 600, devicePixelRatio: 1 } },
    observations: [{ id: "box", text: "A B", computed: { width: "119px", display: "block" },
      rect: { x: 19, y: 19, width: 119, height: 34 } }]
  }]));
}

test("three-engine agreement retains raw inputs and does not assert conformance", () => {
  const input = captures(), before = structuredClone(input), result = crossEngineReport(input, hash);
  assert.equal(result.status, "equal");
  assert.equal(result.conformance, "not-assessed");
  assert.deepEqual(result.coverage, { expectedEngines: 3, presentEngines: 3, readyEngines: 3, comparedPairs: 3, expectedPairs: 3 });
  assert.deepEqual(result.pairs.map(p => p.engines), [["chromium", "firefox"], ["chromium", "webkit"], ["firefox", "webkit"]]);
  assert.deepEqual(result.engines.firefox.capture, before.firefox);
  assert.deepEqual(input, before);
  assert.equal(reportExitCode(result), 0);
});

test("raw numerical and CSS serialization differences are never normalized", () => {
  const input = captures();
  input.firefox.observations[0].rect.y += 0.5;
  input.webkit.observations[0].computed.width = "119.0px";
  const result = crossEngineReport(input, hash);
  assert.equal(result.status, "different");
  assert.equal(reportExitCode(result), 1);
  assert.equal(result.pairs[0].differences[0].right, 19.5);
  assert.equal(result.pairs[1].differences[0].right, "119.0px");
  assert.equal(result.pairs[2].differences.length, 2);
});

test("missing engines never count as agreement; unrelated pairs remain useful", () => {
  const input = captures();
  delete input.webkit;
  const result = crossEngineReport(input, hash);
  assert.equal(result.status, "incomplete");
  assert.equal(result.engines.webkit.status, "missing");
  assert.deepEqual(result.pairs.map(p => p.status), ["equal", "not-compared", "not-compared"]);
  assert.equal(result.coverage.comparedPairs, 1);
  assert.equal(reportExitCode(result), 2);
  assert.equal(crossEngineReport({}, hash).coverage.comparedPairs, 0);
});

test("invalid receipts, false engine identity and inconsistent viewport fail closed", () => {
  for (const corrupt of [
    c => { delete c.provenance; }, c => { c.provenance.engine = "chromium"; },
    c => { c.provenance.engineFamily = "blink"; }, c => { c.provenance.browserVersion = " "; },
    c => { c.provenance.playwrightVersion = "unrecorded"; }, c => { c.provenance.fixtureSha256 = "broken"; },
    c => { c.provenance.viewport.width = 801; }, c => { c.observations = []; },
    c => { c.observations[0].rect.width = NaN; }
  ]) {
    const input = captures(); corrupt(input.firefox);
    const result = crossEngineReport(input, hash);
    assert.equal(result.status, "invalid");
    assert.ok(result.engines.firefox.errors.length > 0);
    assert.equal(result.coverage.comparedPairs, 1);
  }
  const input = captures(); input.webkit = null;
  assert.equal(crossEngineReport(input, hash).engines.webkit.status, "invalid");
  assert.throws(() => crossEngineReport({ safari: {} }, hash), TypeError);
  assert.throws(() => crossEngineReport(captures(), "bad"), TypeError);
});

test("even unanimous receipt hashes must match independently supplied source", () => {
  const input = captures();
  for (const value of Object.values(input)) value.provenance.fixtureSha256 = "b".repeat(64);
  const result = crossEngineReport(input, hash);
  assert.equal(result.status, "incompatible");
  assert.equal(result.coverage.comparedPairs, 0);
  assert.equal(result.engines.chromium.errors[0].expected, hash);
});

test("different recorded hosts, sources and fixture identities are not compared", () => {
  for (const corrupt of [
    c => { c.provenance.hostPlatform = "darwin"; }, c => { c.provenance.hostArchitecture = "arm64"; },
    c => { c.provenance.nodeVersion = "other-node"; }, c => { c.provenance.playwrightVersion = "other-pw"; },
    c => { c.provenance.fixtureSha256 = "a".repeat(64); }, c => { c.fixtureId = "other"; },
    c => { c.environment.viewportWidth = c.provenance.viewport.width = 900; }
  ]) {
    const input = captures(); corrupt(input.webkit);
    const result = crossEngineReport(input, hash);
    assert.equal(result.status, "incompatible");
    assert.equal(result.coverage.comparedPairs, 1);
    assert.equal(result.pairs[0].status, "equal");
  }
});

test("CLI hashes source bytes and reports missing, malformed, and differing input", () => {
  const dir = mkdtempSync(join(tmpdir(), "browser-cross-engine-"));
  const cli = fileURLToPath(new URL("./cross-engine.mjs", import.meta.url));
  const html = join(dir, "source.html"), files = ["chromium", "firefox", "webkit"].map(e => join(dir, `${e}.json`));
  const input = captures();
  try {
    writeFileSync(html, source);
    Object.values(input).forEach((value, i) => writeFileSync(files[i], JSON.stringify(value)));
    const run = (...paths) => spawnSync(process.execPath, [cli, html, ...paths], { encoding: "utf8", timeout: 5000, maxBuffer: 1024 * 1024 });
    let result = run(...files);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stdout).expectedSourceSha256, hash);
    input.webkit.observations[0].text = "changed";
    writeFileSync(files[2], JSON.stringify(input.webkit));
    assert.equal(run(...files).status, 1);
    result = run(files[0], files[1], "-");
    assert.equal(result.status, 2); assert.equal(JSON.parse(result.stdout).engines.webkit.status, "missing");
    result = run(files[0], files[1], join(dir, "absent.json"));
    assert.equal(result.status, 2); assert.match(JSON.parse(result.stdout).inputs.captures.webkit.error, /ENOENT/);
    writeFileSync(files[2], "{");
    result = run(...files);
    assert.equal(result.status, 2); assert.equal(JSON.parse(result.stdout).engines.webkit.status, "invalid");
    writeFileSync(files[2], JSON.stringify(captures().webkit));
    writeFileSync(html, `${source}\n`);
    result = run(...files);
    assert.equal(result.status, 2); assert.equal(JSON.parse(result.stdout).status, "incompatible");
    assert.equal(run(...files, "extra").status, 2);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});
