// Bounded real-engine hit-target negative control. No native CSS pointer-events
// semantics are claimed: the original native input policy remains unchanged.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { collectOne } from "./collect.mjs";
import { compareNativeBlockBridge } from "../../tests/browser-oracles/native-block-bridge.mjs";

const ORIGINAL_RULE = "#first { width: 100px;";
const MUTATED_RULE = "#first { pointer-events: none; width: 100px;";
const affected = [
  ["original", "first-center"],
  ["original", "first-lower"],
  ["original", "first-right"],
  ["restored", "first-center"],
  ["restored", "first-lower"],
  ["restored", "first-right"],
];
export const EXPECTED_HIT_DIFFERENCES = Object.freeze(affected.map(([phase, probe]) => ({
  kind: "hit-target", path: "$.hitTargets." + phase + "." + probe,
  phase, probe, left: "first", right: "outer",
})));

export function mutateNativeBridgeHitSource(originalBytes) {
  if (!(originalBytes instanceof Uint8Array)) {
    throw new TypeError("original HTML bytes required");
  }
  const original = Buffer.from(originalBytes).toString("utf8");
  assert.equal(original.split(ORIGINAL_RULE).length, 2,
    "exactly one reviewed hit-target mutation site required");
  return Buffer.from(original.replace(ORIGINAL_RULE, MUTATED_RULE), "utf8");
}

export function verifyNativeHitSensitivity(report) {
  assert.equal(report?.status, "different", "real hit mutation must be detected");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, [], "modified source must remain valid");
  assert.deepEqual(report.incompatible, [], "capture must match modified source");
  assert.deepEqual(report.differences, EXPECTED_HIT_DIFFERENCES,
    "only six first-child input targets may change; geometry must remain equal");
}

async function main(args) {
  if (args.length !== 3 || !["chromium", "firefox", "webkit"].includes(args[0])) {
    throw new Error("usage: node tools/browser-oracles/native-hit-sensitivity.mjs ENGINE NATIVE_JSON NEW_OUTPUT_DIR");
  }
  const [engine, nativeFile, output] = args;
  const out = resolve(output);
  const source = await readFile(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
  const mutated = mutateNativeBridgeHitSource(source);
  const native = JSON.parse(await readFile(nativeFile, "utf8"));

  await mkdir(dirname(out), { recursive: true });
  await mkdir(out); // No stale artifacts or overwritten snapshots.
  const html = join(out, "pointer-events-mutation.html");
  await writeFile(html, mutated, { flag: "wx", mode: 0o444 });
  const types = await import("playwright");
  const require = createRequire(import.meta.url);
  const version = require("playwright/package.json").version;
  const capture = await collectOne(engine, {
    browserTypes: types, playwrightVersion: version, fixturePath: html,
  });
  await writeFile(join(out, "mutated-capture.json"),
    JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });

  // Compare to the original native policy but validate against the mutated
  // HTML source bytes, preserving the independent browser provenance receipt.
  const report = compareNativeBlockBridge(native, capture, mutated);
  await writeFile(join(out, "intentional-hit-mutation.json"),
    JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  verifyNativeHitSensitivity(report);
  console.log("PASS native bridge real-engine hit sensitivity " +
    JSON.stringify({ engine, sourceSha256: report.sourceSha256,
      differences: report.differences, conformance: report.conformance }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
