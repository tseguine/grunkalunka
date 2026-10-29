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
const widthBoundGeometry = () => ({
  "bounds-host": { x: 0, y: 278, width: 180, height: 60 },
  "bounds-first": { x: 6, y: 284, width: 84, height: 10 },
  "bounds-second": { x: 6, y: 294, width: 100, height: 8 },
});
const heightBoundGeometry = () => ({
  "height-limited": { x: 0, y: 338, width: 80, height: 60 },
  "height-limited-first": { x: 6, y: 344, width: 30, height: 24 },
  "height-limited-second": { x: 6, y: 368, width: 20, height: 12 },
});
const visibilityGeometry = () => ({
  "visibility-host": { x: 300, y: 358, width: 40, height: 25 },
  "visibility-child": { x: 300, y: 358, width: 14, height: 6 },
  "visibility-inherited": { x: 300, y: 364, width: 12, height: 5 },
  "visibility-sibling": { x: 300, y: 383, width: 20, height: 5 },
});
const visibilityPhase = shown => ({
  boxes: visibilityGeometry(),
  computed: {
    "visibility-host": shown,
    "visibility-child": true,
    "visibility-inherited": shown,
    "visibility-sibling": true,
  },
  hits: {
    child: "visibility-child",
    host: shown ? "visibility-host" : null,
    inherited: shown ? "visibility-inherited" : null,
    sibling: "visibility-sibling",
  },
});
const visibilityStates = () => ({
  hidden: visibilityPhase(false),
  visible: visibilityPhase(true),
  restored: visibilityPhase(false),
});
const descendantClipBoxes = () => ({
  "clip-parent": { x: 100, y: 388, width: 30, height: 22 },
  "clip-child": { x: 105, y: 393, width: 40, height: 18 },
  "clip-sibling": { x: 100, y: 410, width: 10, height: 5 },
});
const descendantClipPhase = clipped => ({
  mode: clipped ? "clip" : "visible",
  boxes: descendantClipBoxes(),
  hits: {
    inside: "clip-child",
    "padding-edge": "clip-child",
    "border-edge": clipped ? "clip-parent" : "clip-child",
    "outside-parent": clipped ? null : "clip-child",
    "bottom-border": clipped ? "clip-parent" : "clip-child",
    sibling: "clip-sibling",
    "parent-border": "clip-parent",
  },
});
const descendantClipStates = () => ({
  clipped: descendantClipPhase(true),
  visible: descendantClipPhase(false),
  restored: descendantClipPhase(true),
});
const boxlessPhase = mode => {
  const contents = mode === "contents";
  return {
    display: mode, hasBox: !contents,
    boxes: contents ? {
      "contents-first": { x: 0, y: 415, width: 24, height: 10 },
      "contents-second": { x: 0, y: 425, width: 12, height: 6 },
      "contents-sibling": { x: 200, y: 431, width: 20, height: 5 },
    } : {
      "contents-first": { x: 205, y: 422, width: 24, height: 10 },
      "contents-second": { x: 205, y: 432, width: 12, height: 6 },
      "contents-sibling": { x: 200, y: 451, width: 20, height: 5 },
    },
    hits: contents ? {
      "first-contents": "contents-first",
      "second-contents": "contents-second",
      "wrapper-border": null,
      "first-block": null,
      "sibling-contents": "contents-sibling",
      "sibling-block": null,
      outside: null,
    } : {
      "first-contents": null,
      "second-contents": null,
      "wrapper-border": "contents-wrapper",
      "first-block": "contents-first",
      "sibling-contents": "contents-wrapper",
      "sibling-block": "contents-sibling",
      outside: null,
    },
  };
};
const boxlessStates = () => ({
  contents: boxlessPhase("contents"),
  block: boxlessPhase("block"),
  restored: boxlessPhase("contents"),
});
const pointerGeometry = () => ({
  "pointer-host": { x: 350, y: 436, width: 30, height: 26 },
  "pointer-inherited": { x: 350, y: 436, width: 10, height: 6 },
  "pointer-restored": { x: 350, y: 442, width: 14, height: 7 },
  "pointer-sibling": { x: 350, y: 462, width: 20, height: 5 },
});
const pointerPhase = auto => ({
  boxes: pointerGeometry(),
  computed: {
    "pointer-host": auto,
    "pointer-inherited": auto,
    "pointer-restored": true,
    "pointer-sibling": true,
  },
  hits: {
    inherited: auto ? "pointer-inherited" : null,
    host: auto ? "pointer-host" : null,
    restored: "pointer-restored",
    "host-lower": auto ? "pointer-host" : null,
    sibling: "pointer-sibling",
    outside: null,
  },
});
const pointerStates = () => ({
  none: pointerPhase(false),
  auto: pointerPhase(true),
  restored: pointerPhase(false),
});
const autoMarginGeometry = shifted => ({
  "auto-host": { x: 0, y: 467, width: 200, height: 64 },
  "auto-centered": { x: shifted ? 20 : 80, y: 467, width: 40, height: 8 },
  "auto-left": { x: 140, y: 475, width: 50, height: 7 },
  "auto-right": { x: 12, y: 482, width: 30, height: 6 },
  "auto-limited": { x: 60, y: 488, width: 80, height: 6 },
  "auto-sibling": { x: 0, y: 531, width: 20, height: 5 },
});
const autoMarginPhase = shifted => ({
  boxes: autoMarginGeometry(shifted),
  computed: {
    "auto-centered": {
      left: shifted ? "20px" : "80px", right: shifted ? "140px" : "80px",
    },
    "auto-left": { left: "140px", right: "10px" },
    "auto-right": { left: "12px", right: "158px" },
    "auto-limited": { left: "60px", right: "60px" },
  },
  hits: {
    "center-original": shifted ? "auto-host" : "auto-centered",
    "center-shifted": shifted ? "auto-centered" : "auto-host",
    "left-auto": "auto-left", "right-auto": "auto-right",
    limited: "auto-limited", host: "auto-host", sibling: "auto-sibling", outside: null,
  },
});
const autoMarginStates = () => ({
  centered: autoMarginPhase(false),
  leftFixed: autoMarginPhase(true),
  restored: autoMarginPhase(false),
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
  widthBounds: widthBoundGeometry(),
  heightBounds: heightBoundGeometry(),
  visibilityStates: visibilityStates(),
  descendantClipStates: descendantClipStates(),
  boxlessStates: boxlessStates(),
  pointerStates: pointerStates(),
  autoMarginStates: autoMarginStates(),
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
    widthBounds: widthBoundGeometry(),
    heightBounds: heightBoundGeometry(),
    visibilityStates: visibilityStates(),
    descendantClipStates: descendantClipStates(),
    boxlessStates: boxlessStates(),
    pointerStates: pointerStates(),
  autoMarginStates: autoMarginStates(),
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
    "#bounds-containing { width: 400px; height: 80px; }",
    "#bounds-host { width: 400px; min-width: 45%; max-width: 30%;",
    "#bounds-first { width: 50%; height: 10px; }",
    "#bounds-second { width: 250px; max-width: 100px; height: 8px; }",
    "#height-limited { width: 80px; height: 90px; min-height: 75%; max-height: 50%;",
    "#height-limited-first { width: 30px; height: 50%; }",
    "#height-limited-second { width: 20px; height: 12px; }",
    "#visibility-host { width: 40px; height: 25px; margin-left: 300px; visibility: hidden; }",
    "#visibility-child { width: 14px; height: 6px; visibility: visible; }",
    "#visibility-inherited { width: 12px; height: 5px; }",
    "#visibility-sibling { width: 20px; height: 5px; margin-left: 300px; }",
    "#clip-parent { width: 20px; height: 12px; margin-left: 100px;",
    "#clip-child { width: 40px; height: 18px; }",
    "#clip-sibling { width: 10px; height: 5px; margin-left: 100px; }",
    "#contents-wrapper { display: contents; width: 40px; height: 24px; margin: 2px 0 0 200px;",
    "#contents-first { width: 24px; height: 10px; }",
    "#contents-second { width: 12px; height: 6px; }",
    "#contents-sibling { width: 20px; height: 5px; margin-left: 200px; }",
    "#pointer-host { width: 30px; height: 26px; margin-left: 350px; pointer-events: none; }",
    "#pointer-inherited { width: 10px; height: 6px; }",
    "#pointer-restored { width: 14px; height: 7px; pointer-events: auto; }",
    "#pointer-sibling { width: 20px; height: 5px; margin-left: 350px; }",
    "#auto-host { width: 200px; height: 64px; }",
    "#auto-centered { width: 40px; height: 8px; margin: 0 auto; }",
    "#auto-left { width: 50px; height: 7px; margin-left: auto; margin-right: 10px; }",
    "#auto-right { width: 30px; height: 6px; margin-left: 12px; margin-right: auto; }",
    "#auto-limited { width: auto; max-width: 80px; height: 6px; margin: 0 auto; }",
    "#auto-sibling { width: 20px; height: 5px; }",
    "#scroll-target { width: 80px; height: 30px; }",
    '<div id="outer"><div id="first"></div><div id="second"></div></div>',
    '<div id="height-host"><div id="height-first"></div><div id="height-second"></div></div>',
    '<div id="border-host"><div id="border-first"></div><div id="border-second"></div></div>',
    '<div id="scroll-spacer"><div id="bounds-containing"><div id="bounds-host"><div id="bounds-first"></div><div id="bounds-second"></div></div><div id="height-limited"><div id="height-limited-first"></div><div id="height-limited-second"></div></div></div><div id="visibility-host"><div id="visibility-child"></div><div id="visibility-inherited"></div></div><div id="visibility-sibling"></div><div id="clip-parent"><div id="clip-child"></div></div><div id="clip-sibling"></div><div id="contents-wrapper"><div id="contents-first"></div><div id="contents-second"></div></div><div id="contents-sibling"></div><div id="pointer-host"><div id="pointer-inherited"></div><div id="pointer-restored"></div></div><div id="pointer-sibling"></div><div id="auto-host"><div id="auto-centered"></div><div id="auto-left"></div><div id="auto-right"></div><div id="auto-limited"></div></div><div id="auto-sibling"></div></div><div id="scroll-target"></div>',
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
  assert.ok(source.includes('autoCenter.style.marginLeft = "20px"'),
    "actual CSS margin mutation required");
  assert.ok(source.includes('autoCenter.style.removeProperty("margin-left")'),
    "CSS margin restoration required");
  assert.ok(source.includes("const autoMarginStates = {"),
    "all three observed auto-margin phases required");
  assert.ok(source.includes("const style = getComputedStyle(document.getElementById(id))"),
    "used margins must come from browser style");
  assert.ok(source.includes("document.elementFromPoint(10, 590)"),
    "offscreen target must be selected by actual viewport DOM hit testing");
  assert.ok(source.includes("window.scrollTo(0, 350)"),
    "source must perform an actual page-level scroll");
  assert.ok(source.includes("const heightBounds = Object.fromEntries(limitedIds.map("),
    "height limits must be measured from fresh DOM rectangles");
  assert.ok(source.includes("const widthBounds = Object.fromEntries(boundsIds.map("),
    "width constraints must be measured from fresh DOM rectangles");
  assert.ok(source.includes('visibilityHost.style.visibility = "visible"'),
    "source must toggle actual visibility, not manually edit receipt data");
  assert.ok(source.includes('visibilityHost.style.removeProperty("visibility")'),
    "source must restore the original hidden declaration");
  assert.ok(source.includes("document.elementFromPoint(x, y)"),
    "actual DOM elementFromPoint must supply all visibility hits");
  assert.ok(source.includes('clipParent.style.overflow = "visible"'),
    "source must toggle real overflow clipping");
  assert.ok(source.includes('clipParent.style.removeProperty("overflow")'),
    "source must restore original clipping");
  assert.ok(source.includes("getComputedStyle(document.getElementById(\"clip-parent\")).overflowX"),
    "reference clip mode must come from computed CSS");
  assert.ok(source.includes('contentsWrapper.style.display = "block"'),
    "source must give the boxless wrapper a real principal box");
  assert.ok(source.includes('contentsWrapper.style.removeProperty("display")'),
    "source must restore the original boxless declaration");
  assert.ok(source.includes('wrapper.getClientRects().length !== 0'),
    "principal box presence must come from real DOM measurement");
  assert.ok(source.includes('pointerHost.style.pointerEvents = "auto"'),
    "source must toggle actual inherited pointer-events");
  assert.ok(source.includes('pointerHost.style.removeProperty("pointer-events")'),
    "source must restore pointer-events:none");
  assert.ok(source.includes('getComputedStyle(document.getElementById(id)).pointerEvents === "auto"'),
    "reference pointer eligibility must come from computed DOM styles");
  assert.ok(source.includes("const pointerStates = {"),
    "source must preserve all three independent pointer phases");
  assert.ok(source.includes("const boxlessStates = {"),
    "all three boxless phases must be included in source-hashed evidence");
  assert.ok(source.includes("const descendantClipStates = {"),
    "clip snapshots must be preserved in source-hashed evidence");
  assert.ok(source.includes("const visibilityStates = {"),
    "all three visibility phases must be retained");
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

test("min/max widths compare independent real box rectangles and nested widths", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.widthBounds, {
    "bounds-host": { x: 0, y: 278, width: 180, height: 60 },
    "bounds-first": { x: 6, y: 284, width: 84, height: 10 },
    "bounds-second": { x: 6, y: 294, width: 100, height: 8 },
  });
  right.widthBounds["bounds-first"].width = 85;
  const report = compareNativeBlockBridge(left, right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences, [{
    kind: "width-bound", id: "bounds-first", field: "width",
    left: 84, right: 85, path: "$.widthBounds.bounds-first.width",
  }]);
});

test("missing or malformed constrained-width evidence fails closed", () => {
  for (const change of [
    c => { delete c.widthBounds; },
    c => { delete c.widthBounds["bounds-first"]; },
    c => { c.widthBounds["bounds-host"].width = -1; },
    c => { c.widthBounds["bounds-second"].y = NaN; },
    c => { delete c.widthBounds["bounds-second"].height; },
    c => { c.widthBounds.unexpected = { x: 0, y: 0, width: 0, height: 0 }; },
  ]) {
    const right = browser();
    change(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.widthBounds["bounds-first"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("min/max heights compare real border boxes and dependent percentage children", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.heightBounds, {
    "height-limited": { x: 0, y: 338, width: 80, height: 60 },
    "height-limited-first": { x: 6, y: 344, width: 30, height: 24 },
    "height-limited-second": { x: 6, y: 368, width: 20, height: 12 },
  });
  right.heightBounds["height-limited-first"].height = 25;
  const result = compareNativeBlockBridge(left, right, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "height-bound", id: "height-limited-first", field: "height",
    left: 24, right: 25, path: "$.heightBounds.height-limited-first.height",
  }]);
});

test("incomplete or malformed constrained-height evidence cannot pass", () => {
  for (const change of [
    c => { delete c.heightBounds; },
    c => { delete c.heightBounds["height-limited"]; },
    c => { c.heightBounds["height-limited-first"].height = -1; },
    c => { c.heightBounds["height-limited-second"].y = NaN; },
    c => { delete c.heightBounds["height-limited"].width; },
    c => { c.heightBounds.unexpected = {}; },
  ]) {
    const right = browser();
    change(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.heightBounds["height-limited-first"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("visibility inheritance agrees and a hidden parent cannot intercept hits", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.visibilityStates.hidden.hits, {
    child: "visibility-child",
    host: null,
    inherited: null,
    sibling: "visibility-sibling",
  });
  assert.deepEqual(right.visibilityStates.visible.hits, {
    child: "visibility-child",
    host: "visibility-host",
    inherited: "visibility-inherited",
    sibling: "visibility-sibling",
  });
  assert.deepEqual(right.visibilityStates.restored, right.visibilityStates.hidden);
  right.visibilityStates.hidden.hits.host = "visibility-host";
  const altered = compareNativeBlockBridge(left, right, fixture);
  assert.equal(altered.status, "different");
  assert.deepEqual(altered.differences, [{
    kind: "visibility-hit", phase: "hidden", probe: "host",
    left: null, right: "visibility-host",
    path: "$.visibilityStates.hidden.hits.host",
  }]);
});

test("visibility rectangle and inherited computed state changes are separate differences", () => {
  const observed = browser();
  observed.visibilityStates.visible.boxes["visibility-child"].y++;
  observed.visibilityStates.hidden.computed["visibility-child"] = false;
  const result = compareNativeBlockBridge(native(), observed, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "visibility-computed", phase: "hidden", id: "visibility-child",
    left: true, right: false,
    path: "$.visibilityStates.hidden.computed.visibility-child",
  }, {
    kind: "visibility-box", phase: "visible", id: "visibility-child", field: "y",
    left: 358, right: 359,
    path: "$.visibilityStates.visible.boxes.visibility-child.y",
  }]);
});

test("missing, malformed and surplus visibility evidence fails closed", () => {
  for (const mutate of [
    r => { delete r.visibilityStates; },
    r => { delete r.visibilityStates.visible; },
    r => { delete r.visibilityStates.hidden.computed; },
    r => { delete r.visibilityStates.visible.hits.host; },
    r => { r.visibilityStates.visible.hits.inherited = "body"; },
    r => { r.visibilityStates.hidden.computed["visibility-inherited"] = "hidden"; },
    r => { r.visibilityStates.hidden.boxes["visibility-host"].width = -1; },
    r => { r.visibilityStates.restored.boxes["visibility-child"].y = Infinity; },
    r => { delete r.visibilityStates.hidden.boxes["visibility-sibling"]; },
    r => { r.visibilityStates.extra = {}; },
  ]) {
    const observed = browser();
    mutate(observed);
    assert.equal(compareNativeBlockBridge(native(), observed, fixture).status, "invalid");
  }
  const left = native();
  delete left.visibilityStates.hidden.hits.host;
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("real descendant padding clips distinguish own border and overflowing child hits", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.descendantClipStates.clipped.hits, {
    inside: "clip-child",
    "padding-edge": "clip-child",
    "border-edge": "clip-parent",
    "outside-parent": null,
    "bottom-border": "clip-parent",
    sibling: "clip-sibling",
    "parent-border": "clip-parent",
  });
  assert.deepEqual(right.descendantClipStates.visible.hits, {
    inside: "clip-child",
    "padding-edge": "clip-child",
    "border-edge": "clip-child",
    "outside-parent": "clip-child",
    "bottom-border": "clip-child",
    sibling: "clip-sibling",
    "parent-border": "clip-parent",
  });
  assert.deepEqual(right.descendantClipStates.restored, right.descendantClipStates.clipped);
  right.descendantClipStates.clipped.hits["outside-parent"] = "clip-child";
  const result = compareNativeBlockBridge(left, right, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "descendant-clip-hit", phase: "clipped", probe: "outside-parent",
    left: null, right: "clip-child",
    path: "$.descendantClipStates.clipped.hits.outside-parent",
  }]);
});

test("clip mode and geometry differences are independently reported", () => {
  const right = browser();
  right.descendantClipStates.clipped.mode = "visible";
  right.descendantClipStates.visible.boxes["clip-child"].width = 41;
  const report = compareNativeBlockBridge(native(), right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences, [{
    kind: "descendant-clip-mode", phase: "clipped",
    left: "clip", right: "visible",
    path: "$.descendantClipStates.clipped.mode",
  }, {
    kind: "descendant-clip-box", phase: "visible", id: "clip-child",
    field: "width", left: 40, right: 41,
    path: "$.descendantClipStates.visible.boxes.clip-child.width",
  }]);
});

test("missing, partial and malformed descendant clips never count as equality", () => {
  for (const mutate of [
    c => { delete c.descendantClipStates; },
    c => { delete c.descendantClipStates.restored; },
    c => { delete c.descendantClipStates.clipped.boxes; },
    c => { delete c.descendantClipStates.visible.hits["parent-border"]; },
    c => { c.descendantClipStates.visible.hits.inside = "body"; },
    c => { c.descendantClipStates.clipped.mode = "hidden"; },
    c => { c.descendantClipStates.visible.boxes["clip-child"].height = NaN; },
    c => { c.descendantClipStates.clipped.boxes["clip-parent"].width = -1; },
    c => { c.descendantClipStates.clipped.extra = 0; },
    c => { c.descendantClipStates.clipped.hits.unknown = null; },
  ]) {
    const right = browser();
    mutate(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.descendantClipStates.visible.hits["padding-edge"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("boxless wrappers flatten children, then restore normal block box geometry", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.boxlessStates.contents, right.boxlessStates.restored);
  assert.equal(right.boxlessStates.contents.hasBox, false);
  assert.equal(right.boxlessStates.block.hasBox, true);
  assert.deepEqual(right.boxlessStates.contents.hits, {
    "first-contents": "contents-first",
    "second-contents": "contents-second",
    "wrapper-border": null,
    "first-block": null,
    "sibling-contents": "contents-sibling",
    "sibling-block": null,
    outside: null,
  });
  assert.deepEqual(right.boxlessStates.block.hits, {
    "first-contents": null,
    "second-contents": null,
    "wrapper-border": "contents-wrapper",
    "first-block": "contents-first",
    "sibling-contents": "contents-wrapper",
    "sibling-block": "contents-sibling",
    outside: null,
  });
  right.boxlessStates.contents.hits["wrapper-border"] = "contents-wrapper";
  const report = compareNativeBlockBridge(left, right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences, [{
    kind: "boxless-hit", phase: "contents", probe: "wrapper-border",
    left: null, right: "contents-wrapper",
    path: "$.boxlessStates.contents.hits.wrapper-border",
  }]);
});

test("boxless display, principal box and sibling geometry are independent comparisons", () => {
  const right = browser();
  right.boxlessStates.contents.hasBox = true;
  right.boxlessStates.block.display = "contents";
  right.boxlessStates.block.boxes["contents-sibling"].y++;
  const report = compareNativeBlockBridge(native(), right, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences, [{
    kind: "boxless-mode", phase: "contents", field: "hasBox",
    left: false, right: true, path: "$.boxlessStates.contents.hasBox",
  }, {
    kind: "boxless-mode", phase: "block", field: "display",
    left: "block", right: "contents", path: "$.boxlessStates.block.display",
  }, {
    kind: "boxless-box", phase: "block", id: "contents-sibling", field: "y",
    left: 451, right: 452, path: "$.boxlessStates.block.boxes.contents-sibling.y",
  }]);
});

test("invalid and incomplete boxless receipts fail closed", () => {
  for (const mutate of [
    r => { delete r.boxlessStates; },
    r => { delete r.boxlessStates.restored; },
    r => { delete r.boxlessStates.contents.boxes; },
    r => { delete r.boxlessStates.block.boxes["contents-first"]; },
    r => { r.boxlessStates.block.display = "none"; },
    r => { r.boxlessStates.restored.hasBox = 0; },
    r => { r.boxlessStates.contents.hits.outside = "body"; },
    r => { delete r.boxlessStates.contents.hits["first-contents"]; },
    r => { r.boxlessStates.contents.boxes["contents-second"].y = Infinity; },
    r => { r.boxlessStates.block.boxes["contents-sibling"].width = -1; },
    r => { r.boxlessStates.unknown = {}; },
  ]) {
    const right = browser();
    mutate(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.boxlessStates.block.hits["first-block"];
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});

test("real pointer-events inheritance preserves boxes and restores eligible descendants", () => {
  const left = native(), right = browser();
  assert.equal(compareNativeBlockBridge(left, right, fixture).status, "equal");
  assert.deepEqual(right.pointerStates.none.hits, {
    inherited: null, host: null, restored: "pointer-restored",
    "host-lower": null, sibling: "pointer-sibling", outside: null,
  });
  assert.deepEqual(right.pointerStates.auto.hits, {
    inherited: "pointer-inherited", host: "pointer-host",
    restored: "pointer-restored", "host-lower": "pointer-host",
    sibling: "pointer-sibling", outside: null,
  });
  assert.deepEqual(right.pointerStates.restored, right.pointerStates.none);
  right.pointerStates.none.hits.inherited = "pointer-inherited";
  const mismatch = compareNativeBlockBridge(left, right, fixture);
  assert.equal(mismatch.status, "different");
  assert.deepEqual(mismatch.differences, [{
    kind: "pointer-hit", phase: "none", probe: "inherited",
    left: null, right: "pointer-inherited",
    path: "$.pointerStates.none.hits.inherited",
  }]);
});

test("computed pointer policy and fixed geometry differences are independently visible", () => {
  const observed = browser();
  observed.pointerStates.auto.computed["pointer-inherited"] = false;
  observed.pointerStates.restored.boxes["pointer-sibling"].y += 1;
  const report = compareNativeBlockBridge(native(), observed, fixture);
  assert.equal(report.status, "different");
  assert.deepEqual(report.differences, [{
    kind: "pointer-computed", phase: "auto", id: "pointer-inherited",
    left: true, right: false,
    path: "$.pointerStates.auto.computed.pointer-inherited",
  }, {
    kind: "pointer-box", phase: "restored", id: "pointer-sibling",
    field: "y", left: 462, right: 463,
    path: "$.pointerStates.restored.boxes.pointer-sibling.y",
  }]);
});

test("incomplete or impossible pointer observations fail closed", () => {
  for (const mutate of [
    r => { delete r.pointerStates; },
    r => { delete r.pointerStates.restored; },
    r => { delete r.pointerStates.auto.computed; },
    r => { delete r.pointerStates.none.boxes["pointer-inherited"]; },
    r => { r.pointerStates.auto.computed["pointer-host"] = "auto"; },
    r => { r.pointerStates.none.boxes["pointer-restored"].width = -1; },
    r => { r.pointerStates.restored.boxes["pointer-sibling"].y = Infinity; },
    r => { r.pointerStates.auto.hits.host = "body"; },
    r => { delete r.pointerStates.auto.hits["host-lower"]; },
    r => { r.pointerStates.auto.other = {}; },
  ]) {
    const right = browser();
    mutate(right);
    assert.equal(compareNativeBlockBridge(native(), right, fixture).status, "invalid");
  }
  const left = native();
  delete left.pointerStates.none.hits.inherited;
  assert.equal(compareNativeBlockBridge(left, browser(), fixture).status, "invalid");
});


test("source-observed auto margins center, move and restore boxes and input", () => {
  const result = compareNativeBlockBridge(native(), browser(), fixture);
  assert.equal(result.status, "equal", JSON.stringify(result.differences));
  const right = browser();
  assert.deepEqual(right.autoMarginStates.centered, right.autoMarginStates.restored);
  assert.deepEqual(right.autoMarginStates.centered.hits, {
    "center-original": "auto-centered", "center-shifted": "auto-host",
    "left-auto": "auto-left", "right-auto": "auto-right",
    limited: "auto-limited", host: "auto-host",
    sibling: "auto-sibling", outside: null,
  });
  assert.deepEqual(right.autoMarginStates.leftFixed.hits, {
    "center-original": "auto-host", "center-shifted": "auto-centered",
    "left-auto": "auto-left", "right-auto": "auto-right",
    limited: "auto-limited", host: "auto-host",
    sibling: "auto-sibling", outside: null,
  });
  assert.deepEqual(right.autoMarginStates.leftFixed.computed["auto-centered"],
    { left: "20px", right: "140px" });
});

test("auto margin geometry, used CSS margins and targets report distinct mismatches", () => {
  const observed = browser();
  observed.autoMarginStates.leftFixed.boxes["auto-centered"].x += 1;
  observed.autoMarginStates.centered.computed["auto-limited"].right = "61px";
  observed.autoMarginStates.restored.hits["center-original"] = "auto-host";
  const result = compareNativeBlockBridge(native(), observed, fixture);
  assert.equal(result.status, "different");
  assert.deepEqual(result.differences, [{
    kind: "auto-margin-computed", phase: "centered", id: "auto-limited", side: "right",
    left: "60px", right: "61px",
    path: "$.autoMarginStates.centered.computed.auto-limited.right",
  }, {
    kind: "auto-margin-box", phase: "leftFixed", id: "auto-centered", field: "x",
    left: 20, right: 21,
    path: "$.autoMarginStates.leftFixed.boxes.auto-centered.x",
  }, {
    kind: "auto-margin-hit", phase: "restored", probe: "center-original",
    left: "auto-centered", right: "auto-host",
    path: "$.autoMarginStates.restored.hits.center-original",
  }]);
});

test("absent, partial, surplus or impossible auto-margin evidence fails closed", () => {
  for (const change of [
    r => { delete r.autoMarginStates; },
    r => { delete r.autoMarginStates.leftFixed; },
    r => { delete r.autoMarginStates.centered.computed; },
    r => { delete r.autoMarginStates.restored.boxes["auto-left"]; },
    r => { r.autoMarginStates.centered.boxes["auto-centered"].width = -1; },
    r => { r.autoMarginStates.centered.boxes["auto-host"].x = Infinity; },
    r => { r.autoMarginStates.leftFixed.computed["auto-centered"].left = "auto"; },
    r => { r.autoMarginStates.leftFixed.computed["auto-centered"].right = "-1px"; },
    r => { delete r.autoMarginStates.restored.hits["center-original"]; },
    r => { r.autoMarginStates.centered.hits["center-original"] = "body"; },
    r => { r.autoMarginStates.leftFixed.extra = {}; },
    r => { r.autoMarginStates.unknown = {}; },
  ]) {
    const observed = browser();
    change(observed);
    assert.equal(compareNativeBlockBridge(native(), observed, fixture).status, "invalid");
  }
  const reported = native();
  delete reported.autoMarginStates.centered.computed["auto-centered"].right;
  assert.equal(compareNativeBlockBridge(reported, browser(), fixture).status, "invalid");
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
