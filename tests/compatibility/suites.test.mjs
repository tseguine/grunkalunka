import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { suiteInventory } from './suites.mjs';

test('all six registered suites remain unexecuted without result evidence', () => {
  const suites = suiteInventory();
  assert.deepEqual(suites.map(suite => suite.id),
    ['test262', 'typescript', 'benchmarks-game', 'jetstream', 'node', 'speedometer']);
  for (const suite of suites) {
    assert.equal(suite.upstream.outcome, 'not-executed');
    assert.equal(suite.upstream.executed, 0);
    assert.equal(suite.adapted.executed, 0);
    assert.equal(suite.admission.executed, 0);
  }
});

test('adapted executions and admission rejections are counted separately', () => {
  const fixtures = completedSelection();
  const ts = suiteInventory(fixtures).find(suite => suite.id === 'typescript');
  assert.deepEqual(ts.adapted, { selected: 24, executed: 24, passed: 24 });
  assert.deepEqual(ts.admission, { selected: 8, executed: 8, passed: 8 });
  assert.deepEqual(ts.upstream, { executed: 0, outcome: 'not-executed' });
});

function completedSelection() {
  return ['a', 'b', 'c', 'd'].flatMap(operator => Array.from({ length: 8 }, (_, index) => {
    const expectation = operator === 'c' ? 'rejected' : 'admitted';
    return { id: `${expectation}:ts-r${operator}${index + 1}`, expectation, executed: true, outcome: 'passed' };
  }));
}

test('failed observations are not counted as passing', () => {
  const fixtures = completedSelection();
  fixtures[0].outcome = 'failed';
  const ts = suiteInventory(fixtures).find(suite => suite.id === 'typescript');
  assert.equal(ts.adapted.executed, 24);
  assert.equal(ts.adapted.passed, 23);
});

test('incomplete, duplicate and incorrectly classified evidence is rejected', () => {
  const missing = completedSelection().slice(1);
  assert.throws(() => suiteInventory(missing), /missing result/);
  const duplicate = completedSelection();
  duplicate.push(duplicate[0]);
  assert.throws(() => suiteInventory(duplicate), /duplicate result/);
  const pending = completedSelection();
  pending[0].executed = false;
  assert.throws(() => suiteInventory(pending), /unexecuted result/);
  const misclassified = completedSelection();
  misclassified[0].expectation = 'rejected';
  assert.throws(() => suiteInventory(misclassified), /wrong expectation/);
  const invalid = completedSelection();
  invalid[0].outcome = 'not-executed';
  assert.throws(() => suiteInventory(invalid), /invalid outcome/);
});

test('the command rejects a file without execution results', () => {
  const result = spawnSync(process.execPath, [fileURLToPath(new URL('./suites.mjs', import.meta.url)),
    fileURLToPath(new URL('./suites.json', import.meta.url))], { encoding: 'utf8', timeout: 10_000 });
  assert.equal(result.error, undefined);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /report must contain fixture results/);
});
