// Real-engine negative control for the hand-resolved native block bridge.
// Deliberately change one CSS declaration in an independently captured HTML
// fixture, while retaining the original native Rust geometry as the reference.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { collectOne } from "./collect.mjs";
import { compareNativeBlockBridge } from "../../tests/browser-oracles/native-block-bridge.mjs";

const WIDTH_RULE = "#first { width: 100px;";
const CHANGED_RULE = "#first { width: 101px;";
const EXPECTED_DIFFERENCES = Object.freeze([
  { path: "$.observations[1].rect.width", left: 118, right: 119 },
  { path: "$.reflow.restored.first.width", left: 118, right: 119 },
]);

export function mutateNativeBridgeSource(originalBytes) {
  if (!(originalBytes instanceof Uint8Array)) {
    throw new TypeError("original fixture bytes are required");
  }
  const text = Buffer.from(originalBytes).toString("utf8");
  assert.equal(text.split(WIDTH_RULE).length, 2,
    "exactly one independently reviewed width mutation site required");
  return Buffer.from(text.replace(WIDTH_RULE, CHANGED_RULE), "utf8");
}

export function verifyNativeBridgeSensitivity(report) {
  assert.equal(report?.status, "different", "real CSS mutation must produce differences");
  assert.equal(report.conformance, "not-assessed");
  assert.deepEqual(report.invalid, [], "mutated capture must remain schema-valid");
  assert.deepEqual(report.incompatible, [], "mutated capture must match its own source hash");
  assert.deepEqual(report.differences?.map(d => ({
    path: d.path, left: d.left, right: d.right,
  })), EXPECTED_DIFFERENCES,
  "the width mutation must change exactly the original and restored child border widths");
}

async function main(args) {
  if (args.length !== 3 || !["chromium", "firefox", "webkit"].includes(args[0])) {
    throw new Error("usage: node tools/browser-oracles/native-bridge-sensitivity.mjs ENGINE NATIVE_JSON NEW_OUTPUT_DIR");
  }
  const [engine, nativeFile, out] = args;
  const outputDir = resolve(out);
  const fixture = await readFile(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
  const changed = mutateNativeBridgeSource(fixture);
  const native = JSON.parse(await readFile(nativeFile, "utf8"));
  await mkdir(dirname(outputDir), { recursive: true });
  await mkdir(outputDir); // Never reuse or overwrite earlier evidence.
  const path = join(outputDir, "mutation.html");
  await writeFile(path, changed, { flag: "wx", mode: 0o444 });

  // Do not synthesize the capture or repair its provenance; use a fresh
  // Playwright engine instance reading the mutated local HTML as-is.
  const browserTypes = await import("playwright");
  const require = createRequire(import.meta.url);
  const playwrightVersion = require("playwright/package.json").version;
  const capture = await collectOne(engine, {
    browserTypes, playwrightVersion, fixturePath: path,
  });
  await writeFile(join(outputDir, "mutated-capture.json"),
    JSON.stringify(capture, null, 2) + "\n", { flag: "wx" });

  // The capture is source-checked against its *mutated* source bytes. The
  // native model deliberately still describes the original tree, so a strict
  // geometry mismatch proves the comparator responds to an actual edit.
  const report = compareNativeBlockBridge(native, capture, changed);
  await writeFile(join(outputDir, "intentional-source-mutation.json"),
    JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  verifyNativeBridgeSensitivity(report);
  console.log("PASS native bridge real-engine mutation sensitivity " +
    JSON.stringify({ engine, sourceSha256: report.sourceSha256,
      differences: report.differences, conformance: report.conformance }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write(String(error) + "\n");
    process.exitCode = 2;
  });
}
