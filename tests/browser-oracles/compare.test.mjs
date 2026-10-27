import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { compareCaptures, validateCapture } from "./compare.mjs";

function capture() {
  return {
    schemaVersion: 1,
    fixtureId: "layout-baseline-v1",
    environment: {
      userAgent: "TestEngine/1",
      viewportWidth: 800,
      viewportHeight: 600,
      devicePixelRatio: 1,
      url: "file:///layout-baseline.html",
      timestamp: "2026-10-27T09:00:00.000Z"
    },
    observations: [
      {
        id: "frame",
        text: "abc",
        computed: { width: "240px", display: "block", lineHeight: "normal" },
        rect: { x: 0, y: 0, width: 240, height: 42 }
      },
      {
        id: "block",
        text: "abc",
        computed: { width: "100px", display: "block", lineHeight: "normal" },
        rect: { x: 15, y: 15, width: 114, height: 34 }
      }
    ]
  };
}

test("validates capture without deriving conformance from observations", () => {
  assert.deepEqual(validateCapture(capture()), []);
  const same = capture();
  same.environment.userAgent = "Different/42";
  same.environment.timestamp = "2026-10-27T10:00:00Z";
  same.environment.url = "https://different.example/fixture";
  const report = compareCaptures(capture(), same);
  assert.equal(report.status, "equal");
  assert.deepEqual(report.differences, []);
  assert.equal(report.captures.right.environment.userAgent, "Different/42");
});

test("geometry uses an explicit CSS pixel tolerance", () => {
  const expected = capture();
  const actual = capture();
  actual.observations[0].rect.width += 0.125;
  assert.equal(compareCaptures(expected, actual).status, "different");
  assert.equal(compareCaptures(expected, actual, { rectTolerance: 0.125 }).status, "equal");
  const report = compareCaptures(expected, actual, { rectTolerance: 0.01 });
  assert.equal(report.differences[0].kind, "geometry");
  assert.equal(report.differences[0].tolerance, 0.01);
  assert.equal(report.differences[0].path, "$.observations[0].rect.width");
});

test("computed styles compare strings except explicitly tolerated CSS pixel lengths", () => {
  const expected = capture();
  const actual = capture();
  actual.observations[0].computed.width = "240.05px";
  actual.observations[0].computed.display = "inline";
  assert.equal(compareCaptures(expected, actual, { stylePxTolerance: 0.1 }).status, "different");
  const report = compareCaptures(expected, actual, { stylePxTolerance: 0.01 });
  assert.equal(report.differences.length, 2);
  assert.deepEqual(report.differences.map(d => d.kind), ["computed-style", "computed-style"]);
  actual.observations[0].computed.display = "block";
  assert.equal(compareCaptures(expected, actual, { stylePxTolerance: 0.1 }).status, "equal");
});

test("a missing computed property is not a zero or empty computed value", () => {
  const expected = capture();
  const actual = capture();
  delete actual.observations[0].computed.width;
  actual.observations[0].computed.lineHeight = "";
  const report = compareCaptures(expected, actual);
  assert.equal(report.status, "different");
  const widths = report.differences.filter(d => d.path.endsWith(".width"));
  assert.equal(widths.length, 1);
  assert.equal(widths[0].kind, "missing-property");
  assert.equal(widths[0].right, null);
  assert.ok(report.differences.some(d => d.right === ""));
});

test("nonmatching fixtures and environments are incompatible, not comparable", () => {
  const expected = capture();
  const actual = capture();
  actual.fixtureId = "another-fixture";
  actual.environment.viewportWidth = 801;
  actual.environment.devicePixelRatio = 2;
  const report = compareCaptures(expected, actual);
  assert.equal(report.status, "incompatible");
  assert.equal(report.incompatibilities.length, 3);
  assert.deepEqual(report.differences, []);
});

test("IDs and observation ordering are part of the contract", () => {
  const expected = capture();
  const actual = capture();
  actual.observations.reverse();
  assert.equal(compareCaptures(expected, actual).status, "incompatible");
  actual.observations = [expected.observations[0], expected.observations[0]];
  const report = compareCaptures(expected, actual);
  assert.equal(report.status, "invalid");
  assert.ok(report.rightErrors.some(error => error.message.includes("duplicate")));
});

test("invalid JSON capture shapes produce path-bearing diagnostics", () => {
  const broken = capture();
  broken.observations[0].rect.width = Number.NaN;
  broken.observations[1].computed.display = 5;
  broken.environment.viewportHeight = 0;
  const report = compareCaptures(capture(), broken);
  assert.equal(report.status, "invalid");
  assert.ok(report.rightErrors.some(error => error.path.endsWith(".rect.width")));
  assert.ok(report.rightErrors.some(error => error.path.endsWith(".computed.display")));
  assert.ok(report.rightErrors.some(error => error.path.endsWith(".viewportHeight")));
  assert.ok(validateCapture(null).some(error => error.path === "$"));
});

test("rejects invalid tolerances instead of silently widening comparisons", () => {
  for (const option of [
    { rectTolerance: -1 },
    { rectTolerance: Number.NaN },
    { stylePxTolerance: "0.5" },
    { stylePxTolerance: Infinity }
  ]) {
    assert.throws(() => compareCaptures(capture(), capture(), option), TypeError);
  }
});

test("CLI exit codes and reports distinguish equal, different and incompatible", () => {
  const dir = mkdtempSync(join(tmpdir(), "grunkalunka-oracle-"));
  const cli = fileURLToPath(new URL("./compare.mjs", import.meta.url));
  const left = join(dir, "left.json");
  const right = join(dir, "right.json");
  try {
    writeFileSync(left, JSON.stringify(capture()));
    writeFileSync(right, JSON.stringify(capture()));
    const run = (...opts) => spawnSync(process.execPath, [cli, left, right, ...opts], { encoding: "utf8" });
    let result = run();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stdout).status, "equal");

    const modified = capture();
    modified.observations[0].rect.x += 1;
    writeFileSync(right, JSON.stringify(modified));
    result = run();
    assert.equal(result.status, 1, result.stderr);
    assert.equal(JSON.parse(result.stdout).status, "different");

    result = run("--rect-tolerance=1");
    assert.equal(result.status, 0, result.stderr);

    modified.environment.viewportWidth += 1;
    writeFileSync(right, JSON.stringify(modified));
    result = run();
    assert.equal(result.status, 2, result.stderr);
    assert.equal(JSON.parse(result.stdout).status, "incompatible");

    result = run("--rect-tolerance=bad");
    assert.equal(result.status, 2);
    assert.match(result.stderr, /invalid tolerance/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
