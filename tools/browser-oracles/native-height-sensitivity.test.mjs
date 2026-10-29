import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  EXPECTED_HEIGHT_DIFFERENCES, mutateNativeHeightSource,
  verifyNativeHeightSensitivity,
} from "./native-height-sensitivity.mjs";

const fixture = readFileSync(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));

const report = () => ({
  status: "different", conformance: "not-assessed",
  invalid: [], incompatible: [],
  differences: structuredClone(EXPECTED_HEIGHT_DIFFERENCES),
});

test("one reviewed percentage-height edit preserves all original fixture bytes", () => {
  const before = Buffer.from(fixture);
  const after = mutateNativeHeightSource(before);
  const original = "#height-first { width: 30px; height: 50%;";
  const mutated = "#height-first { width: 30px; height: 60%;";
  assert.notDeepEqual(after, before);
  assert.equal(Buffer.compare(before, fixture), 0, "source buffer must be immutable");
  assert.equal(after.toString("utf8").split(mutated).length, 2);
  assert.equal(after.toString("utf8").replace(mutated, original), before.toString("utf8"));
  // The original first/second blocks, browser hit probes and the native sample
  // remain unchanged; this control must not mutate another geometry channel.
  assert.ok(after.includes(Buffer.from("#first { width: 100px;")));
  assert.ok(after.includes(Buffer.from("#height-second { width: 20px; height: 25%;")));
});

test("source mutation rejects missing, duplicate or nonbinary CSS inputs", () => {
  const original = "#height-first { width: 30px; height: 50%;";
  const alreadyChanged = Buffer.from(fixture.toString("utf8").replace(original,
    "#height-first { width: 30px; height: 60%;"));
  assert.throws(() => mutateNativeHeightSource(alreadyChanged), /exactly one/);
  assert.throws(() => mutateNativeHeightSource(Buffer.from(
    fixture.toString("utf8") + "\n" + original)), /exactly one/);
  for (const bad of [null, undefined, "not binary", { bytes: [] }]) {
    assert.throws(() => mutateNativeHeightSource(bad), TypeError);
  }
});

test("height sensitivity requires exactly two source-driven differences", () => {
  assert.doesNotThrow(() => verifyNativeHeightSensitivity(report()));
  assert.deepEqual(EXPECTED_HEIGHT_DIFFERENCES.map(d => [d.id, d.field, d.left, d.right]), [
    ["height-first", "height", 46, 54],
    ["height-second", "y", 156, 164],
  ]);
  for (const mutate of [
    r => { r.status = "equal"; },
    r => { r.status = "incompatible"; },
    r => { r.conformance = "passed"; },
    r => { r.invalid.push({ path: "$", message: "missing height" }); },
    r => { r.incompatible.push({ path: "$.provenance", message: "stale source hash" }); },
    r => { r.differences.pop(); },
    r => { r.differences[0].right = 46; },
    r => { r.differences[1].right = 163; },
    r => { r.differences[0].kind = "geometry"; },
    r => { r.differences[1].field = "height"; },
    r => { r.differences.reverse(); },
    r => { r.differences.push({ path: "$.hitTargets.original.first-right" }); },
  ]) {
    const invalid = report();
    mutate(invalid);
    assert.throws(() => verifyNativeHeightSensitivity(invalid));
  }
});
