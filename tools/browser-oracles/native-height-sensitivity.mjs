// Independent real-engine negative control for definite percentage-height geometry.
// Original native geometry remains untouched; disagreement is expected only for
// the two fields downstream of one reviewed CSS percentage-height source edit.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { collectOne } from "./collect.mjs";
import { compareNativeBlockBridge } from "../../tests/browser-oracles/native-block-bridge.mjs";

const ORIGINAL_RULE = "#height-first { width: 30px; height: 50%;";
const MUTATED_RULE = "#height-first { width: 30px; height: 60%;";

export const EXPECTED_HEIGHT_DIFFERENCES = Object.freeze([
  {
    kind: "percentage-height", id: "height-first", field: "height",
    left: 46, right: 54, path: "$.percentageHeights.height-first.height",
  },
  {
    kind: "percentage-height", id: "height-second", field: "y",
    left: 156, right: 164, path: "$.percentageHeights.height-second.y",
  },
]);

export function mutateNativeHeightSource(originalBytes) {
  if (!(originalBytes instanceof Uint8Array)) {
    throw new TypeError("original HTML bytes required");
  }
  const original = Buffer.from(originalBytes).toString("utf8");
  assert.equal(original.split(ORIGINAL_RULE).length, 2,
    "exactly one reviewed percentage-height mutation site required");
  return Buffer.from(original.replace(ORIGINAL_RULE, MUTATED_RULE), "utf8");
}

export function verifyNativeHeightSensitivity(report) {
  assert.equal(report?.status, "different", "real height mutation must be detected");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, [], "height mutation must retain valid evidence");
  assert.deepEqual(report.incompatible, [], "capture must match modified source bytes");
  assert.deepEqual(report.differences, EXPECTED_HEIGHT_DIFFERENCES,
    "only the modified child's height and following sibling's y may differ");
}

async function main(args) {
  if (args.length !== 3 || !["chromium", "firefox", "webkit"].includes(args[0])) {
    throw new Error("usage: node tools/browser-oracles/native-height-sensitivity.mjs ENGINE NATIVE_JSON NEW_OUTPUT_DIR");
  }
  const [engine, nativeFile, output] = args;
  const out = resolve(output);
  const source = await readFile(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
  const mutated = mutateNativeHeightSource(source);
  const native = JSON.parse(await readFile(nativeFile, "utf8"));

  await mkdir(dirname(out), { recursive: true });
  await mkdir(out); // Fresh directory; never mix previous evidence into this run.
  const html = join(out, "percentage-height-mutation.html");
  await writeFile(html, mutated, { flag: "wx", mode: 0o444 });

  const browserTypes = await import("playwright");
  const require = createRequire(import.meta.url);
  const version = require("playwright/package.json").version;
  // A fresh browser instance executes the changed HTML. No synthetic capture
  // or modification of the original Rust evidence is permitted.
  const capture = await collectOne(engine, {
    browserTypes, playwrightVersion: version, fixturePath: html,
  });
  await writeFile(join(out, "mutated-capture.json"),
    JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });

  // Validate provenance against the MUTATED bytes while comparing the original
  // independent native receipt. Exactly two intentional differences must remain.
  const report = compareNativeBlockBridge(native, capture, mutated);
  await writeFile(join(out, "intentional-height-mutation.json"),
    JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  verifyNativeHeightSensitivity(report);
  console.log("PASS native bridge real-engine percentage-height sensitivity " +
    JSON.stringify({ engine, sourceSha256: report.sourceSha256,
      differences: report.differences, conformance: report.conformance }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
