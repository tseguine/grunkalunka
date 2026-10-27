// Dependency-free, deliberately narrow comparison of browser observation captures.
// Equality here is differential evidence, never a conformance claim.
function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Refuse numeric tolerance when conversion loses a nonzero value to zero.
// Exact string comparison remains available for these exceptional values.
function finiteDecimal(text) {
  const value = Number(text);
  const nonzeroSignificand = /[1-9]/.test(text.split(/[eE]/, 1)[0]);
  return Number.isFinite(value) && !(value === 0 && nonzeroSignificand) ? value : NaN;
}

export function validateCapture(capture) {
  const errors = [];
  const error = (path, message) => errors.push({ path, message });
  if (!isRecord(capture)) {
    error("$", "capture must be a JSON object");
    return errors;
  }
  if (capture.schemaVersion !== 1) error("$.schemaVersion", "expected schema version 1");
  if (typeof capture.fixtureId !== "string" || capture.fixtureId.length === 0) {
    error("$.fixtureId", "expected nonempty string");
  }
  const env = capture.environment;
  if (!isRecord(env)) {
    error("$.environment", "expected object");
  } else {
    if (typeof env.userAgent !== "string" || env.userAgent.length === 0) {
      error("$.environment.userAgent", "expected nonempty string");
    }
    if (typeof env.url !== "string" || env.url.length === 0) {
      error("$.environment.url", "expected nonempty string");
    }
    if (typeof env.timestamp !== "string" || !Number.isFinite(Date.parse(env.timestamp))) {
      error("$.environment.timestamp", "expected parseable timestamp string");
    }
    for (const name of ["viewportWidth", "viewportHeight", "devicePixelRatio"]) {
      if (!Number.isFinite(env[name]) || env[name] <= 0) {
        error(`$.environment.${name}`, "expected finite positive number");
      }
    }
  }
  if (!Array.isArray(capture.observations) || capture.observations.length === 0) {
    error("$.observations", "expected nonempty array");
  } else {
    const ids = new Set();
    for (let i = 0; i < capture.observations.length; i += 1) {
      const obs = capture.observations[i];
      const root = `$.observations[${i}]`;
      if (!isRecord(obs)) {
        error(root, "expected object");
        continue;
      }
      if (typeof obs.id !== "string" || obs.id.length === 0) {
        error(`${root}.id`, "expected nonempty string");
      } else if (ids.has(obs.id)) {
        error(`${root}.id`, `duplicate observation ID ${JSON.stringify(obs.id)}`);
      } else {
        ids.add(obs.id);
      }
      if (typeof obs.text !== "string") error(`${root}.text`, "expected string");
      if (!isRecord(obs.computed)) {
        error(`${root}.computed`, "expected object");
      } else {
        for (const [key, value] of Object.entries(obs.computed)) {
          if (typeof value !== "string") {
            error(`${root}.computed.${key}`, "expected computed-style string");
          }
        }
      }
      if (!isRecord(obs.rect)) {
        error(`${root}.rect`, "expected object");
      } else {
        for (const key of ["x", "y", "width", "height"]) {
          if (typeof obs.rect[key] !== "number" || !Number.isFinite(obs.rect[key])) {
            error(`${root}.rect.${key}`, "expected finite number");
          }
        }
        if (typeof obs.rect.width === "number" && obs.rect.width < 0) {
          error(`${root}.rect.width`, "expected nonnegative width");
        }
        if (typeof obs.rect.height === "number" && obs.rect.height < 0) {
          error(`${root}.rect.height`, "expected nonnegative height");
        }
      }
    }
  }
  return errors;
}

export function compareCaptures(left, right, options = {}) {
  const rectTolerance = options.rectTolerance ?? 0;
  const stylePxTolerance = options.stylePxTolerance ?? 0;
  for (const [key, value] of Object.entries({ rectTolerance, stylePxTolerance })) {
    if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
      throw new TypeError(`${key} must be a finite nonnegative number`);
    }
  }
  const leftErrors = validateCapture(left);
  const rightErrors = validateCapture(right);
  const report = {
    schemaVersion: 1,
    kind: "browser-oracle-comparison",
    tolerances: { rectCssPx: rectTolerance, computedCssPx: stylePxTolerance },
    status: "invalid",
    leftErrors,
    rightErrors,
    incompatibilities: [],
    differences: [],
    captures: { left, right }
  };
  if (leftErrors.length || rightErrors.length) return report;

  const incompatible = (path, leftValue, rightValue) => {
    report.incompatibilities.push({ path, left: leftValue, right: rightValue });
  };
  if (left.fixtureId !== right.fixtureId) {
    incompatible("$.fixtureId", left.fixtureId, right.fixtureId);
  }
  for (const name of ["viewportWidth", "viewportHeight", "devicePixelRatio"]) {
    if (!Object.is(left.environment[name], right.environment[name])) {
      incompatible(`$.environment.${name}`, left.environment[name], right.environment[name]);
    }
  }
  if (left.observations.length !== right.observations.length) {
    incompatible("$.observations.length", left.observations.length, right.observations.length);
  } else {
    for (let i = 0; i < left.observations.length; i += 1) {
      if (left.observations[i].id !== right.observations[i].id) {
        incompatible(`$.observations[${i}].id`, left.observations[i].id, right.observations[i].id);
      }
    }
  }
  if (report.incompatibilities.length) {
    report.status = "incompatible";
    return report;
  }
  const different = (kind, path, expected, actual, tolerance) => {
    const entry = { kind, path, left: expected, right: actual };
    if (tolerance !== undefined) entry.tolerance = tolerance;
    report.differences.push(entry);
  };
  const px = /^(-?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?)px$/i;
  for (let i = 0; i < left.observations.length; i += 1) {
    const a = left.observations[i], b = right.observations[i];
    const root = `$.observations[${i}]`;
    if (a.text !== b.text) different("text", `${root}.text`, a.text, b.text);
    const keys = [...new Set([...Object.keys(a.computed), ...Object.keys(b.computed)])].sort();
    for (const key of keys) {
      const path = `${root}.computed.${key}`;
      const hasA = Object.hasOwn(a.computed, key), hasB = Object.hasOwn(b.computed, key);
      if (!hasA || !hasB) {
        different("missing-property", path, hasA ? a.computed[key] : null, hasB ? b.computed[key] : null);
        continue;
      }
      const aValue = a.computed[key], bValue = b.computed[key];
      if (aValue === bValue) continue;
      const apx = px.exec(aValue), bpx = px.exec(bValue);
      // Zero (including the default) means exact CSS string comparison.
      if (stylePxTolerance > 0 && apx && bpx &&
          Math.abs(finiteDecimal(apx[1]) - finiteDecimal(bpx[1])) <= stylePxTolerance) continue;
      different("computed-style", path, aValue, bValue, apx && bpx ? stylePxTolerance : undefined);
    }
    for (const key of ["x", "y", "width", "height"]) {
      const av = a.rect[key], bv = b.rect[key];
      if (Math.abs(av - bv) > rectTolerance) {
        different("geometry", `${root}.rect.${key}`, av, bv, rectTolerance);
      }
    }
  }
  report.status = report.differences.length ? "different" : "equal";
  return report;
}


import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

function usage() {
  return "Usage: node tests/browser-oracles/compare.mjs left.json right.json [--rect-tolerance=CSS_PX] [--style-px-tolerance=CSS_PX]";
}

async function main(args) {
  const files = [];
  const options = {};
  for (const arg of args) {
    if (arg.startsWith("--rect-tolerance=") || arg.startsWith("--style-px-tolerance=")) {
      const separator = arg.indexOf("=");
      const name = arg.slice(0, separator), raw = arg.slice(separator + 1);
      if (!/^(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(raw)) {
        throw new Error(`invalid tolerance: ${arg}`);
      }
      const value = finiteDecimal(raw);
      if (!Number.isFinite(value) || value < 0) throw new Error(`invalid tolerance: ${arg}`);
      const field = name === "--rect-tolerance" ? "rectTolerance" : "stylePxTolerance";
      if (Object.hasOwn(options, field)) throw new Error(`duplicate tolerance: ${name}`);
      options[field] = value;
    } else if (arg.startsWith("-")) {
      throw new Error(`unknown option: ${arg}\n${usage()}`);
    } else {
      files.push(arg);
    }
  }
  if (files.length !== 2) throw new Error(usage());
  const [left, right] = await Promise.all(files.map(async file => {
    return JSON.parse(await readFile(file, "utf8"));
  }));
  const report = compareCaptures(left, right, options);
  process.stdout.write(JSON.stringify(report, null, 2) + "\n");
  process.exitCode = report.status === "equal" ? 0 : report.status === "different" ? 1 : 2;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
