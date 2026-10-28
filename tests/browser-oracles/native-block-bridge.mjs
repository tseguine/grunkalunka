// Selected native block-border geometry versus a real browser receipt.
// This is not the generic browser-vs-browser comparator or a CSS conformance test.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateCapture } from "./compare.mjs";

const ids = ["outer", "first", "second"];
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
    comparisonScope: "three border rectangles from an explicit resolved block tree; no HTML/CSS parsing, inline layout or paint",
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
