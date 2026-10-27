import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { compareCaptures } from "./compare.mjs";

function capture(width = "0px") {
  return {
    schemaVersion: 1, fixtureId: "comparison-regression",
    environment: {
      userAgent: "Test/1", url: "file:///fixture.html",
      timestamp: "2026-10-27T09:00:00Z",
      viewportWidth: 800, viewportHeight: 600, devicePixelRatio: 1
    },
    observations: [{
      id: "box", text: "", computed: { width },
      rect: { x: 0, y: 0, width: 0, height: 0 }
    }]
  };
}

test("default and explicit zero preserve exact CSS strings and raw inputs", () => {
  for (const width of ["0.0px", "-0px", "1e-999px"]) {
    const left = capture(), right = capture(width);
    for (const options of [{}, { stylePxTolerance: 0 }]) {
      const report = compareCaptures(left, right, options);
      assert.equal(report.status, "different", width);
      assert.equal(report.differences[0].right, width);
      assert.strictEqual(report.captures.left, left);
      assert.strictEqual(report.captures.right, right);
    }
  }
});

test("positive tolerance explicitly permits numeric pixel comparison", () => {
  for (const width of ["0.0px", "0e-999px", "0.125px"]) {
    assert.equal(compareCaptures(capture(), capture(width), { stylePxTolerance: 0.125 }).status, "equal");
  }
  assert.equal(compareCaptures(capture(), capture("0.25px"), { stylePxTolerance: 0.125 }).status, "different");
  assert.equal(compareCaptures(capture(), capture("0em"), { stylePxTolerance: 1 }).status, "different");
});

test("overflow and nonzero underflow do not become numeric pixel agreement", () => {
  for (const width of ["1e-999px", "-1e-999px", "1e999px"]) {
    assert.equal(compareCaptures(capture(), capture(width), { stylePxTolerance: 1 }).status, "different", width);
  }
});

test("CLI consumes the whole tolerance and rejects malformed or lossy values", () => {
  const dir = mkdtempSync(join(tmpdir(), "grunkalunka-oracle-cli-"));
  const cli = fileURLToPath(new URL("./compare.mjs", import.meta.url));
  const left = join(dir, "left.json"), right = join(dir, "right.json");
  try {
    writeFileSync(left, JSON.stringify(capture()));
    writeFileSync(right, JSON.stringify(capture()));
    const run = (...args) => spawnSync(process.execPath, [cli, left, right, ...args], {
      encoding: "utf8", timeout: 5000, maxBuffer: 1024 * 1024
    });
    for (const flag of ["--rect-tolerance", "--style-px-tolerance"]) {
      for (const raw of ["0=garbage", "", " ", " 1", "1 ", "0x10", "-1", "NaN", "Infinity", "1e999", "1e-999"]) {
        const result = run(`${flag}=${raw}`);
        assert.ifError(result.error);
        assert.equal(result.status, 2, `${flag}=${JSON.stringify(raw)}`);
        assert.equal(result.stdout, "");
        assert.match(result.stderr, /invalid tolerance/);
      }
      const duplicate = run(`${flag}=0`, `${flag}=0`);
      assert.equal(duplicate.status, 2);
      assert.match(duplicate.stderr, /duplicate tolerance/);
      for (const raw of ["0", "0.125", ".125", "1e-2", "0e-999"]) {
        const result = run(`${flag}=${raw}`);
        assert.ifError(result.error);
        assert.equal(result.status, 0, result.stderr);
        assert.equal(JSON.parse(result.stdout).status, "equal");
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
