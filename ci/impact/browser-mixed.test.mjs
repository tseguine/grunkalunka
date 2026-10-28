import assert from 'node:assert/strict';
import test from 'node:test';
import {
  HISTORICAL_PATHS, OFFLINE_BROWSER_TESTS, browserWorkflowCoverage,
  browserLayoutPlan,
} from './browser-mixed.mjs';

const groups = [{ id: 'test:browser-oracle-contracts', dependsOn: ['browser-oracles'] }];
const modules = [
  { id: 'grunkalunka-layout', paths: ['crates/grunkalunka-layout/'] },
  { id: 'browser-oracles', paths: [
    'tests/browser-oracles/', 'tools/browser-oracles/',
  ] },
  { id: 'grunkalunka-compiler', paths: ['crates/grunkalunka-compiler/'] },
];
const packages = ['grunkalunka-compiler', 'grunkalunka-layout'];
const tests = [
  ...packages.map(id => ({ id: 'cargo-test:' + id, dependsOn: [id] })),
  ...['workspace-format', 'workspace-clippy', 'workspace-tests'].map(id => ({
    id: 'check:' + id,
    dependsOn: [...packages],
    buildDependsOn: [],
  })),
];

function workflow(paths, command) {
  const triggers = paths.map(path => "      - '" + path + "'").join('\n');
  return [
    'name: fixture',
    '',
    'on:',
    '  pull_request:',
    '    branches: [master]',
    '    paths:',
    triggers,
    '  push:',
    '    branches: [master]',
    '    paths:',
    triggers,
    '  workflow_dispatch:',
    '',
    'permissions:',
    '  contents: read',
    '',
    'jobs:',
    '  check:',
    '    steps:',
    '      - run: ' + command,
    '',
  ].join('\n');
}

function workflows() {
  return {
    'browser-oracles.yml': workflow(
      ['tests/browser-oracles/**'], 'node --test tests/browser-oracles/*.test.mjs'),
    'browser-collector.yml': workflow(
      ['tools/browser-oracles/**'], 'node --test tools/browser-oracles/collect.test.mjs'),
    'browser-smoke.yml': workflow(
      ['tests/browser-oracles/**', 'tools/browser-oracles/**'],
      'engine: [chromium, firefox, webkit]\n      - run: node tools/browser-oracles/smoke.mjs $' +
        '{{ matrix.engine }}\n      - image: mcr.microsoft.com/playwright:v1.64.1-noble@sha256:test'),
  };
}

const analyze = changes => browserLayoutPlan({
  modules, groups, tests, packages, workflows: changes ?? workflows(),
});

test('historical browser grid and layout change paths are exact and separate', () => {
  assert.equal(HISTORICAL_PATHS.length, 8);
  assert.equal(new Set(HISTORICAL_PATHS).size, 8);
  assert.ok(HISTORICAL_PATHS.some(x => x.startsWith('tools/browser-oracles/')));
  assert.ok(HISTORICAL_PATHS.some(x => x.startsWith('crates/grunkalunka-layout/')));
  assert.equal(OFFLINE_BROWSER_TESTS.length, 2);
});

test('workflow coverage verifies both PR and push triggers and offline/browser commands', () => {
  assert.deepEqual(browserWorkflowCoverage(workflows()), {
    safeToRehearse: true, reason: null,
  });
  for (const modify of [
    w => { delete w['browser-smoke.yml']; },
    w => { w['browser-smoke.yml'] = w['browser-smoke.yml']
      .replace("      - 'tools/browser-oracles/**'", "      - 'tools/elsewhere/**'"); },
    w => { w['browser-oracles.yml'] = w['browser-oracles.yml']
      .replace('node --test tests/browser-oracles/*.test.mjs', 'node --test stale.mjs'); },
    w => { w['browser-collector.yml'] = w['browser-collector.yml']
      .replace('  workflow_dispatch:', '  no-workflow-dispatch:'); },
    w => { w['browser-smoke.yml'] = w['browser-smoke.yml']
      .replace('engine: [chromium, firefox, webkit]', 'engine: [chromium]'); },
  ]) {
    const x = workflows();
    modify(x);
    assert.equal(browserWorkflowCoverage(x).safeToRehearse, false);
    assert.equal(analyze(x).full, true, 'missing optional workflow cannot be silently ignored');
  }
});

test('real changed-file union selects two offline browser targets and only layout Cargo tests', () => {
  const plan = analyze();
  assert.equal(plan.full, false, plan.reason);
  assert.match(plan.baselineFallback[0], /no executable test covers changed owner: browser-oracles/);
  assert.deepEqual(plan.selectedBuildModules, ['browser-oracles', 'grunkalunka-layout']);
  assert.deepEqual(plan.selectedTests, [
    'cargo-test:grunkalunka-layout', 'check:workspace-clippy',
    'check:workspace-format', 'check:workspace-tests',
    'external:browser-collector-contract', 'external:browser-oracle-contract',
  ]);
  assert.deepEqual(plan.cargoCommands.build.slice(-2), ['-p', 'grunkalunka-layout']);
  assert.deepEqual(plan.cargoCommands.test.slice(-2), ['-p', 'grunkalunka-layout']);
  assert.ok(!plan.selectedTests.includes('cargo-test:grunkalunka-compiler'));
  assert.deepEqual(plan.offlineBrowserCommands[0],
    ['node', '--test', 'tests/browser-oracles/*.test.mjs']);
  assert.equal(plan.preservedNotExecuted.length, 1, 'browser engine smoke is not implied by offline tests');
});

test('accidental new layout dependency or test changes invalidate the strict allowlist', () => {
  const invalidModules = modules.map(x => x.id === 'grunkalunka-layout'
    ? { ...x, dependsOn: ['grunkalunka-compiler'] } : x);
  assert.throws(() => browserLayoutPlan({
    modules: invalidModules, groups, tests, packages, workflows: workflows(),
  }));
  const newCheck = [...tests, { id: 'check:new-browser-check', dependsOn: ['browser-oracles'] }];
  assert.throws(() => browserLayoutPlan({
    modules, groups, tests: newCheck, packages, workflows: workflows(),
  }));
});
