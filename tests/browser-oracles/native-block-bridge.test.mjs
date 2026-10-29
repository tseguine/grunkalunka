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
const geometryMap = () => Object.fromEntries(
  ["outer", "first", "second"].map((id, i) => [id, { ...geometry[i] }]));
const percentageHeightGeometry = () => ({
  "height-host": { x: 0, y: 96, width: 132, height: 92 },
  "height-first": { x: 6, y: 105, width: 36, height: 46 },
  "height-second": { x: 6, y: 156, width: 20, height: 20 },
});
const borderBoxGeometry = () => ({
  "border-host": { x: 0, y: 198, width: 120, height: 80 },
  "border-first": { x: 6, y: 207, width: 54, height: 34 },
  "border-second": { x: 6, y: 246, width: 27, height: 17 },
});
const scrollSample = scrollY => ({
  scrollX: 0, scrollY,
  targetRect: { x: 0, y: 928 - scrollY, width: 80, height: 30 },
  hitTarget: scrollY === 350 ? "scroll-target" : null,
});
const scrollViewport = () => ({
  documentHeight: 958,
  before: scrollSample(0),
  scrolled: scrollSample(350),
  restored: scrollSample(0),
});
const hidden = () => ({
  outer: { x: 0, y: 0, width: 284, height: 40 },
  first: null,
  second: { x: 12, y: 12, width: 86, height: 16 },
});
const originalHits = {
  "first-center": "first",
  "first-lower": "first",
  "second-original": "second",
  "left-margin": "outer",
  "outer-border": "outer",
  outside: null,
  "first-right": "first",
};
const hiddenHits = {
  "first-center": "second",
  "first-lower": "outer",
  "second-original": null,
  "left-margin": "second",
  "outer-border": "outer",
  outside: null,
  "first-right": "outer",
};
const hitTargets = () => ({
  original: { ...originalHits },
  hidden: { ...hiddenHits },
  restored: { ...originalHits },
});
const native = () => ({
  schemaVersion: 1, kind: "grunkalunka-native-block-geometry",
  fixtureId: "native-block-bridge-v1",
  viewport: { width: 800, height: 600, devicePixelRatio: 1 },
  geometry: geometryMap(),
  percentageHeights: percentageHeightGeometry(),
  borderBoxes: borderBoxGeometry(),
  scrollViewport: scrollViewport(),
  reflow: { hidden: hidden(), restored: geometryMap() },
  hitTargets: hitTargets(),
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
    reflow: { hidden: hidden(), restored: geometryMap() },
    hitTargets: hitTargets(),
    percentageHeights: percentageHeightGeometry(),
    borderBoxes: borderBoxGeometry(),
    scrollViewport: scrollViewport(),
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
    "#height-host { width: 120px; height: 80px; margin: 10px 0 0 0;",
    "#height-first { width: 30px; height: 50%; margin: 3px 0 5px 0;",
    "#height-second { width: 20px; height: 25%; margin: 0;",
    "#border-host { width: 120px; height: 80px; margin: 10px 0 0 0;",
    "#border-first { width: 50%; height: 50%; margin: 3px 0 5px 0;",
    "#border-second { width: 25%; height: 25%; margin: 0;",
    "#scroll-spacer { height: 650px; }",
    "#scroll-target { width: 80px; height: 30px; }",
    '<div id="outer"><div id="first"></div><div id="second"></div></div>',
    '<div id="height-host"><div id="height-first"></div><div id="height-second"></div></div>',
    '<div id="border-host"><div id="border-first"></div><div id="border-second"></div></div>',
    '<div id="scroll-spacer"></div><div id="scroll-target"></div>',
  ]) {
    assert.ok(source.includes(rule), "fixture drift: " + rule);
  }
  assert.ok(source.includes('first.style.display = "none"'), "live display:none phase must be measured");
  assert.ok(source.includes('first.style.removeProperty("display")'),
    "fixture must restore its original CSS state");
  assert.ok(source.includes("reflow: { hidden, restored }"), "both reflow states must be retained");
  assert.ok(source.includes("const percentageHeights = Object.fromEntries(heightIds.map("),
    "percentage boxes must be independently measured from DOM rectangles");
  assert.ok(source.includes("const borderBoxes = Object.fromEntries(borderIds.map("),
    "border-box observations must come from independent DOM rectangles");
  assert.ok(source.includes("document.elementFromPoint(10, 590)"),
    "offscreen target must be selected by actual viewport DOM hit testing");
  assert.ok(source.includes("window.scrollTo(0, 350)"),
    "source must perform an actual page-level scroll");
  assert.ok(source.includes("const scrollViewport = {"),
    "all scroll phases must be retained in the original source receipt");
  assert.ok(source.includes("document.elementFromPoint(x, y)"),
    "browser input targets must come from the DOM hit test");
  assert.ok(source.includes("const hiddenTargets = hitTargets()"));
  assert.ok(source.includes("const restoredTargets = hitTargets()"));
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

test("definite percentage-height rectangles compare independently from fixed boxes", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  right.percentageHeights["height-first"].height = 47;
  const result = compareNativeBlockBridge(left, right, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "percentage-height", id: "height-first", field: "height",
    left: 46, right: 47, path: "$.percentageHeights.height-first.height"
  }]);
});

test("missing or malformed percentage-height evidence cannot count as equality", () => {
  for (const change of [
    c => { delete c.percentageHeights; },
    c => { delete c.percentageHeights["height-first"]; },
    c => { c.percentageHeights["height-first"].height = NaN; },
    c => { c.percentageHeights["height-host"].width = -1; },
    c => { c.percentageHeights["unexpected"] = { x: 0, y: 0, width: 1, height: 1 }; },
  ]) {
    const right = browser();
    change(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.percentageHeights["height-second"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("border-box widths and heights are an independent exact-parity channel", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  right.borderBoxes["border-first"].width = 55;
  const result = compareNativeBlockBridge(left, right, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "border-box", id: "border-first", field: "width",
    left: 54, right: 55, path: "$.borderBoxes.border-first.width"
  }]);
});

test("missing or malformed border-box records fail before any comparison", () => {
  for (const change of [
    c => { delete c.borderBoxes; },
    c => { delete c.borderBoxes["border-first"]; },
    c => { c.borderBoxes["border-host"].width = -1; },
    c => { c.borderBoxes["border-second"].y = Infinity; },
    c => { c.borderBoxes["unexpected"] = { x: 0, y: 0, width: 1, height: 1 }; },
  ]) {
    const right = browser();
    change(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.borderBoxes["border-second"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("page scroll translates independent viewport hits and restores positions", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.scrollViewport, {
    documentHeight: 958,
    before: scrollSample(0), scrolled: scrollSample(350), restored: scrollSample(0),
  });
  right.scrollViewport.scrolled.targetRect.y = 579;
  const difference = compareNativeBlockBridge(left, right, fixture);
  assert.equal(difference.status, "different");
  assert.deepEqual(difference.differences, [{
    kind: "viewport-scroll", phase: "scrolled", field: "targetRect.y",
    left: 578, right: 579, path: "$.scrollViewport.scrolled.targetRect.y",
  }]);

  const hit = browser();
  hit.scrollViewport.scrolled.hitTarget = null;
  const hitDifference = compareNativeBlockBridge(native(), hit, fixture);
  assert.deepEqual(hitDifference.differences, [{
    kind: "viewport-scroll", phase: "scrolled", field: "hitTarget",
    left: "scroll-target", right: null, path: "$.scrollViewport.scrolled.hitTarget",
  }]);
});

test("missing, partial and impossible scroll receipts fail closed", () => {
  for (const change of [
    c => { delete c.scrollViewport; },
    c => { delete c.scrollViewport.scrolled; },
    c => { c.scrollViewport.documentHeight = NaN; },
    c => { c.scrollViewport.scrolled.scrollY = -1; },
    c => { c.scrollViewport.scrolled.scrollY = 350.5; },
    c => { c.scrollViewport.before.hitTarget = "scroll-spacer"; },
    c => { delete c.scrollViewport.restored.targetRect.width; },
    c => { c.scrollViewport.scrolled.targetRect.height = Infinity; },
    c => { c.scrollViewport.other = {}; },
  ]) {
    const right = browser();
    change(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.scrollViewport.scrolled;
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("hidden siblings reflow and restored boxes are compared independently", () => {
  const right = browser();
  right.reflow.hidden.second.y += 2;
  let report = compareNativeBlockBridge(native(), right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences.map(d => [d.phase, d.id, d.field, d.left, d.right]),
    [["hidden", "second", "y", 12, 14]]);

  const stillVisible = browser();
  stillVisible.reflow.hidden.first = { ...geometry[1] };
  report = compareNativeBlockBridge(native(), stillVisible, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences.map(d => [d.kind, d.phase, d.id]),
    [["visibility", "hidden", "first"]]);

  const didNotRestore = browser();
  didNotRestore.reflow.restored.first.y += 1;
  report = compareNativeBlockBridge(native(), didNotRestore, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences.map(d => [d.phase, d.id, d.field]),
    [["restored", "first", "y"]]);
});

test("omitted or malformed reflow evidence is invalid, never an unchanged box", () => {
  for (const mutate of [
    c => { delete c.reflow; },
    c => { delete c.reflow.hidden; },
    c => { delete c.reflow.hidden.second; },
    c => { c.reflow.hidden.second = undefined; },
    c => { c.reflow.hidden.second.y = Number.NaN; },
    c => { c.reflow.restored.outer.width = -1; },
  ]) {
    const capture = browser();
    mutate(capture);
    assert.equal(compareNativeBlockBridge(native(), capture, fixture).status, "invalid",
      "missing/malformed browser reflow must never compare equal");
  }
  const missingNative = native();
  delete missingNative.reflow;
  assert.equal(compareNativeBlockBridge(missingNative, browser(), fixture).status, "invalid");
});

test("native and browser hit probes match before, during, and after hiding", () => {
  const report = compareNativeBlockBridge(native(), browser(), fixture);
  assert.equal(report.status, "equal", JSON.stringify(report.differences));
  assert.deepEqual(report.browser.hitTargets.original, originalHits);
  assert.deepEqual(report.browser.hitTargets.hidden, hiddenHits);
  assert.deepEqual(report.browser.hitTargets.restored, originalHits);
});

test("a changed browser hit target is a phase-specific difference", () => {
  for (const [phase, probe, oldTarget, newTarget] of [
    ["original", "first-center", "first", "outer"],
    ["hidden", "first-center", "second", "outer"],
    ["hidden", "second-original", null, "outer"],
    ["restored", "first-right", "first", null],
  ]) {
    const right = browser();
    right.hitTargets[phase][probe] = newTarget;
    const report = compareNativeBlockBridge(native(), right, fixture);
    assert.equal(report.status, "different");
    const differences = report.differences.filter(d => d.kind === "hit-target");
    assert.deepEqual(differences.map(d => [d.phase, d.probe, d.left, d.right]),
      [[phase, probe, oldTarget, newTarget]]);
  }
});

test("incomplete, unrecognized or malformed hit observations fail closed", () => {
  for (const change of [
    c => { delete c.hitTargets; },
    c => { delete c.hitTargets.hidden; },
    c => { delete c.hitTargets.original["first-center"]; },
    c => { c.hitTargets.hidden["first-center"] = undefined; },
    c => { c.hitTargets.hidden["first-center"] = "body"; },
    c => { c.hitTargets.restored["first-center"] = 0; },
  ]) {
    const observed = browser();
    change(observed);
    assert.equal(compareNativeBlockBridge(native(), observed, fixture).status, "invalid");
  }
  const reported = native();
  reported.hitTargets.original["first-center"] = "html";
  assert.equal(compareNativeBlockBridge(reported, browser(), fixture).status, "invalid");
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
