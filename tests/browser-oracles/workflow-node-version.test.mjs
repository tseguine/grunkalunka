import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

// A narrow source-layout contract, not a general YAML parser. The runtime
// assertions in both jobs independently check what setup-node actually selected.
test("browser producers and report job use one exact Node version", async () => {
  const workflow = await readFile(new URL("../../.github/workflows/browser-smoke.yml", import.meta.url), "utf8");
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
  const check = 'node -e \'require("node:assert/strict").equal(process.versions.node, process.env.BROWSER_NODE_VERSION)\'';
  for (const job of jobs) {
    assert.match(job, /uses: actions\/setup-node@/);
    assert.ok(job.includes(`run: ${check}`), "verify the selected version before producing/consuming evidence");
  }
});
