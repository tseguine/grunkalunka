// Exact comparison of independent native row layout with browser observations.
// This selected fixed-size profile is not a general CSS flex conformance test.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateCapture } from "./compare.mjs";

const ids = ["row-host", "row-a", "row-b", "row-inner", "row-c", "row-after"];
const phases = ["nowrap", "wrap", "wide", "columnGap", "rowGap", "hidden",
  "relative", "clipped", "scrolled", "justifyEnd", "justifyCenter", "spaceBetween",
  "alignEnd", "alignCenter", "wrapAlign", "wrapSingle", "safeOverflow", "overlapClipped",
  "nestedOverlap", "nestedOverflow", "relativeAlign", "clippedAlign", "scrolledAlign", "restored"];
const probes = ["a", "b", "nowrap", "wrap", "gap-wrap", "after", "host-border", "gap", "outside"];
const rectFields = ["x", "y", "width", "height"];
const viewportFields = ["width", "height", "devicePixelRatio"];
const computedFields = ["display", "boxSizing", "width", "height", "flexDirection",
  "flexWrap", "alignItems", "alignContent", "justifyContent", "columnGap", "rowGap",
  "flexGrow", "flexShrink", "flexBasis", "minWidth"];
const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const keys = (value, names) => record(value) && Object.keys(value).length === names.length &&
  names.every(name => Object.hasOwn(value, name));
const sameRect = (left, right) => left === null || right === null ? left === right :
  rectFields.every(field => Object.is(left[field], right[field]));

export function compareNativeFlexBridge(native, browser, sourceBytes) {
  if (!(sourceBytes instanceof Uint8Array) || sourceBytes.byteLength === 0) {
    throw new TypeError("nonempty fixture bytes required");
  }
  const sourceSha256 = createHash("sha256").update(sourceBytes).digest("hex");
  const invalid = [], incompatible = [], differences = [];
  const error = (path, message) => invalid.push({ path, message });
  const mismatch = (path, left, right) => incompatible.push({ path, left, right });
  const report = {
    schemaVersion: 1,
    kind: "grunkalunka-native-browser-flex",
    status: "invalid",
    conformance: "not-assessed",
    comparisonScope: "exact selected fixed-size row/wrap/alignment geometry and viewport hits, including non-positioned flex overlap; no CSS parsing, flex factors, intrinsic flex sizing or general stacking",
    sourceSha256, invalid, incompatible, differences, native, browser
  };

  if (!keys(native, ["schemaVersion", "kind", "fixtureId", "viewport", "states"]) ||
      native.schemaVersion !== 1 || native.kind !== "grunkalunka-native-flex-geometry" ||
      native.fixtureId !== "native-flex-bridge-v1") {
    error("$.native", "complete native flex bridge v1 receipt required");
  }
  if (!keys(native?.viewport, viewportFields) || native.viewport.width !== 800 ||
      native.viewport.height !== 600 || native.viewport.devicePixelRatio !== 1) {
    error("$.native.viewport", "fixed 800x600 DPR1 viewport required");
  }
  for (const issue of validateCapture(browser)) {
    error("$.browser" + issue.path.slice(1), issue.message);
  }
  if (!keys(browser, ["schemaVersion", "fixtureId", "environment", "observations", "states", "provenance"]) ||
      browser.fixtureId !== "native-flex-bridge-v1") {
    error("$.browser", "complete browser flex bridge v1 receipt required");
  }
  if (!keys(browser?.environment, ["userAgent", "viewportWidth", "viewportHeight",
    "devicePixelRatio", "url", "timestamp"])) {
    error("$.browser.environment", "complete browser environment required");
  }

  function validateRect(value, path) {
    if (!keys(value, rectFields)) {
      error(path, "complete border rectangle required");
      return;
    }
    for (const field of rectFields) {
      if (!Number.isSafeInteger(value[field]) || value[field] < 0) {
        error(path + "." + field, "nonnegative exact integer CSS pixels required");
      }
    }
  }

  function validateStates(owner, states) {
    const path = "$." + owner + ".states";
    if (!keys(states, phases)) {
      error(path, "exactly the twenty-four named flex phases required");
      return;
    }
    for (const phase of phases) {
      const sample = states[phase];
      const point = path + "." + phase;
      if (!keys(sample, ["scrollY", "boxes", "hits"])) {
        error(point, "complete flex sample required");
        continue;
      }
      if (sample.scrollY !== (["scrolled", "scrolledAlign"].includes(phase) ? 10 : 0)) {
        error(point + ".scrollY", "actual phase-specific scroll position required");
      }
      if (!keys(sample.boxes, ids)) {
        error(point + ".boxes", "exactly the six named border boxes required");
      } else {
        for (const id of ids) {
          const field = point + ".boxes." + id;
          if (phase === "hidden" && ["row-b", "row-inner"].includes(id)) {
            if (sample.boxes[id] !== null) error(field, "suppressed box must be explicit null");
          } else {
            validateRect(sample.boxes[id], field);
          }
        }
      }
      if (!keys(sample.hits, probes)) {
        error(point + ".hits", "exactly the nine named viewport probes required");
      } else {
        for (const probe of probes) {
          const target = sample.hits[probe];
          if (target !== null && !ids.includes(target)) {
            error(point + ".hits." + probe, "known target ID or explicit null required");
          } else if (target !== null && sample.boxes?.[target] === null) {
            error(point + ".hits." + probe, "a suppressed box cannot be a hit target");
          }
        }
      }
    }
  }
  validateStates("native", native?.states);
  validateStates("browser", browser?.states);

  if (!Array.isArray(browser?.observations) || browser.observations.length !== ids.length) {
    error("$.browser.observations", "six ordered baseline observations required");
  } else {
    for (const [index, id] of ids.entries()) {
      const obs = browser.observations[index];
      const path = "$.browser.observations[" + index + "]";
      if (!keys(obs, ["id", "text", "computed", "rect"]) || obs.id !== id || obs.text !== "") {
        error(path, "ordered, empty-text baseline observation required for " + id);
        continue;
      }
      if (!keys(obs.computed, computedFields) || !computedFields.every(field =>
        typeof obs.computed[field] === "string" && obs.computed[field].trim())) {
        error(path + ".computed", "complete browser-observed row style required");
      }
      validateRect(obs.rect, path + ".rect");
    }
  }

  const p = browser?.provenance;
  if (!keys(p, ["collector", "playwrightVersion", "engine", "engineFamily", "browserVersion",
    "fixtureSha256", "hostPlatform", "hostArchitecture", "nodeVersion", "viewport"]) ||
      !["browserVersion", "playwrightVersion", "hostPlatform", "hostArchitecture", "nodeVersion"]
        .every(field => typeof p[field] === "string" && p[field].trim() && p[field] !== "unrecorded") ||
      !keys(p.viewport, viewportFields) || typeof p.fixtureSha256 !== "string" ||
      !/^[a-f0-9]{64}$/.test(p.fixtureSha256)) {
    error("$.browser.provenance", "complete source-hashed reference-engine provenance required");
  }
  if (invalid.length) return report;

  if (browser.environment.viewportWidth !== native.viewport.width ||
      browser.environment.viewportHeight !== native.viewport.height ||
      browser.environment.devicePixelRatio !== native.viewport.devicePixelRatio) {
    mismatch("$.environment.viewport", native.viewport, browser.environment);
  }
  if (p.collector !== "playwright" || !Object.hasOwn(families, p.engine) ||
      p.engineFamily !== families[p.engine] || p.fixtureSha256 !== sourceSha256 ||
      !viewportFields.every(field => p.viewport[field] === native.viewport[field])) {
    mismatch("$.provenance", "matching Playwright source receipt, engine family and viewport", p);
  }
  if (incompatible.length) {
    report.status = "incompatible";
    return report;
  }

  // Internal channels must describe the same baseline, and a global scroll
  // must preserve document-space rectangles. Compare restoration for each
  // source independently so copying the same broken final state cannot pass.
  for (const [index, id] of ids.entries()) {
    if (!sameRect(browser.observations[index].rect, browser.states.nowrap.boxes[id])) {
      error("$.browser.observations[" + index + "].rect", "baseline channels disagree");
    }
  }
  for (const [owner, receipt] of [["native", native], ["browser", browser]]) {
    for (const id of ids) {
      if (!sameRect(receipt.states.nowrap.boxes[id], receipt.states.restored.boxes[id])) {
        error("$." + owner + ".states.restored.boxes." + id, "baseline geometry was not restored");
      }
      if (!sameRect(receipt.states.wrap.boxes[id], receipt.states.scrolled.boxes[id])) {
        error("$." + owner + ".states.scrolled.boxes." + id, "scroll changed document-space geometry");
      }
      if (!sameRect(receipt.states.alignCenter.boxes[id], receipt.states.scrolledAlign.boxes[id])) {
        error("$." + owner + ".states.scrolledAlign.boxes." + id, "scroll changed aligned document-space geometry");
      }
      if (!sameRect(receipt.states.safeOverflow.boxes[id], receipt.states.overlapClipped.boxes[id])) {
        error("$." + owner + ".states.overlapClipped.boxes." + id, "clipping changed overlap layout geometry");
      }
    }
    for (const probe of probes) {
      if (receipt.states.nowrap.hits[probe] !== receipt.states.restored.hits[probe]) {
        error("$." + owner + ".states.restored.hits." + probe, "baseline input target was not restored");
      }
    }
  }
  if (invalid.length) return report;

  for (const phase of phases) {
    for (const id of ids) {
      const left = native.states[phase].boxes[id], right = browser.states[phase].boxes[id];
      if (left === null) continue; // Schema validation requires the same suppressed nodes.
      for (const field of rectFields) {
        if (!Object.is(left[field], right[field])) {
          differences.push({ kind: "geometry", path: "$.states." + phase + ".boxes." + id + "." + field,
            left: left[field], right: right[field] });
        }
      }
    }
    for (const probe of probes) {
      const left = native.states[phase].hits[probe], right = browser.states[phase].hits[probe];
      if (left !== right) {
        differences.push({ kind: "hit", path: "$.states." + phase + ".hits." + probe, left, right });
      }
    }
  }
  report.status = differences.length ? "different" : "equal";
  return report;
}

async function main(args) {
  if (args.length !== 3) {
    throw new Error("usage: node tests/browser-oracles/native-flex-bridge.mjs NATIVE_JSON BROWSER_JSON FIXTURE_HTML");
  }
  const [native, browser, source] = await Promise.all(args.map(file => readFile(file)));
  const report = compareNativeFlexBridge(JSON.parse(native), JSON.parse(browser), source);
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exitCode = report.status === "equal" ? 0 : report.status === "different" ? 1 : 2;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
