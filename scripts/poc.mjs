// One-command replay: build the workspace, then compare its emitted Wasm.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { programs, rejected } from '../tests/compatibility/corpus.mjs';
import { watchdogBudget, corpusWatchdogCounts } from '../tests/compatibility/watchdog-budget.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const { outerMs } = watchdogBudget(corpusWatchdogCounts(programs, rejected));
function run(command, args, timeout) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false, timeout });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, `${command} failed`);
}
run(process.execPath, ['scripts/licenses.mjs'], 60_000);
run('cargo', ['build', '--workspace', '--locked', '--offline', '-j', '4'], 600_000);
run(process.execPath, ['tests/compatibility/check.mjs'], outerMs);
run(process.execPath, ['tests/compatibility/primitive-values-check.mjs'], 90_000);
