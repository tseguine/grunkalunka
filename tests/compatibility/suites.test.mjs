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
  assert.deepEqual(ts.adapted, { selected: 32, executed: 32, passed: 32 });
  assert.deepEqual(ts.admission, { selected: 0, executed: 0, passed: 0 });
  assert.deepEqual(ts.upstream, { executed: 0, outcome: 'not-executed' });
});

function completedSelection() {
  return suiteInventory().flatMap(suite => suite.selection.map(item => ({
    id: item.id, expectation: item.kind === 'adapted-runtime' ? 'admitted' : 'rejected',
    executed: true, outcome: 'passed',
  })));
}

test('Mandelbrot fulfilled numeric prerequisites do not imply full output or upstream execution', () => {
  const suite = suiteInventory().find(item => item.id === 'benchmarks-game');
  assert.match(suite.fulfilledPrerequisites.join(' '), /for loops.*bitwise\/shift/);
  assert.match(suite.blockers.join(' '), /Typed arrays.*process\/stdout/);
  assert.doesNotMatch(suite.blockers.join(' '), /requires for loops|requires.*bitwise operators/);
  assert.deepEqual(suite.upstream, { executed: 0, outcome: 'not-executed' });
});

test('failed observations are not counted as passing', () => {
  const fixtures = completedSelection();
  fixtures.find(item => item.id === 'admitted:ts-ra1').outcome = 'failed';
  const ts = suiteInventory(fixtures).find(suite => suite.id === 'typescript');
  assert.equal(ts.adapted.executed, 32);
  assert.equal(ts.adapted.passed, 31);
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
