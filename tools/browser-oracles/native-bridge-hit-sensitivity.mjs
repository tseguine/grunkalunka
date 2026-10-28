// Independent real-engine negative control for native hit-target comparison.
// A CSS pointer-events edit must change live elementFromPoint targets while
// preserving all border geometry, visibility and provenance observations.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { collectOne } from "./collect.mjs";
import { compareNativeBlockBridge } from "../../tests/browser-oracles/native-block-bridge.mjs";

const ORIGINAL = "#first { width: 100px;";
const MUTATED = "#first { pointer-events: none; width: 100px;";
const AFFECTED_PROBES = ["first-center", "first-lower", "first-right"];
const EXPECTED_DIFFERENCES = Object.freeze(
  ["original", "restored"].flatMap(phase => AFFECTED_PROBES.map(probe => ({
    kind: "hit-target", phase, probe,
    path: "$.hitTargets." + phase + "." + probe, left: "first", right: "outer",
  })))
);

export function mutateHitTargetSource(originalBytes) {
  if (!(originalBytes instanceof Uint8Array)) {
    throw new TypeError("original fixture bytes are required");
  }
  const source = Buffer.from(originalBytes).toString("utf8");
  assert.equal(source.split(ORIGINAL).length, 2,
    "exactly one independently reviewed pointer-events mutation site required");
  return Buffer.from(source.replace(ORIGINAL, MUTATED), "utf8");
}

export function verifyHitTargetSensitivity(report) {
  assert.equal(report?.status, "different",
    "real pointer-events mutation must change observed hit targets");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, [], "mutated capture must remain schema-valid");
  assert.deepEqual(report.incompatible, [], "mutated source provenance must match");
  assert.deepEqual(report.differences?.map(d => ({
    kind: d.kind, phase: d.phase, probe: d.probe,
    path: d.path, left: d.left, right: d.right,
  })), EXPECTED_DIFFERENCES,
  "exactly six original/restored targets must differ; geometry and hidden targets must not");
}

async function main(args) {
  if (args.length !== 3 || !["chromium", "firefox", "webkit"].includes(args[0])) {
    throw new Error(
      "usage: node tools/browser-oracles/native-bridge-hit-sensitivity.mjs ENGINE NATIVE_JSON NEW_OUTPUT_DIR"
    );
  }
  const [engine, nativeFile, out] = args;
  const outputDir = resolve(out);
  const original = await readFile(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
  const changed = mutateHitTargetSource(original);
  const native = JSON.parse(await readFile(nativeFile, "utf8"));
  await mkdir(dirname(outputDir), { recursive: true });
  await mkdir(outputDir); // Require fresh, non-overwritten source evidence.
  const fixturePath = join(outputDir, "pointer-events-mutation.html");
  await writeFile(fixturePath, changed, { flag: "wx", mode: 0o444 });

  // Capture unmodified native browser data from the mutated HTML in a fresh
  // browser instance; never derive target IDs from geometric rectangles.
  const browserTypes = await import("playwright");
  const require = createRequire(import.meta.url);
  const playwrightVersion = require("playwright/package.json").version;
  const capture = await collectOne(engine, {
    browserTypes, playwrightVersion, fixturePath,
  });
  await writeFile(join(outputDir, "mutated-capture.json"),
    JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });

  // Compare the original native Rust receipt against the new browser source.
  // The intentional cross-source difference is the negative control.
  const report = compareNativeBlockBridge(native, capture, changed);
  await writeFile(join(outputDir, "intentional-hit-mutation.json"),
    JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  verifyHitTargetSensitivity(report);
  console.log("PASS native bridge real-engine hit-target sensitivity " +
    JSON.stringify({ engine, sourceSha256: report.sourceSha256,
      differences: report.differences, conformance: report.conformance }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
