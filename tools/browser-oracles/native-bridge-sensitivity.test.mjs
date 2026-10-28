import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  mutateNativeBridgeSource, verifyNativeBridgeSensitivity,
} from "./native-bridge-sensitivity.mjs";

const fixture = readFileSync(new URL("../../tests/browser-oracles/native-block-bridge.html", import.meta.url));

const differences = [
  { kind: "geometry", path: "$.observations[1].rect.width", left: 118, right: 119 },
  { kind: "geometry", phase: "restored", id: "first", field: "width",
    path: "$.reflow.restored.first.width", left: 118, right: 119 },
];
const report = () => ({
  status: "different", conformance: "not-assessed",
  invalid: [], incompatible: [],
  differences: structuredClone(differences),
});

test("mutates exactly one CSS declaration without changing the original fixture", () => {
  const before = Buffer.from(fixture);
  const after = mutateNativeBridgeSource(before);
  assert.notDeepEqual(after, before);
  assert.equal(Buffer.compare(before, fixture), 0, "do not modify original bytes");
  assert.equal(after.toString("utf8").split("#first { width: 101px;").length, 2);
  assert.equal(after.toString("utf8").replace("#first { width: 101px;",
    "#first { width: 100px;"), before.toString("utf8"));
});

test("source mutation fails closed if the fixture's reviewed width rule changes", () => {
  const modified = Buffer.from(fixture.toString("utf8").replace(
    "#first { width: 100px;", "#first { width: 101px;"));
  assert.throws(() => mutateNativeBridgeSource(modified), /exactly one/);
  assert.throws(() => mutateNativeBridgeSource(Buffer.from(
    fixture.toString("utf8") + "\n#first { width: 100px;")), /exactly one/);
  assert.throws(() => mutateNativeBridgeSource("not binary"), TypeError);
});

test("strict sensitivity requires both independent border-width discrepancies", () => {
  assert.doesNotThrow(() => verifyNativeBridgeSensitivity(report()));
  for (const mutate of [
    r => { r.status = "equal"; },
    r => { r.status = "incompatible"; },
    r => { r.invalid.push({ path: "$", message: "missing" }); },
    r => { r.incompatible.push({ path: "$", message: "source mismatch" }); },
    r => { r.conformance = "passed"; },
    r => { r.differences.splice(1, 1); },
    r => { r.differences[0].right = 118; },
    r => { r.differences[1].path = "$.reflow.hidden.first.width"; },
    r => { r.differences.push({ path: "$.other", left: 0, right: 1 }); },
  ]) {
    const candidate = report();
    mutate(candidate);
    assert.throws(() => verifyNativeBridgeSensitivity(candidate));
  }
});
