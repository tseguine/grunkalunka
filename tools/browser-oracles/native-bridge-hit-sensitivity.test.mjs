import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  mutateHitTargetSource, verifyHitTargetSensitivity,
} from "./native-bridge-hit-sensitivity.mjs";

const fixture = readFileSync(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));
const affected = ["first-center", "first-lower", "first-right"];
const differences = ["original", "restored"].flatMap(phase => affected.map(probe => ({
  kind: "hit-target", phase, probe,
  path: "$.hitTargets." + phase + "." + probe, left: "first", right: "outer",
})));
const report = () => ({
  status: "different", conformance: "not-assessed",
  invalid: [], incompatible: [], differences: structuredClone(differences),
});

test("injects one pointer-events rule without modifying original source bytes", () => {
  const before = Buffer.from(fixture);
  const after = mutateHitTargetSource(before);
  assert.notDeepEqual(after, before);
  assert.deepEqual(before, fixture, "preserve the original source");
  assert.equal(after.toString("utf8").split("#first { pointer-events: none; width: 100px;").length, 2);
  assert.equal(after.toString("utf8").replace(
    "#first { pointer-events: none; width: 100px;", "#first { width: 100px;"),
    before.toString("utf8"), "no other source edits");
});

test("mutation fails closed on renamed, repeated, or invalid input", () => {
  const source = fixture.toString("utf8");
  assert.throws(() => mutateHitTargetSource(Buffer.from(source.replace(
    "#first { width: 100px;", "#first { width: 102px;"))), /exactly one/);
  assert.throws(() => mutateHitTargetSource(Buffer.from(
    source + "\n#first { width: 100px;")), /exactly one/);
  assert.throws(() => mutateHitTargetSource("not binary"), TypeError);
});

test("requires exactly the six live original and restored target disagreements", () => {
  assert.doesNotThrow(() => verifyHitTargetSensitivity(report()));
  for (const mutate of [
    r => { r.status = "equal"; },
    r => { r.status = "incompatible"; },
    r => { r.invalid.push({ path: "$", message: "missing capture" }); },
    r => { r.incompatible.push({ path: "$", message: "source mismatch" }); },
    r => { r.conformance = "passed"; },
    r => { r.differences.splice(2, 1); },
    r => { r.differences[0].right = "first"; },
    r => { r.differences[1].kind = "geometry"; },
    r => { r.differences[2].phase = "hidden"; },
    r => { r.differences[3].probe = "second-original"; },
    r => { r.differences[4].path = "$.hitTargets.restored.left-margin"; },
    r => { r.differences.push({
      kind: "hit-target", phase: "hidden", probe: "first-center",
      path: "$.hitTargets.hidden.first-center", left: "second", right: "outer",
    }); },
  ]) {
    const candidate = report();
    mutate(candidate);
    assert.throws(() => verifyHitTargetSensitivity(candidate));
  }
});
