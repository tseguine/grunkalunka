import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  EXPECTED_HIT_DIFFERENCES, mutateNativeBridgeHitSource, verifyNativeHitSensitivity,
} from "./native-hit-sensitivity.mjs";

const fixture = readFileSync(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
const report = () => ({
  status: "different", conformance: "not-assessed",
  invalid: [], incompatible: [], differences: structuredClone(EXPECTED_HIT_DIFFERENCES),
});

test("changes one isolated pointer-events rule while preserving all other fixture bytes", () => {
  const before = Buffer.from(fixture);
  const changed = mutateNativeBridgeHitSource(before);
  assert.notDeepEqual(changed, before);
  assert.deepEqual(fixture, before, "original fixture remains byte-for-byte unchanged");
  assert.equal(changed.toString("utf8").split("#first { pointer-events: none; width: 100px;").length, 2);
  assert.equal(changed.toString("utf8").replace(
    "#first { pointer-events: none; width: 100px;", "#first { width: 100px;"),
  before.toString("utf8"));
});

test("fixture drift, ambiguous mutation sites and nonbytes fail closed", () => {
  assert.throws(() => mutateNativeBridgeHitSource("text"), TypeError);
  const shifted = Buffer.from(fixture.toString("utf8").replace("#first { width: 100px;",
    "#first { width: 101px;"));
  assert.throws(() => mutateNativeBridgeHitSource(shifted), /exactly one/);
  assert.throws(() => mutateNativeBridgeHitSource(Buffer.from(
    fixture.toString("utf8") + "\n#first { width: 100px;")), /exactly one/);
});

test("exact six hit differences are required; geometry and source errors fail", () => {
  assert.equal(EXPECTED_HIT_DIFFERENCES.length, 6);
  assert.doesNotThrow(() => verifyNativeHitSensitivity(report()));
  for (const change of [
    r => { r.status = "equal"; },
    r => { r.status = "incompatible"; },
    r => { r.conformance = "passed"; },
    r => { r.invalid.push({ path: "$.hitTargets", message: "missing" }); },
    r => { r.incompatible.push({ path: "$.source", message: "mismatch" }); },
    r => { r.differences.pop(); },
    r => { r.differences[0].right = "first"; },
    r => { r.differences[0].probe = "second-original"; },
    r => { r.differences.push({ kind: "geometry", path: "$.geometry", left: 1, right: 2 }); },
  ]) {
    const candidate = report();
    change(candidate);
    assert.throws(() => verifyNativeHitSensitivity(candidate));
  }
});
