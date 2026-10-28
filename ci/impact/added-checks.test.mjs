import assert from 'node:assert/strict';
import test from 'node:test';
import { scopedAddedChecks } from './added-checks.mjs';
import { planImpact } from './impact.mjs';

const manifest = 'scripts/checks/003500-browser-regression.json';
const descriptor = 'ci/impact/checks/browser-regression.json';
const source = 'tests/browser-oracles/regression.test.mjs';
const command = { command: 'node', args: ['--test', 'tests/browser-oracles/regression.test.mjs'] };
const check = { id: 'check:browser-regression', ...command,
  dependsOn: ['test:browser-regressions'] };
const registration = [manifest, descriptor];
const inputs = ({ changedPaths = registration, addedPaths = registration,
  declared = [check], observed = [command] } = {}) =>
  ({ changedPaths, addedPaths, declared, observed });

test('new paired command and impact descriptor select browser tests without workspace Cargo', () => {
  const scopedPaths = scopedAddedChecks(inputs());
  assert.deepEqual(scopedPaths, {
    [manifest]: check.id,
    [descriptor]: check.id,
  });
  const result = planImpact({
    changedPaths: [...registration, source],
    modules: [
      { id: 'browser-oracles', paths: ['tests/browser-oracles/'] },
      { id: 'unrelated-cargo', paths: ['crates/unrelated/'] },
    ],
    groups: [{ id: 'test:browser-regressions', dependsOn: ['browser-oracles'] }],
    tests: [check],
    global: ['ci/impact/', 'scripts/checks/'],
    scopedPaths,
  });
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.deepEqual(result.tests, ['check:browser-regression']);
  assert.deepEqual(result.build, ['browser-oracles']);
  assert.ok(!result.build.includes('unrelated-cargo'));
});

test('new check registrations select their own tests even without changing source', () => {
  const result = planImpact({
    changedPaths: registration,
    modules: [{ id: 'p1-tests', paths: ['tools/bench/tests/'] }],
    groups: [{ id: 'test:browser-regressions', dependsOn: ['p1-tests'] }],
    tests: [check],
    global: ['ci/impact/', 'scripts/checks/'],
    scopedPaths: scopedAddedChecks(inputs()),
  });
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.deepEqual(result.tests, ['check:browser-regression']);
  assert.deepEqual(result.build, ['p1-tests']);
});

test('ordinary new tests in existing suites need no registration changes or global fallback', () => {
  assert.deepEqual(scopedAddedChecks(inputs({
    changedPaths: [source], addedPaths: [source],
  })), {});
  const result = planImpact({
    changedPaths: [source],
    modules: [{ id: 'browser-oracles', paths: ['tests/browser-oracles/'] }],
    tests: [{ id: 'check:browser', dependsOn: ['browser-oracles'] }],
    global: ['ci/impact/', 'scripts/checks/'],
  });
  assert.equal(result.full, false);
  assert.deepEqual(result.tests, ['check:browser']);
});

test('unpaired, edited, deleted and unexpected registration paths fail closed', () => {
  for (const args of [
    { changedPaths: [manifest], addedPaths: [manifest] },
    { changedPaths: [descriptor], addedPaths: [descriptor] },
    { changedPaths: registration, addedPaths: [manifest] },
    { changedPaths: registration, addedPaths: [descriptor] },
    { changedPaths: registration, addedPaths: [] },
    { changedPaths: [...registration, 'scripts/checks/README.md'] },
    { changedPaths: [...registration, 'ci/impact/checks/other/more.json'] },
  ]) {
    assert.throws(() => scopedAddedChecks(inputs(args)));
  }
});

test('stale or unsafe new descriptors cannot obtain a scoped registration exemption', () => {
  for (const args of [
    { declared: [] },
    { declared: [{ ...check, id: 'check:wrong' }] },
    { declared: [{ ...check, dependsOn: [], paths: [] }] },
    { declared: [{ ...check, scope: 'workspace' }] },
    { observed: [] },
    { observed: [{ command: 'node', args: ['unrelated.mjs'] }] },
  ]) assert.throws(() => scopedAddedChecks(inputs(args)));
});

test('global CI and registration modifications still require full verification', () => {
  const scopedPaths = scopedAddedChecks(inputs());
  const tests = [check];
  const kwargs = {
    modules: [], groups: [{ id: 'test:browser-regressions' }],
    tests, global: ['ci/impact/', 'scripts/checks/', '.github/'], scopedPaths,
  };
  const changed = [...registration, '.github/workflows/ci.yml'];
  const plan = planImpact({ ...kwargs, changedPaths: changed });
  assert.equal(plan.full, true);
  assert.match(plan.reasons.join(' '), /global build input/);
  for (const badScope of [
    { [manifest]: 'check:missing' },
    { [manifest]: 'test:browser-regressions' },
    { ['scripts/checks/777777-unrelated.json']: check.id },
  ]) {
    const unsafe = planImpact({ ...kwargs, changedPaths: registration,
      scopedPaths: badScope });
    assert.equal(unsafe.full, true);
  }
});
