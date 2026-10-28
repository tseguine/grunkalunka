import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

// A narrow source-layout contract, not a general YAML parser. The runtime
// assertions in both jobs independently check what setup-node actually selected.
test("browser producers and report job use one exact Node version", async () => {
  const workflow = (await readFile(new URL("../../.github/workflows/browser-smoke.yml", import.meta.url), "utf8")).replace(/\r\n/g, "\n");
  const header = workflow.split("\njobs:\n", 1)[0];
  assert.match(header, /^env:\n(?:  #[^\n]*\n)*  BROWSER_NODE_VERSION: '\d+\.\d+\.\d+'$/m,
    "declare an exact patch version at workflow scope, not a floating range");
  assert.equal([...workflow.matchAll(/^\s*BROWSER_NODE_VERSION:/gm)].length, 1,
    "job/step overrides must not diverge from the shared version");
  assert.doesNotMatch(workflow, /^\s*node-version-file:/m);
  const selectors = [...workflow.matchAll(/^\s*node-version:\s*(.+)$/gm)].map(match => match[1]);
  assert.deepEqual(selectors, Array(2).fill("${{ env.BROWSER_NODE_VERSION }}"));
  const jobs = workflow.split(/^  (?:smoke|cross-engine):\n/m).slice(1);
  assert.equal(jobs.length, 2);
  for (const job of jobs) {
    assert.ok(job.includes("uses: actions/setup-node@"));
  }
  for (const file of ["smoke.test.mjs", "cross-engine.test.mjs"]) {
    const source = await readFile(new URL("../../tools/browser-oracles/hosted/" + file,
      import.meta.url), "utf8");
    assert.ok(source.includes("process.versions.node"));
    assert.ok(source.includes("process.env.BROWSER_NODE_VERSION"));
  }
});
