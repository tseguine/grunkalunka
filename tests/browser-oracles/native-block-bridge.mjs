// Selected native block-border geometry versus a real browser receipt.
// This is not the generic browser-vs-browser comparator or a CSS conformance test.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateCapture } from "./compare.mjs";

const ids = ["outer", "first", "second"];
const heightIds = ["height-host", "height-first", "height-second"];
const borderIds = ["border-host", "border-first", "border-second"];
const widthBoundIds = ["bounds-host", "bounds-first", "bounds-second"];
const heightBoundIds = ["height-limited", "height-limited-first", "height-limited-second"];
const scrollPhases = ["before", "scrolled", "restored"];
const probes = ["first-center", "first-lower", "second-original", "left-margin",
  "outer-border", "outside", "first-right"];
const rectFields = ["x", "y", "width", "height"];
const computedFields = ["display", "boxSizing", "width", "height", "marginTop",
  "paddingLeft", "borderLeftWidth"];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const hash = bytes => createHash("sha256").update(bytes).digest("hex");

export function compareNativeBlockBridge(native, browser, sourceBytes) {
  if (!(sourceBytes instanceof Uint8Array)) throw new TypeError("fixture bytes required");
  const sourceSha256 = hash(sourceBytes);
  const invalid = [];
  const incompatible = [];
  const differences = [];
  const error = (path, message) => invalid.push({ path, message });
  const mismatch = (path, left, right) => incompatible.push({ path, left, right });
  const report = {
    schemaVersion: 1, kind: "grunkalunka-native-browser-geometry",
    status: "invalid", conformance: "not-assessed", sourceSha256,
    comparisonScope: "selected block, percentage-height, border-box, constrained-width/height and page-scroll rectangle/hit observations; no HTML/CSS parsing, inline layout or paint",
    invalid, incompatible, differences, native, browser,
  };
  if (!record(native) || native.schemaVersion !== 1 ||
      native.kind !== "grunkalunka-native-block-geometry" ||
      native.fixtureId !== "native-block-bridge-v1") {
    error("$.native", "expected a native block bridge v1 receipt");
  } else {
    if (!record(native.viewport) ||
        native.viewport.width !== 800 || native.viewport.height !== 600 ||
        native.viewport.devicePixelRatio !== 1) {
      error("$.native.viewport", "expected the fixed 800x600 DPR1 bridge viewport");
    }
    if (!record(native.geometry) ||
        Object.keys(native.geometry).sort().join(",") !== [...ids].sort().join(",")) {
      error("$.native.geometry", "exactly the three named native rectangles required");
    } else {
      for (const id of ids) {
        const rect = native.geometry[id];
        for (const field of rectFields) {
          if (!record(rect) || !Number.isFinite(rect[field]) ||
              (["width", "height"].includes(field) && rect[field] < 0)) {
            error("$.native.geometry." + id + "." + field, "finite rectangle coordinate required");
          }
        }
      }
    }
  }
  for (const item of validateCapture(browser)) error("$.browser" + item.path.slice(1), item.message);

  // The collector preserves extra fixture observations without rewriting
  // them. Both the native and browser records must independently include all
  // reflow states: missing data must never pass as an unchanged rectangle.
  const phases = ["hidden", "restored"];
  const sortedIds = [...ids].sort().join(",");
  function validateReflow(name, value) {
    if (!record(value) || Object.keys(value).sort().join(",") !== phases.join(",")) {
      error("$." + name + ".reflow", "exactly hidden and restored phases required");
      return;
    }
    for (const phase of phases) {
      const state = value[phase];
      if (!record(state) || Object.keys(state).sort().join(",") !== sortedIds) {
        error("$." + name + ".reflow." + phase, "three named rectangles required");
        continue;
      }
      for (const id of ids) {
        const box = state[id];
        if (box === null) continue; // Explicit display:none; never missing/undefined.
        if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
          error("$." + name + ".reflow." + phase + "." + id, "complete rectangle or null required");
          continue;
        }
        for (const field of rectFields) {
          if (!Number.isFinite(box[field]) ||
              ((field === "width" || field === "height") && box[field] < 0)) {
            error("$." + name + ".reflow." + phase + "." + id + "." + field,
              "finite CSS-pixel rectangle coordinate required");
          }
        }
      }
    }
  }
  validateReflow("native", native?.reflow);
  validateReflow("browser", browser?.reflow);

  // Independent CSS percentage-height observations remain a separate
  // measurement channel; incomplete boxes never count as agreement.
  function validateHeightBoxes(owner, value) {
    const path = "$." + owner + ".percentageHeights";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...heightIds].sort().join(",")) {
      error(path, "exactly the three named percentage-height border boxes required");
      return;
    }
    for (const id of heightIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete percentage-height rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateHeightBoxes("native", native?.percentageHeights);
  validateHeightBoxes("browser", browser?.percentageHeights);

  function validateBorderBoxes(owner, value) {
    const path = "$." + owner + ".borderBoxes";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...borderIds].sort().join(",")) {
      error(path, "exactly the three named border-box rectangles required");
      return;
    }
    for (const id of borderIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete border-box rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateBorderBoxes("native", native?.borderBoxes);
  validateBorderBoxes("browser", browser?.borderBoxes);

  // Source-captured min/max-width rectangles are an independent geometry
  // channel. A missing field is not a zero or an observed equal.
  function validateWidthBounds(owner, value) {
    const path = "$." + owner + ".widthBounds";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...widthBoundIds].sort().join(",")) {
      error(path, "exactly the three named constrained-width boxes required");
      return;
    }
    for (const id of widthBoundIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete constrained-width rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateWidthBounds("native", native?.widthBounds);
  validateWidthBounds("browser", browser?.widthBounds);

  function validateHeightBounds(owner, value) {
    const path = "$." + owner + ".heightBounds";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...heightBoundIds].sort().join(",")) {
      error(path, "exactly three named constrained-height boxes required");
      return;
    }
    for (const id of heightBoundIds) {
      const box = value[id];
      if (!record(box) || Object.keys(box).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(path + "." + id, "complete constrained-height rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(box[field]) ||
            (["width", "height"].includes(field) && box[field] < 0)) {
          error(path + "." + id + "." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateHeightBounds("native", native?.heightBounds);
  validateHeightBounds("browser", browser?.heightBounds);

  function validateScrollViewport(owner, value) {
    const path = "$." + owner + ".scrollViewport";
    const expected = [...scrollPhases, "documentHeight"].sort().join(",");
    if (!record(value) || Object.keys(value).sort().join(",") !== expected) {
      error(path, "document height and exactly three scroll phases required");
      return;
    }
    if (!Number.isSafeInteger(value.documentHeight) || value.documentHeight < 600) {
      error(path + ".documentHeight", "valid document extent required");
    }
    for (const phase of scrollPhases) {
      const sample = value[phase];
      const point = path + "." + phase;
      if (!record(sample) ||
          Object.keys(sample).sort().join(",") !==
            ["scrollX", "scrollY", "targetRect", "hitTarget"].sort().join(",")) {
        error(point, "complete document-scroll sample required");
        continue;
      }
      for (const key of ["scrollX", "scrollY"]) {
        if (!Number.isSafeInteger(sample[key]) || sample[key] < 0) {
          error(point + "." + key, "finite nonnegative scroll position required");
        }
      }
      if (sample.hitTarget !== null && sample.hitTarget !== "scroll-target") {
        error(point + ".hitTarget", "known scroll target or explicit null required");
      }
      const rect = sample.targetRect;
      if (!record(rect) || Object.keys(rect).sort().join(",") !== [...rectFields].sort().join(",")) {
        error(point + ".targetRect", "complete target rectangle required");
        continue;
      }
      for (const field of rectFields) {
        if (!Number.isFinite(rect[field]) ||
            (["width", "height"].includes(field) && rect[field] < 0)) {
          error(point + ".targetRect." + field, "finite CSS-pixel rectangle required");
        }
      }
    }
  }
  validateScrollViewport("native", native?.scrollViewport);
  validateScrollViewport("browser", browser?.scrollViewport);

  const targetPhases = ["original", "hidden", "restored"];
  function validateTargets(owner, value) {
    const path = "$." + owner + ".hitTargets";
    if (!record(value) ||
        Object.keys(value).sort().join(",") !== [...targetPhases].sort().join(",")) {
      error(path, "exactly original, hidden and restored target phases required");
      return;
    }
    for (const phase of targetPhases) {
      const map = value[phase];
      if (!record(map) || Object.keys(map).sort().join(",") !== [...probes].sort().join(",")) {
        error(path + "." + phase, "exactly the seven named hit probes required");
        continue;
      }
      for (const probe of probes) {
        if (map[probe] !== null && !ids.includes(map[probe])) {
          error(path + "." + phase + "." + probe, "known target ID or explicit null required");
        }
      }
    }
  }
  validateTargets("native", native?.hitTargets);
  validateTargets("browser", browser?.hitTargets);
  if (!invalid.length) {
    if (browser.fixtureId !== native.fixtureId) {
      mismatch("$.fixtureId", native.fixtureId, browser.fixtureId);
    }
    if (browser.environment.viewportWidth !== native.viewport.width ||
        browser.environment.viewportHeight !== native.viewport.height ||
        browser.environment.devicePixelRatio !== native.viewport.devicePixelRatio) {
      mismatch("$.environment.viewport", native.viewport, browser.environment);
    }
    if (browser.observations.length !== ids.length ||
        !ids.every((id, index) => browser.observations[index]?.id === id)) {
      mismatch("$.observations", ids, browser.observations.map(o => o.id));
    }
    const p = browser.provenance;
    if (!record(p) || p.collector !== "playwright" ||
        !Object.hasOwn(families, p.engine) || p.engineFamily !== families[p.engine] ||
        !["browserVersion", "playwrightVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]
          .every(key => typeof p[key] === "string" && p[key].trim() && p[key] !== "unrecorded") ||
        !record(p.viewport) || p.viewport.width !== native.viewport.width ||
        p.viewport.height !== native.viewport.height ||
        p.viewport.devicePixelRatio !== native.viewport.devicePixelRatio ||
        p.fixtureSha256 !== sourceSha256) {
      mismatch("$.provenance", "matching Playwright source receipt, engine and viewport", p ?? null);
    }
    for (const [index, id] of ids.entries()) {
      const obs = browser.observations[index];
      for (const key of computedFields) {
        if (!Object.hasOwn(obs?.computed ?? {}, key) || !obs.computed[key]) {
          error("$.browser.observations[" + index + "].computed." + key,
            "required browser-observed computed property");
        }
      }
      if (obs?.text !== "") {
        mismatch("$.observations[" + index + "].text", "", obs?.text ?? null);
      }
    }
  }
  if (invalid.length) return report;
  if (incompatible.length) {
    report.status = "incompatible";
    return report;
  }
  for (const [index, id] of ids.entries()) {
    for (const field of rectFields) {
      const left = native.geometry[id][field], right = browser.observations[index].rect[field];
      if (!Object.is(left, right)) {
        differences.push({ id, field, left, right, path: "$.observations[" + index + "].rect." + field });
      }
    }
  }
  // A hidden node has no box at all. Compare visibility as well as border
  // geometry, then compare the restored state independently of the baseline.
  for (const phase of phases) {
    for (const id of ids) {
      const left = native.reflow[phase][id], right = browser.reflow[phase][id];
      if (left === null || right === null) {
        if (left !== right) {
          differences.push({ kind: "visibility", phase, id, field: "visible",
            left: left !== null, right: right !== null,
            path: "$.reflow." + phase + "." + id });
        }
        continue;
      }
      for (const field of rectFields) {
        if (!Object.is(left[field], right[field])) {
          differences.push({ kind: "geometry", phase, id, field,
            left: left[field], right: right[field],
            path: "$.reflow." + phase + "." + id + "." + field });
        }
      }
    }
  }
  for (const id of heightIds) {
    for (const field of rectFields) {
      const left = native.percentageHeights[id][field];
      const right = browser.percentageHeights[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "percentage-height", id, field, left, right,
          path: "$.percentageHeights." + id + "." + field,
        });
      }
    }
  }
  for (const id of borderIds) {
    for (const field of rectFields) {
      const left = native.borderBoxes[id][field];
      const right = browser.borderBoxes[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "border-box", id, field, left, right,
          path: "$.borderBoxes." + id + "." + field,
        });
      }
    }
  }
  for (const id of widthBoundIds) {
    for (const field of rectFields) {
      const left = native.widthBounds[id][field];
      const right = browser.widthBounds[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "width-bound", id, field, left, right,
          path: "$.widthBounds." + id + "." + field,
        });
      }
    }
  }
  for (const id of heightBoundIds) {
    for (const field of rectFields) {
      const left = native.heightBounds[id][field];
      const right = browser.heightBounds[id][field];
      if (!Object.is(left, right)) {
        differences.push({
          kind: "height-bound", id, field, left, right,
          path: "$.heightBounds." + id + "." + field,
        });
      }
    }
  }
  const nativeScroll = native.scrollViewport, browserScroll = browser.scrollViewport;
  if (!Object.is(nativeScroll.documentHeight, browserScroll.documentHeight)) {
    differences.push({
      kind: "viewport-scroll", field: "documentHeight",
      left: nativeScroll.documentHeight, right: browserScroll.documentHeight,
      path: "$.scrollViewport.documentHeight",
    });
  }
  for (const phase of scrollPhases) {
    const left = nativeScroll[phase], right = browserScroll[phase];
    for (const field of ["scrollX", "scrollY", "hitTarget"]) {
      if (!Object.is(left[field], right[field])) {
        differences.push({
          kind: "viewport-scroll", phase, field, left: left[field], right: right[field],
          path: "$.scrollViewport." + phase + "." + field,
        });
      }
    }
    for (const field of rectFields) {
      if (!Object.is(left.targetRect[field], right.targetRect[field])) {
        differences.push({
          kind: "viewport-scroll", phase, field: "targetRect." + field,
          left: left.targetRect[field], right: right.targetRect[field],
          path: "$.scrollViewport." + phase + ".targetRect." + field,
        });
      }
    }
  }
  // These targets come from independent source-side elementFromPoint and
  // native hit_test calls. A missing/outside target is explicit JSON null.
  for (const phase of targetPhases) {
    for (const probe of probes) {
      const left = native.hitTargets[phase][probe];
      const right = browser.hitTargets[phase][probe];
      if (left !== right) {
        differences.push({
          kind: "hit-target", phase, probe, left, right,
          path: "$.hitTargets." + phase + "." + probe,
        });
      }
    }
  }
  report.status = differences.length ? "different" : "equal";
  return report;
}

async function main(args) {
  if (args.length !== 2) throw new Error("usage: node tests/browser-oracles/native-block-bridge.mjs native.json browser.json");
  const [native, browser, fixture] = await Promise.all([
    ...args.map(path => readFile(path, "utf8").then(JSON.parse)),
    readFile(new URL("./native-block-bridge.html", import.meta.url)),
  ]);
  const report = compareNativeBlockBridge(native, browser, fixture);
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exitCode = report.status === "equal" ? 0 : report.status === "different" ? 1 : 2;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
