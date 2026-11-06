import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { compareNativeFlexBridge } from "./native-flex-bridge.mjs";

const fixture = readFileSync(new URL("./native-flex-bridge.html", import.meta.url));
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
const ids = ["row-host", "row-a", "row-b", "row-inner", "row-c", "row-after"];
const phases = ["nowrap", "wrap", "wide", "columnGap", "rowGap", "hidden",
  "relative", "clipped", "scrolled", "justifyEnd", "justifyCenter", "spaceBetween",
  "justifyAround", "justifyEvenly", "alignEnd", "alignCenter", "wrapAlign", "wrapSingle", "safeOverflow", "overlapClipped",
  "nestedOverlap", "nestedOverflow", "relativeAlign", "clippedAlign", "scrolledAlign",
  "selfAlignMixed", "selfAlignAuto", "selfAlignWrap", "selfAlignOverflow",
  "selfAlignClipped", "selfAlignScrolled", "contentEnd", "contentCenter",
  "contentBetween", "contentAround", "contentEvenly", "contentSingle", "contentNoWrap", "contentOverflow",
  "contentClipped", "contentScrolled", "restored"];
const probes = ["a", "b", "nowrap", "wrap", "gap-wrap", "after", "host-border", "gap", "outside"];
const computedFields = ["display", "boxSizing", "width", "height", "flexDirection",
  "flexWrap", "alignItems", "alignContent", "justifyContent", "columnGap", "rowGap",
  "flexGrow", "flexShrink", "flexBasis", "minWidth"];
const viewport = { width: 800, height: 600, devicePixelRatio: 1 };

// Independent literal rectangles and targets for comparator contracts. These
// synthetic receipts exercise validation only; they are not browser results.
function states() {
  const rows = {
    nowrap: [[20, 20, 162, 54], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 26, 42, 18], [0, 80, 180, 12]],
    wrap: [[20, 20, 162, 80], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [26, 76, 42, 18], [0, 106, 180, 12]],
    wide: [[20, 20, 232, 54], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 26, 42, 18], [0, 80, 180, 12]],
    columnGap: [[20, 20, 162, 86], [26, 26, 64, 24], [31, 60, 56, 36],
      [34, 63, 25, 8], [114, 58, 42, 18], [0, 112, 180, 12]],
    rowGap: [[20, 20, 162, 90], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [26, 86, 42, 18], [0, 116, 180, 12]],
    hidden: [[20, 20, 162, 36], [26, 26, 64, 24], null,
      null, [100, 26, 42, 18], [0, 62, 180, 12]],
    relative: [[20, 20, 162, 80], [26, 26, 64, 24], [113, 37, 56, 36],
      [116, 40, 25, 8], [26, 76, 42, 18], [0, 106, 180, 12]],
    clipped: [[20, 20, 162, 48], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [26, 76, 42, 18], [0, 74, 180, 12]],
    justifyEnd: [[20, 20, 232, 54], [56, 26, 64, 24], [135, 28, 56, 36],
      [138, 31, 25, 8], [204, 26, 42, 18], [0, 80, 180, 12]],
    justifyCenter: [[20, 20, 232, 54], [41, 26, 64, 24], [120, 28, 56, 36],
      [123, 31, 25, 8], [189, 26, 42, 18], [0, 80, 180, 12]],
    spaceBetween: [[20, 20, 242, 54], [26, 26, 64, 24], [125, 28, 56, 36],
      [128, 31, 25, 8], [214, 26, 42, 18], [0, 80, 180, 12]],
    justifyAround: [[20, 20, 262, 54], [36, 26, 64, 24], [135, 28, 56, 36],
      [138, 31, 25, 8], [224, 26, 42, 18], [0, 80, 180, 12]],
    justifyEvenly: [[20, 20, 242, 54], [36, 26, 64, 24], [125, 28, 56, 36],
      [128, 31, 25, 8], [204, 26, 42, 18], [0, 80, 180, 12]],
    alignEnd: [[20, 20, 232, 72], [26, 62, 64, 24], [105, 46, 56, 36],
      [108, 49, 25, 8], [174, 68, 42, 18], [0, 98, 180, 12]],
    alignCenter: [[20, 20, 232, 72], [26, 44, 64, 24], [105, 37, 56, 36],
      [108, 40, 25, 8], [174, 47, 42, 18], [0, 98, 180, 12]],
    wrapAlign: [[20, 20, 162, 80], [26, 35, 64, 24], [117, 28, 56, 36],
      [120, 31, 25, 8], [26, 76, 42, 18], [0, 106, 180, 12]],
    wrapSingle: [[20, 20, 232, 72], [26, 44, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 50, 42, 18], [0, 98, 180, 12]],
    safeOverflow: [[20, 20, 162, 32], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 28, 42, 18], [0, 58, 180, 12]],
    overlapClipped: [[20, 20, 162, 32], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 28, 42, 18], [0, 58, 180, 12]],
    nestedOverlap: [[20, 20, 162, 52], [26, 26, 64, 24], [105, 26, 56, 36],
      [108, 29, 100, 8], [174, 26, 42, 18], [0, 78, 180, 12]],
    nestedOverflow: [[20, 20, 162, 32], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 100, 40], [174, 28, 42, 18], [0, 58, 180, 12]],
    relativeAlign: [[20, 20, 232, 72], [41, 44, 64, 24], [128, 46, 56, 36],
      [131, 49, 25, 8], [189, 47, 42, 18], [0, 98, 180, 12]],
    clippedAlign: [[20, 20, 228, 40], [39, 30, 64, 24], [118, 28, 56, 36],
      [121, 31, 25, 8], [187, 36, 42, 18], [0, 66, 180, 12]],
    selfAlignMixed: [[20, 20, 232, 72], [26, 26, 64, 24], [105, 46, 56, 36],
      [108, 49, 25, 8], [174, 47, 42, 18], [0, 98, 180, 12]],
    selfAlignAuto: [[20, 20, 232, 72], [26, 26, 64, 24], [105, 37, 56, 36],
      [108, 40, 25, 8], [174, 47, 42, 18], [0, 98, 180, 12]],
    selfAlignWrap: [[20, 20, 232, 72], [26, 44, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 26, 42, 18], [0, 98, 180, 12]],
    selfAlignOverflow: [[20, 20, 232, 32], [26, 26, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [174, 27, 42, 18], [0, 58, 180, 12]],
    selfAlignClipped: [[20, 20, 232, 48], [26, 38, 64, 24], [105, 43, 56, 36],
      [108, 46, 25, 8], [174, 35, 42, 18], [0, 74, 180, 12]],
    contentEnd: [[20, 20, 162, 112], [26, 67, 64, 24], [105, 60, 56, 36],
      [108, 63, 25, 8], [26, 108, 42, 18], [0, 138, 180, 12]],
    contentCenter: [[20, 20, 162, 112], [26, 51, 64, 24], [105, 44, 56, 36],
      [108, 47, 25, 8], [26, 92, 42, 18], [0, 138, 180, 12]],
    contentBetween: [[20, 20, 112, 152], [26, 26, 64, 24], [31, 80, 56, 36],
      [34, 83, 25, 8], [26, 148, 42, 18], [0, 178, 180, 12]],
    contentAround: [[20, 20, 112, 172], [26, 36, 64, 24], [31, 90, 56, 36],
      [34, 93, 25, 8], [26, 158, 42, 18], [0, 198, 180, 12]],
    contentEvenly: [[20, 20, 112, 152], [26, 36, 64, 24], [31, 80, 56, 36],
      [34, 83, 25, 8], [26, 138, 42, 18], [0, 178, 180, 12]],
    contentSingle: [[20, 20, 232, 72], [26, 53, 64, 24], [105, 46, 56, 36],
      [108, 49, 25, 8], [174, 56, 42, 18], [0, 98, 180, 12]],
    contentNoWrap: [[20, 20, 232, 72], [26, 44, 64, 24], [105, 37, 56, 36],
      [108, 40, 25, 8], [174, 47, 42, 18], [0, 98, 180, 12]],
    contentOverflow: [[20, 20, 162, 53], [26, 35, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [26, 76, 42, 18], [0, 79, 180, 12]],
    contentClipped: [[20, 20, 162, 112], [26, 35, 64, 24], [105, 28, 56, 36],
      [108, 31, 25, 8], [26, 123, 42, 18], [0, 138, 180, 12]],
  };
  const targets = {
    nowrap: ["row-a", "row-inner", "row-c", "row-after", "row-b", null, "row-host", "row-host", null],
    wrap: ["row-a", "row-inner", "row-host", "row-c", "row-b", "row-after", "row-host", "row-host", null],
    wide: ["row-a", "row-inner", "row-c", "row-after", "row-b", null, "row-host", "row-host", "row-host"],
    columnGap: ["row-a", "row-host", "row-host", "row-host", "row-c", null, "row-host", "row-host", null],
    rowGap: ["row-a", "row-inner", "row-host", "row-host", "row-b", null, "row-host", "row-host", null],
    hidden: ["row-a", "row-c", "row-host", null, "row-after", null, "row-host", "row-host", null],
    relative: ["row-a", "row-host", "row-host", "row-c", "row-b", "row-after", "row-host", "row-host", null],
    clipped: ["row-a", "row-inner", "row-host", "row-after", "row-b", null, "row-host", "row-host", null],
    scrolled: ["row-a", "row-b", "row-host", "row-c", "row-host", null, "row-host", "row-host", null],
    justifyEnd: ["row-host", "row-a", "row-b", "row-after", "row-host", null, "row-host", "row-a", "row-host"],
    justifyCenter: ["row-host", "row-host", "row-host", "row-after", "row-host", null, "row-host", "row-a", "row-host"],
    spaceBetween: ["row-a", "row-host", "row-b", "row-after", "row-host", null, "row-host", "row-host", "row-host"],
    justifyAround: ["row-host", "row-host", "row-b", "row-after", "row-host", null, "row-host", "row-a", "row-host"],
    justifyEvenly: ["row-host", "row-host", "row-b", "row-after", "row-host", null, "row-host", "row-a", "row-host"],
    alignEnd: ["row-host", "row-host", "row-host", "row-a", "row-b", null, "row-host", "row-host", "row-host"],
    alignCenter: ["row-host", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    wrapAlign: ["row-host", "row-host", "row-host", "row-c", "row-b", "row-after", "row-host", "row-host", null],
    wrapSingle: ["row-host", "row-inner", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    safeOverflow: ["row-a", "row-inner", "row-c", null, "row-b", null, "row-host", "row-host", null],
    overlapClipped: ["row-a", "row-inner", "row-host", null, "row-after", null, "row-host", "row-host", null],
    nestedOverlap: ["row-a", "row-inner", "row-c", "row-after", "row-host", null, "row-host", "row-host", null],
    nestedOverflow: ["row-a", "row-inner", "row-c", null, "row-inner", null, "row-host", "row-host", null],
    relativeAlign: ["row-host", "row-host", "row-host", "row-host", "row-host", null, "row-host", "row-host", "row-host"],
    clippedAlign: ["row-host", "row-host", "row-host", null, null, null, "row-host", "row-a", null],
    scrolledAlign: ["row-host", "row-inner", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignMixed: ["row-a", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignAuto: ["row-a", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignWrap: ["row-host", "row-inner", "row-c", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignOverflow: ["row-a", "row-inner", "row-c", null, "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignClipped: ["row-host", "row-host", "row-host", "row-after", "row-b", null, "row-host", "row-host", "row-host"],
    selfAlignScrolled: ["row-a", "row-host", "row-c", null, null, null, "row-host", "row-host", "row-host"],
    contentEnd: ["row-host", "row-host", "row-host", "row-a", "row-b", null, "row-host", "row-host", null],
    contentCenter: ["row-host", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", null],
    contentBetween: ["row-a", "row-host", null, "row-host", "row-host", null, "row-host", "row-host", null],
    contentAround: ["row-host", "row-host", null, "row-host", "row-host", null, "row-host", "row-host", null],
    contentEvenly: ["row-host", "row-host", null, "row-host", "row-host", null, "row-host", "row-host", null],
    contentSingle: ["row-host", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    contentNoWrap: ["row-host", "row-host", "row-host", "row-host", "row-b", null, "row-host", "row-host", "row-host"],
    contentOverflow: ["row-host", "row-inner", "row-host", "row-c", "row-b", null, "row-host", "row-host", null],
    contentClipped: ["row-host", "row-inner", "row-host", "row-host", "row-b", null, "row-host", "row-host", null],
    contentScrolled: ["row-host", "row-host", "row-host", "row-after", "row-host", null, "row-host", "row-host", null],
  };
  rows.scrolled = rows.wrap;
  rows.scrolledAlign = rows.alignCenter;
  rows.selfAlignScrolled = rows.selfAlignClipped;
  rows.contentScrolled = rows.contentClipped;
  rows.restored = rows.nowrap;
  targets.restored = targets.nowrap;
  return Object.fromEntries(phases.map(phase => [phase, {
    scrollY: phase === "contentScrolled" ? 60 :
      (["scrolled", "scrolledAlign", "selfAlignScrolled"].includes(phase) ? 10 : 0),
    boxes: Object.fromEntries(ids.map((id, index) => {
      const value = rows[phase][index];
      return [id, value === null ? null : Object.fromEntries(
        ["x", "y", "width", "height"].map((field, i) => [field, value[i]]))];
    })),
    hits: Object.fromEntries(probes.map((probe, index) => [probe, targets[phase][index]]))
  }]));
}

function receipts(engine = "chromium") {
  const native = {
    schemaVersion: 1, kind: "grunkalunka-native-flex-geometry",
    fixtureId: "native-flex-bridge-v1", viewport: { ...viewport }, states: states()
  };
  const browser = {
    schemaVersion: 1, fixtureId: native.fixtureId,
    environment: {
      userAgent: "synthetic offline comparator test", viewportWidth: 800, viewportHeight: 600,
      devicePixelRatio: 1, url: "file:///synthetic-native-flex-bridge.html",
      timestamp: "2026-11-04T00:00:00.000Z"
    },
    observations: ids.map(id => ({
      id, text: "",
      computed: Object.fromEntries(computedFields.map(field => [field, "synthetic observed style"])),
      rect: { ...native.states.nowrap.boxes[id] }
    })),
    states: states(),
    provenance: {
      collector: "playwright", playwrightVersion: "1.64.1", engine,
      engineFamily: { chromium: "blink", firefox: "gecko", webkit: "webkit" }[engine],
      browserVersion: "synthetic", fixtureSha256: sha(fixture),
      hostPlatform: "synthetic", hostArchitecture: "synthetic", nodeVersion: "v24.21.0",
      viewport: { ...viewport }
    }
  };
  return { native, browser };
}

const compare = pair => compareNativeFlexBridge(pair.native, pair.browser, fixture);

test("complete fixed-row, alignment and overlap receipts agree for each identified reference-engine family", () => {
  for (const engine of ["chromium", "firefox", "webkit"]) {
    const report = compare(receipts(engine));
    assert.equal(report.status, "equal");
    assert.equal(report.conformance, "not-assessed");
    assert.equal(report.sourceSha256, sha(fixture));
    assert.deepEqual([report.invalid, report.incompatible, report.differences], [[], [], []]);
    assert.deepEqual(Object.keys(report.native.states), phases);
    assert.equal(Object.values(report.native.states).flatMap(state => Object.keys(state.boxes)).length, 252);
    assert.equal(Object.values(report.native.states).flatMap(state => Object.keys(state.hits)).length, 378);
  }
});

test("missing, extra and malformed phase data cannot become equality", () => {
  const mutations = [
    ...phases.map(phase => r => { delete r.states[phase]; }),
    r => { r.states.unmeasured = structuredClone(r.states.wrap); },
    r => { r.states.wrap = null; },
    r => { delete r.states.wrap.scrollY; },
    r => { r.states.wrap.scrollY = 10; },
    r => { r.states.scrolled.scrollY = 0; },
    r => { r.states.scrolledAlign.scrollY = 0; },
    r => { r.states.selfAlignScrolled.scrollY = 0; },
    r => { r.states.contentScrolled.scrollY = 10; },
    r => { r.states.alignCenter.scrollY = 10; },
    r => { delete r.states.wrap.boxes["row-c"]; },
    r => { r.states.wrap.boxes["row-contents"] = null; },
    r => { r.states.wrap.boxes["row-c"] = null; },
    r => { delete r.states.wrap.boxes["row-c"].height; },
    r => { r.states.wrap.boxes["row-c"].unknown = 0; },
    r => { r.states.wrap.boxes["row-c"].x = 26.5; },
    r => { r.states.wrap.boxes["row-c"].width = -1; },
    r => { r.states.wrap.boxes["row-c"].height = Infinity; },
    r => { r.states.wrap.boxes["row-c"].y = Number.MAX_SAFE_INTEGER + 1; },
    r => { r.states.alignEnd.boxes["row-b"].y = 46.5; },
    r => { r.states.selfAlignMixed.boxes["row-b"].y = 46.5; },
    r => { r.states.contentCenter.boxes["row-a"].y = 51.5; },
    r => { r.states.nestedOverlap.boxes["row-inner"] = null; },
    r => { delete r.states.wrap.hits.outside; },
    r => { r.states.wrap.hits.extra = null; },
    r => { r.states.wrap.hits.a = "row-contents"; },
    r => { r.states.wrap.hits.a = false; },
    r => { r.states.wrapSingle.hits.b = "row-contents"; },
    r => { r.states.wrap.extra = 0; },
    r => { r.extra = 0; },
  ];
  for (const owner of ["native", "browser"]) {
    for (const mutate of mutations) {
      const pair = receipts();
      mutate(pair[owner]);
      const report = compare(pair);
      assert.equal(report.status, "invalid", owner + ": " + mutate.toString());
      assert.ok(report.invalid.length > 0);
    }
  }
});

test("suppressed flex item and descendant boxes require explicit null and cannot receive hits", () => {
  for (const owner of ["native", "browser"]) {
    for (const id of ["row-b", "row-inner"]) {
      const pair = receipts();
      pair[owner].states.hidden.boxes[id] = { x: 0, y: 0, width: 0, height: 0 };
      assert.equal(compare(pair).status, "invalid");
      const hit = receipts();
      hit[owner].states.hidden.hits.b = id;
      assert.equal(compare(hit).status, "invalid");
    }
  }
});

test("valid geometry and hit differences are exact and independently reported", () => {
  const pair = receipts();
  pair.browser.states.columnGap.boxes["row-c"].x = 122;
  pair.browser.states.columnGap.hits["gap-wrap"] = "row-host";
  const report = compare(pair);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.columnGap.boxes.row-c.x", left: 114, right: 122 },
    { kind: "hit", path: "$.states.columnGap.hits.gap-wrap", left: "row-c", right: "row-host" }
  ]);
  const hitOnly = receipts();
  hitOnly.browser.states.wrap.hits.a = null;
  assert.deepEqual(compare(hitOnly).differences, [
    { kind: "hit", path: "$.states.wrap.hits.a", left: "row-a", right: null }
  ]);
});

test("overlap hits distinguish tree order and incorrect nested item grouping without geometry drift", () => {
  for (const [phase, probe, incorrect, expected] of [
    ["safeOverflow", "gap-wrap", "row-after", "row-b"],
    ["nestedOverflow", "gap-wrap", "row-after", "row-inner"],
    ["nestedOverlap", "nowrap", "row-inner", "row-c"],
    ["overlapClipped", "gap-wrap", "row-b", "row-after"]
  ]) {
    const pair = receipts();
    pair.native.states[phase].hits[probe] = incorrect;
    const report = compare(pair);
    assert.equal(report.status, "different");
    assert.deepEqual(report.invalid, []);
    assert.deepEqual(report.incompatible, []);
    assert.deepEqual(report.differences, [
      { kind: "hit", path: `$.states.${phase}.hits.${probe}`, left: incorrect, right: expected }
    ]);
  }
});

test("an ignored auto reset reports both moved subtree boxes without unrelated differences", () => {
  const pair = receipts();
  pair.native.states.selfAlignAuto.boxes["row-b"].y = 46;
  pair.native.states.selfAlignAuto.boxes["row-inner"].y = 49;
  const report = compare(pair);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.selfAlignAuto.boxes.row-b.y", left: 46, right: 37 },
    { kind: "geometry", path: "$.states.selfAlignAuto.boxes.row-inner.y", left: 49, right: 40 }
  ]);
});



test("space-evenly justification source change reports shifted geometry and inputs", () => {
  const originalRule = '  host.style.justifyContent = "space-evenly";\n  states.justifyEvenly = sample();';
  const changedRule = '  host.style.justifyContent = "flex-start";\n  states.justifyEvenly = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one evenly-justified source declaration");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  const originalX = { "row-a": 26, "row-b": 105, "row-inner": 108, "row-c": 174 };
  for (const [id, x] of Object.entries(originalX)) {
    pair.browser.states.justifyEvenly.boxes[id].x = x;
  }
  for (const [probe, target] of Object.entries({
    a: "row-a", b: "row-inner", nowrap: "row-c", "gap-wrap": "row-b", gap: "row-host"
  })) {
    pair.browser.states.justifyEvenly.hits[probe] = target;
  }
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.justifyEvenly.boxes.row-a.x", left: 36, right: 26 },
    { kind: "geometry", path: "$.states.justifyEvenly.boxes.row-b.x", left: 125, right: 105 },
    { kind: "geometry", path: "$.states.justifyEvenly.boxes.row-inner.x", left: 128, right: 108 },
    { kind: "geometry", path: "$.states.justifyEvenly.boxes.row-c.x", left: 204, right: 174 },
    { kind: "hit", path: "$.states.justifyEvenly.hits.a", left: "row-host", right: "row-a" },
    { kind: "hit", path: "$.states.justifyEvenly.hits.b", left: "row-host", right: "row-inner" },
    { kind: "hit", path: "$.states.justifyEvenly.hits.nowrap", left: "row-b", right: "row-c" },
    { kind: "hit", path: "$.states.justifyEvenly.hits.gap-wrap", left: "row-host", right: "row-b" },
    { kind: "hit", path: "$.states.justifyEvenly.hits.gap", left: "row-a", right: "row-host" }
  ]);
});

test("space-evenly source change distinguishes all four boxes and its exposed first-item hit", () => {
  const originalRule = '  host.style.alignContent = "space-evenly";\n  states.contentEvenly = sample();';
  const changedRule = '  host.style.alignContent = "flex-start";\n  states.contentEvenly = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one evenly-spaced source declaration");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  const shifted = { "row-a": 10, "row-b": 20, "row-inner": 20, "row-c": 30 };
  for (const [id, distance] of Object.entries(shifted)) {
    pair.browser.states.contentEvenly.boxes[id].y -= distance;
  }
  pair.browser.states.contentEvenly.hits.a = "row-a";
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.contentEvenly.boxes.row-a.y", left: 36, right: 26 },
    { kind: "geometry", path: "$.states.contentEvenly.boxes.row-b.y", left: 80, right: 60 },
    { kind: "geometry", path: "$.states.contentEvenly.boxes.row-inner.y", left: 83, right: 63 },
    { kind: "geometry", path: "$.states.contentEvenly.boxes.row-c.y", left: 138, right: 108 },
    { kind: "hit", path: "$.states.contentEvenly.hits.a", left: "row-host", right: "row-a" }
  ]);
});

test("missing between-line distribution reports every displaced line and descendant", () => {
  const pair = receipts();
  pair.native.states.contentBetween.boxes["row-b"].y = 60;
  pair.native.states.contentBetween.boxes["row-inner"].y = 63;
  pair.native.states.contentBetween.boxes["row-c"].y = 108;
  const report = compare(pair);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.contentBetween.boxes.row-b.y", left: 60, right: 80 },
    { kind: "geometry", path: "$.states.contentBetween.boxes.row-inner.y", left: 63, right: 83 },
    { kind: "geometry", path: "$.states.contentBetween.boxes.row-c.y", left: 108, right: 148 }
  ]);
});

test("restoration, scroll and overlap clipping invariants apply to each receipt independently", () => {
  const mutations = [
    r => { r.states.restored.boxes["row-c"].y += 1; },
    r => { r.states.restored.hits.a = "row-host"; },
    r => { r.states.scrolled.boxes["row-c"].y -= 10; },
    r => { r.states.scrolledAlign.boxes["row-b"].y -= 10; },
    r => { r.states.selfAlignScrolled.boxes["row-b"].y -= 10; },
    r => { r.states.contentScrolled.boxes["row-c"].y -= 60; },
    r => { r.states.alignCenter.boxes["row-inner"].x += 1; },
    r => { r.states.overlapClipped.boxes["row-b"].height -= 1; },
  ];
  for (const mutate of mutations) {
    for (const owners of [["native"], ["browser"], ["native", "browser"]]) {
      const pair = receipts();
      for (const owner of owners) mutate(pair[owner]);
      assert.equal(compare(pair).status, "invalid");
    }
  }
});

test("baseline observation metadata and rectangles must be complete and consistent", () => {
  const mutations = [
    r => { r.observations.pop(); },
    r => { r.observations.reverse(); },
    r => { r.observations[0].text = "unexpected layout text"; },
    r => { delete r.observations[0].computed.flexWrap; },
    r => { r.observations[0].computed.flexWrap = ""; },
    r => { r.observations[0].rect.width += 1; },
    r => { r.observations[0].extra = true; },
    r => { delete r.environment.timestamp; },
  ];
  for (const mutate of mutations) {
    const pair = receipts();
    mutate(pair.browser);
    assert.equal(compare(pair).status, "invalid");
  }
});

test("complete matching provenance is required before numerical comparison", () => {
  const malformed = [
    p => { delete p.hostArchitecture; },
    p => { p.browserVersion = ""; },
    p => { p.playwrightVersion = "unrecorded"; },
    p => { p.fixtureSha256 = "not a sha256 digest"; },
    p => { delete p.viewport.devicePixelRatio; },
    p => { p.extra = true; },
  ];
  for (const mutate of malformed) {
    const pair = receipts();
    mutate(pair.browser.provenance);
    assert.equal(compare(pair).status, "invalid");
  }
  const mismatches = [
    p => { p.fixtureSha256 = "0".repeat(64); },
    p => { p.collector = "unknown"; },
    p => { p.engine = "unknown"; },
    p => { p.engineFamily = "gecko"; },
    p => { p.viewport.width = 801; },
  ];
  for (const mutate of mismatches) {
    const pair = receipts();
    mutate(pair.browser.provenance);
    assert.equal(compare(pair).status, "incompatible");
  }
  const pair = receipts();
  pair.browser.environment.viewportHeight = 601;
  assert.equal(compare(pair).status, "incompatible");
  const native = receipts();
  native.native.viewport.width = 801;
  assert.equal(compare(native).status, "invalid");
});

test("gap source mutation requires its own hash while retaining exactly two observable differences", () => {
  const originalRule = 'host.style.columnGap = "24px";';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one reviewed real CSS mutation site");
  const changed = Buffer.from(source.replace(originalRule, 'host.style.columnGap = "32px";'));
  const pair = receipts();
  pair.browser.states.columnGap.boxes["row-c"].x = 122;
  pair.browser.states.columnGap.hits["gap-wrap"] = "row-host";
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.equal(report.differences.length, 2);
  assert.equal(compare(pair).status, "incompatible", "mutated capture must not authenticate as original source");
});

test("justification source mutation authenticates its changed bytes and detects every subtree and hit change", () => {
  const originalRule = '  host.style.justifyContent = "safe flex-end";\n  states.justifyEnd = sample();';
  const changedRule = '  host.style.justifyContent = "flex-start";\n  states.justifyEnd = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one reviewed justifyEnd assignment");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  for (const id of ["row-a", "row-b", "row-inner", "row-c"]) {
    pair.browser.states.justifyEnd.boxes[id].x -= 30;
  }
  Object.assign(pair.browser.states.justifyEnd.hits, {
    a: "row-a", b: "row-inner", nowrap: "row-c", "gap-wrap": "row-b", gap: "row-host"
  });
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.justifyEnd.boxes.row-a.x", left: 56, right: 26 },
    { kind: "geometry", path: "$.states.justifyEnd.boxes.row-b.x", left: 135, right: 105 },
    { kind: "geometry", path: "$.states.justifyEnd.boxes.row-inner.x", left: 138, right: 108 },
    { kind: "geometry", path: "$.states.justifyEnd.boxes.row-c.x", left: 204, right: 174 },
    { kind: "hit", path: "$.states.justifyEnd.hits.a", left: "row-host", right: "row-a" },
    { kind: "hit", path: "$.states.justifyEnd.hits.b", left: "row-a", right: "row-inner" },
    { kind: "hit", path: "$.states.justifyEnd.hits.nowrap", left: "row-b", right: "row-c" },
    { kind: "hit", path: "$.states.justifyEnd.hits.gap-wrap", left: "row-host", right: "row-b" },
    { kind: "hit", path: "$.states.justifyEnd.hits.gap", left: "row-a", right: "row-host" }
  ]);
  assert.equal(compare(pair).status, "incompatible", "mutated capture must not authenticate as original source");
});

test("positioned-after source mutation authenticates one changed overlap hit without reflow", () => {
  const originalRule = '  after.style.position = "static";\n  states.safeOverflow = sample();';
  const changedRule = '  after.style.position = "relative";\n  states.safeOverflow = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one reviewed safeOverflow paint-order assignment");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  pair.browser.states.safeOverflow.hits["gap-wrap"] = "row-after";
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "hit", path: "$.states.safeOverflow.hits.gap-wrap", left: "row-b", right: "row-after" }
  ]);
  assert.equal(compare(pair).status, "incompatible", "mutated capture must not authenticate as original source");
});

test("per-item source mutation authenticates only the changed subtree and hit", () => {
  const originalRule = '  b.style.alignSelf = "safe flex-end";\n  states.selfAlignMixed = sample();';
  const changedRule = '  b.style.alignSelf = "flex-start";\n  states.selfAlignMixed = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one reviewed per-item alignment assignment");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  pair.browser.states.selfAlignMixed.boxes["row-b"].y = 28;
  pair.browser.states.selfAlignMixed.boxes["row-inner"].y = 31;
  pair.browser.states.selfAlignMixed.hits.b = "row-inner";
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.selfAlignMixed.boxes.row-b.y", left: 46, right: 28 },
    { kind: "geometry", path: "$.states.selfAlignMixed.boxes.row-inner.y", left: 49, right: 31 },
    { kind: "hit", path: "$.states.selfAlignMixed.hits.b", left: "row-host", right: "row-inner" }
  ]);
  assert.equal(compare(pair).status, "incompatible", "changed source is not the original source");
});

test("line-packing source mutation authenticates all moved lines and both changed hits", () => {
  const originalRule = '  host.style.alignContent = "safe flex-end";\n  states.contentEnd = sample();';
  const changedRule = '  host.style.alignContent = "flex-start";\n  states.contentEnd = sample();';
  const source = fixture.toString("utf8");
  assert.equal(source.split(originalRule).length, 2, "one reviewed line-packing assignment");
  const changed = Buffer.from(source.replace(originalRule, changedRule));
  const pair = receipts();
  for (const id of ["row-a", "row-b", "row-inner", "row-c"]) {
    pair.browser.states.contentEnd.boxes[id].y -= 32;
  }
  pair.browser.states.contentEnd.hits.b = "row-inner";
  pair.browser.states.contentEnd.hits.wrap = "row-c";
  assert.equal(compareNativeFlexBridge(pair.native, pair.browser, changed).status, "incompatible");
  pair.browser.provenance.fixtureSha256 = sha(changed);
  const report = compareNativeFlexBridge(pair.native, pair.browser, changed);
  assert.equal(report.status, "different");
  assert.deepEqual(report.invalid, []);
  assert.deepEqual(report.incompatible, []);
  assert.deepEqual(report.differences, [
    { kind: "geometry", path: "$.states.contentEnd.boxes.row-a.y", left: 67, right: 35 },
    { kind: "geometry", path: "$.states.contentEnd.boxes.row-b.y", left: 60, right: 28 },
    { kind: "geometry", path: "$.states.contentEnd.boxes.row-inner.y", left: 63, right: 31 },
    { kind: "geometry", path: "$.states.contentEnd.boxes.row-c.y", left: 108, right: 76 },
    { kind: "hit", path: "$.states.contentEnd.hits.b", left: "row-host", right: "row-inner" },
    { kind: "hit", path: "$.states.contentEnd.hits.wrap", left: "row-a", right: "row-c" }
  ]);
  assert.equal(compare(pair).status, "incompatible", "changed source is not the original source");
});

test("invalid JSON values produce diagnostics and fixture bytes are mandatory", () => {
  for (const bad of [null, [], false, 0, "receipt", {}]) {
    assert.equal(compareNativeFlexBridge(bad, receipts().browser, fixture).status, "invalid");
    assert.equal(compareNativeFlexBridge(receipts().native, bad, fixture).status, "invalid");
  }
  const pair = receipts();
  for (const bytes of [undefined, null, "source", [], new Uint8Array()]) {
    assert.throws(() => compareNativeFlexBridge(pair.native, pair.browser, bytes), TypeError);
  }
});

test("CLI retains equality, difference and invalid-input exit codes", () => {
  const directory = mkdtempSync(join(tmpdir(), "grunkalunka-flex-comparator-"));
  try {
    const nativeFile = join(directory, "native.json");
    const browserFile = join(directory, "browser.json");
    const sourceFile = join(directory, "fixture.html");
    const pair = receipts();
    writeFileSync(nativeFile, JSON.stringify(pair.native));
    writeFileSync(sourceFile, fixture);
    const executable = fileURLToPath(new URL("./native-flex-bridge.mjs", import.meta.url));
    for (const [expected, change] of [
      [0, () => {}],
      [1, () => { pair.browser.states.columnGap.boxes["row-c"].x += 1; }],
      [2, () => { delete pair.browser.states.wrap; }]
    ]) {
      change();
      writeFileSync(browserFile, JSON.stringify(pair.browser));
      const child = spawnSync(process.execPath, [executable, nativeFile, browserFile, sourceFile],
        { encoding: "utf8", timeout: 30_000 });
      assert.equal(child.error, undefined);
      assert.equal(child.signal, null, child.stderr);
      assert.equal(child.status, expected, child.stderr);
      assert.equal(JSON.parse(child.stdout).conformance, "not-assessed");
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
