import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const source = readFileSync(new URL("../../.github/workflows/browser-native-bridge.yml", import.meta.url), "utf8").replace(/\r\n/g, "\n");

function pathsFor(event, next, workflow = source) {
  const prefix = "  " + event + ":\n";
  const start = workflow.indexOf(prefix);
  assert.ok(start >= 0, event + " trigger required");
  const end = workflow.indexOf("  " + next + ":", start + prefix.length);
  assert.ok(end > start, next + " trigger required");
  const section = workflow.slice(start, end);
  assert.ok(section.includes("branches: [master]"), "master trigger required");
  const match = section.match(/    paths:\n((?:      - '[^']+'\n)+)/);
  assert.ok(match, event + " must have known explicit paths");
  return match[1].trimEnd().split("\n").map(line => {
    const parsed = line.match(/^      - '([^']+)'$/);
    assert.ok(parsed, "unsupported workflow path entry");
    return parsed[1];
  });
}

const requiredPaths = [
  "crates/grunkalunka-layout/**",
  "tests/browser-oracles/native-block-bridge.html",
  "tests/browser-oracles/native-block-bridge.mjs",
  "tests/browser-oracles/native-block-bridge.test.mjs",
  "tests/browser-oracles/native-block-bridge-workflow.test.mjs",
  "tests/browser-oracles/compare.mjs",
  "tools/browser-oracles/collect.mjs",
  "tools/browser-oracles/native-bridge-sensitivity.mjs",
  "tools/browser-oracles/native-bridge-sensitivity.test.mjs",
  "tools/browser-oracles/native-hit-sensitivity.mjs",
  "tools/browser-oracles/native-hit-sensitivity.test.mjs",
  "tools/browser-oracles/native-height-sensitivity.mjs",
  "tools/browser-oracles/native-height-sensitivity.test.mjs",
  "tools/browser-oracles/hosted/native-bridge.test.mjs",
  "tools/browser-oracles/package.json",
  ".github/workflows/browser-native-bridge.yml",
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml"
];

function verifyNativeBridgeTriggers(workflow) {
  const pr = pathsFor("pull_request", "push", workflow);
  const push = pathsFor("push", "workflow_dispatch", workflow);
  assert.deepEqual(pr, push, "PR and push source filters must match");
  assert.deepEqual(pr, requiredPaths, "all native bridge and shared inputs must trigger");
  assert.ok(pr.every(p => !p.includes("*") || p.endsWith("/**")),
    "unsupported non-subtree wildcard");
}

test("all inputs to the native browser bridge retrigger both GitHub events", () => {
  verifyNativeBridgeTriggers(source);
});

test("both PR and push require the imported shared capture validator", () => {
  const comparator = readFileSync(new URL("./native-block-bridge.mjs", import.meta.url), "utf8");
  assert.match(comparator, /from ["\']\.\/compare\.mjs["\']/, "the comparator imports shared validation");
  const line = "      - \'tests/browser-oracles/compare.mjs\'\n";
  assert.equal(source.split(line).length, 3, "one exact shared-source path per event");
  assert.throws(() => verifyNativeBridgeTriggers(source.replace(line, "")));
  const i = source.lastIndexOf(line);
  assert.throws(() => verifyNativeBridgeTriggers(source.slice(0, i) + source.slice(i + line.length)));
});

test("real layout artifact feeds one engine's node:test parity and sensitivity suite", () => {
  assert.match(source, /needs: native/);
  assert.ok(source.includes("engine: [chromium, firefox, webkit]"));
  assert.match(source, /cargo run --locked --offline -p grunkalunka-layout --example native_block_bridge/);
  assert.ok(source.includes("actions/upload-artifact@330a01c490aca151604b8cf639adc76d48f6c5d4"));
  assert.ok(source.includes("actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0"));
  assert.ok(source.includes("node --test tools/browser-oracles/hosted/native-bridge.test.mjs"));
  assert.ok(source.includes("BROWSER_BRIDGE_NATIVE: target/native-bridge/native.json"));
  assert.ok(source.includes("BROWSER_ENGINE: ${{ matrix.engine }}"));
  for (const tool of ["native-hit-sensitivity", "native-height-sensitivity"]) {
    assert.ok(!source.includes("node tools/browser-oracles/" + tool + ".mjs"),
      "workflow must invoke the real test, not assemble test assertions inline");
  }
});

test("the hosted node:test suite checks parity before all three live mutations", () => {
  const hosted = readFileSync(new URL("../../tools/browser-oracles/hosted/native-bridge.test.mjs",
    import.meta.url), "utf8");
  const parity = hosted.indexOf("unchanged native geometry and DOM targets agree");
  const width = hosted.indexOf("checkMutation('native-bridge-sensitivity.mjs'");
  const hit = hosted.indexOf("checkMutation('native-hit-sensitivity.mjs'");
  const height = hosted.indexOf("checkMutation('native-height-sensitivity.mjs'");
  assert.ok(parity > 0 && parity < width && width < hit && hit < height,
    "retain baseline parity and all three independent real mutation controls");
  assert.ok(hosted.includes("compareNativeBlockBridge(native, capture, fixture)"));
  assert.ok(hosted.includes("assert.equal(report.status, 'equal'"));
  assert.ok(hosted.includes("writeFile(join(out, 'comparison.json')"));
  assert.ok(hosted.includes("JSON.stringify(capture"));
});

test("height mutation input edits independently retrigger PR and push browser evidence", () => {
  for (const file of [
    "tools/browser-oracles/native-height-sensitivity.mjs",
    "tools/browser-oracles/native-height-sensitivity.test.mjs",
  ]) {
    const entry = "      - '" + file + "'\n";
    assert.equal(source.split(entry).length, 3, file + " must appear in both events");
    assert.throws(() => verifyNativeBridgeTriggers(source.replace(entry, "")));
    const last = source.lastIndexOf(entry);
    assert.throws(() => verifyNativeBridgeTriggers(
      source.slice(0, last) + source.slice(last + entry.length)));
  }
});

test("real engines remain pinned and all report differences as test failures", () => {
  assert.match(source, /mcr\.microsoft\.com\/playwright:v1\.64\.1-noble@sha256:[a-f0-9]{64}/);
  assert.match(source, /node-version: '24\.21\.0'/);
  assert.match(source, /--user 1001/);
  assert.match(source, /fail-fast: false/);
  assert.match(source, /if: \$\{\{ !cancelled\(\) \}\}/);
  assert.ok(!source.includes("|| true"), "no unconditional passing of differences");
  assert.ok(!source.includes("continue-on-error: true"), "real observed mismatch must fail");
  assert.ok(!source.includes("playwright install"), "container already contains engine revisions");
});
