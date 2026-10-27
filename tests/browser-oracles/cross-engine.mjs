import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { compareCaptures, validateCapture } from "./compare.mjs";

const families = { chromium: "blink", firefox: "gecko", webkit: "webkit" };
const engines = Object.keys(families);
const hostFields = ["playwrightVersion", "hostPlatform", "hostArchitecture", "nodeVersion"];
const record = value => value !== null && typeof value === "object" && !Array.isArray(value);
const sha256 = value => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);

function inspect(capture, engine, expectedHash) {
  if (capture === undefined) return { status: "missing", errors: [], capture: null };
  const errors = validateCapture(capture);
  const error = (path, message) => errors.push({ path, message });
  const p = capture?.provenance;
  if (!record(p)) {
    error("$.provenance", "collector provenance required");
  } else {
    for (const [key, expected] of Object.entries({ collector: "playwright", engine, engineFamily: families[engine] })) {
      if (p[key] !== expected) error(`$.provenance.${key}`, `expected ${expected}`);
    }
    for (const key of [...hostFields, "browserVersion"]) {
      if (typeof p[key] !== "string" || !p[key].trim() || p[key].trim() === "unrecorded") {
        error(`$.provenance.${key}`, "recorded nonempty string required");
      }
    }
    if (!sha256(p.fixtureSha256)) error("$.provenance.fixtureSha256", "lowercase SHA-256 required");
    if (!record(p.viewport)) {
      error("$.provenance.viewport", "requested viewport required");
    } else {
      for (const [key, observed] of [["width", "viewportWidth"], ["height", "viewportHeight"], ["devicePixelRatio", "devicePixelRatio"]]) {
        if (!Number.isFinite(p.viewport[key]) || p.viewport[key] <= 0 ||
            p.viewport[key] !== capture.environment?.[observed]) {
          error(`$.provenance.viewport.${key}`, "must match the positive observed viewport value");
        }
      }
    }
  }
  if (errors.length) return { status: "invalid", errors, capture };
  if (p.fixtureSha256 !== expectedHash) {
    return { status: "incompatible", errors: [{ path: "$.provenance.fixtureSha256",
      message: "source receipt does not match supplied source bytes", expected: expectedHash, actual: p.fixtureSha256 }], capture };
  }
  return { status: "ready", errors, capture };
}

/** Compare recorded fields, not conformance; receipts are not authenticated. */
export function crossEngineReport(captures, expectedSourceSha256) {
  if (!record(captures) || Object.keys(captures).some(key => !engines.includes(key))) {
    throw new TypeError("captures must be an object keyed only by chromium, firefox and webkit");
  }
  if (!sha256(expectedSourceSha256)) throw new TypeError("expected source SHA-256 required");
  const records = Object.fromEntries(engines.map(engine => [engine,
    inspect(Object.hasOwn(captures, engine) ? captures[engine] : undefined, engine, expectedSourceSha256)]));
  const pairs = [];
  for (let i = 0; i < engines.length; i += 1) {
    for (let j = i + 1; j < engines.length; j += 1) {
      const left = engines[i], right = engines[j];
      const pair = { engines: [left, right], status: "not-compared", incompatibilities: [], differences: [] };
      pairs.push(pair);
      if (records[left].status !== "ready" || records[right].status !== "ready") continue;
      const a = records[left].capture, b = records[right].capture;
      for (const key of hostFields) {
        if (a.provenance[key] !== b.provenance[key]) {
          pair.incompatibilities.push({ path: `$.provenance.${key}`, left: a.provenance[key], right: b.provenance[key] });
        }
      }
      // No tolerance or CSS-string normalization in cross-engine evidence.
      const comparison = compareCaptures(a, b);
      pair.incompatibilities.push(...comparison.incompatibilities);
      pair.status = pair.incompatibilities.length ? "incompatible" : comparison.status;
      if (pair.status !== "incompatible") pair.differences = comparison.differences;
    }
  }
  const states = Object.values(records).map(value => value.status);
  const status = states.includes("invalid") ? "invalid"
    : states.includes("missing") ? "incomplete"
    : states.includes("incompatible") || pairs.some(p => p.status === "incompatible") ? "incompatible"
    : pairs.some(p => p.status === "different") ? "different" : "equal";
  return {
    schemaVersion: 1, kind: "browser-cross-engine-report", status,
    conformance: "not-assessed", expectedSourceSha256,
    comparisonScope: "reported observations and recorded host fields; fonts and external assets not verified",
    coverage: { expectedEngines: 3, presentEngines: states.filter(s => s !== "missing").length,
      readyEngines: states.filter(s => s === "ready").length,
      comparedPairs: pairs.filter(p => p.status === "equal" || p.status === "different").length, expectedPairs: 3 },
    engines: records, pairs
  };
}

export function reportExitCode(report) {
  return report.status === "equal" ? 0 : report.status === "different" ? 1 : 2;
}

async function main(args) {
  if (args.length !== 4) {
    throw new Error("Usage: node tests/browser-oracles/cross-engine.mjs SOURCE.html CHROMIUM.json FIREFOX.json WEBKIT.json (use - for a missing engine)");
  }
  const source = await readFile(args[0]);
  const expectedHash = createHash("sha256").update(source).digest("hex");
  const inputs = await Promise.all(args.slice(1).map(async file => {
    if (file === "-") return { capture: undefined };
    try {
      return { capture: JSON.parse(await readFile(file, "utf8")) };
    } catch (error) {
      return { capture: error.code === "ENOENT" ? undefined : null, error: String(error) };
    }
  }));
  const report = crossEngineReport(Object.fromEntries(engines.map((engine, i) => [engine, inputs[i].capture])), expectedHash);
  report.inputs = { source: args[0], captures: Object.fromEntries(engines.map((engine, i) =>
    [engine, { path: args[i + 1], error: inputs[i].error ?? null }])) };
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exitCode = reportExitCode(report);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
