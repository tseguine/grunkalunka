// Bounded mixed browser/layout CI rehearsal. Not a required partial-PR gate.
import assert from 'node:assert/strict';
import { previewCargoScope } from './cargo-scope.mjs';
import { planImpact } from './impact.mjs';

// Exact union of actual changed paths from merged browser PRs #55 and #75.
export const HISTORICAL_PATHS = Object.freeze([
  'docs/browser-oracle-grid-placement.md',
  'tests/browser-oracles/grid-placement.html',
  'tests/browser-oracles/grid-placement.test.mjs',
  'tools/browser-oracles/grid-placement.mjs',
  'tools/browser-oracles/smoke.mjs',
  'crates/grunkalunka-layout/examples/click_reflow_ppm.rs',
  'crates/grunkalunka-layout/tests/headless_input_reflow.rs',
  'docs/headless-hit-test.md',
]);

// Executable offline checks from existing optional workflows. This extension
// is *local to the rehearsal*: the authoritative PR planner still fails closed
// for browser changes because real Playwright smoke is not modeled by it.
export const OFFLINE_BROWSER_TESTS = Object.freeze([
  { id: 'external:browser-collector-contract', dependsOn: ['browser-oracles'],
    buildDependsOn: [] },
  { id: 'external:browser-oracle-contract', dependsOn: ['browser-oracles'],
    buildDependsOn: [] },
]);

const EXPECTED_TESTS = Object.freeze([
  'cargo-test:grunkalunka-layout',
  'check:workspace-clippy',
  'check:workspace-format',
  'check:workspace-tests',
  'external:browser-collector-contract',
  'external:browser-oracle-contract',
]);

const EXPECTED_CARGO_COMMANDS = Object.freeze({
  format: ['cargo', 'fmt', '--all', '--check'],
  build: ['cargo', 'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
  clippy: ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
    '-p', 'grunkalunka-layout', '--', '-D', 'warnings'],
  test: ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
});

const workflowContracts = Object.freeze([
  {
    file: 'browser-oracles.yml',
    paths: ['tests/browser-oracles/**'],
    commands: ['node --test tests/browser-oracles/*.test.mjs'],
  },
  {
    file: 'browser-collector.yml',
    paths: ['tools/browser-oracles/**'],
    commands: ['node --test tools/browser-oracles/collect.test.mjs'],
  },
  {
    file: 'browser-smoke.yml',
    paths: ['tests/browser-oracles/**', 'tools/browser-oracles/**'],
    commands: [
      'engine: [chromium, firefox, webkit]',
      'node tools/browser-oracles/smoke.mjs',
      'mcr.microsoft.com/playwright:v',
    ],
  },
]);

// Narrow trigger/command source contract, not a YAML parser or proof of
// actual engine execution. Drift is a full-fallback reason for this rehearsal.
export function browserWorkflowCoverage(workflows) {
  const fail = reason => ({ safeToRehearse: false, reason });
  for (const { file, paths, commands } of workflowContracts) {
    const source = workflows?.[file];
    if (typeof source !== 'string') return fail('missing browser workflow: ' + file);
    const top = source.split(/\npermissions:\n/, 1)[0];
    const match = top.match(/\non:\n  pull_request:\n([\s\S]*?)\n  push:\n([\s\S]*?)\n  workflow_dispatch:/);
    if (!match) return fail('unrecognized browser workflow event structure: ' + file);
    for (const path of paths) {
      for (const [name, trigger] of [['pull_request', match[1]], ['push', match[2]]]) {
        if (!trigger.includes("'" + path + "'")) {
          return fail('missing ' + path + ' ' + name + ' trigger in ' + file);
        }
      }
    }
    for (const command of commands) {
      if (!source.includes(command)) return fail('missing command in ' + file + ': ' + command);
    }
  }
  return { safeToRehearse: true, reason: null };
}

export function browserLayoutPlan({ modules, groups, tests, packages, workflows }) {
  const coverage = browserWorkflowCoverage(workflows);
  if (!coverage.safeToRehearse) return { full: true, reason: coverage.reason };
  const settings = {
    changedPaths: [...HISTORICAL_PATHS], modules, groups,
    global: ['ci/impact/', '.github/', 'Cargo.toml', 'Cargo.lock'],
    ignored: ['docs/'],
  };
  const baseline = planImpact({ ...settings, tests });
  assert.equal(baseline.full, true, 'ordinary planner must remain fail-closed for browser');
  assert.match(baseline.reasons.join('; '), /no executable test covers changed owner: browser-oracles/);
  const expanded = planImpact({
    ...settings,
    tests: [...tests, ...OFFLINE_BROWSER_TESTS],
  });
  const cargo = previewCargoScope(expanded, packages);
  if (expanded.full || cargo.mode !== 'scoped-preview') {
    return { full: true, reason: expanded.reasons.join('; ') || cargo.reason };
  }
  // Refuse scope expansion and wrong executable targets before any test runs.
  assert.deepEqual(expanded.changedPaths, HISTORICAL_PATHS);
  assert.deepEqual(expanded.ignoredPaths, [
    'docs/browser-oracle-grid-placement.md', 'docs/headless-hit-test.md',
  ]);
  assert.deepEqual(expanded.changedOwners, ['browser-oracles', 'grunkalunka-layout']);
  assert.deepEqual(expanded.tests, EXPECTED_TESTS);
  assert.deepEqual(expanded.build, ['browser-oracles', 'grunkalunka-layout']);
  assert.deepEqual(cargo.selectedBuildPackages, ['grunkalunka-layout']);
  assert.deepEqual(cargo.selectedTestPackages, ['grunkalunka-layout']);
  assert.equal(cargo.advisoryOnly, true);
  assert.deepEqual(cargo.commands, EXPECTED_CARGO_COMMANDS);
  return {
    full: false,
    advisoryOnly: true,
    baselineFallback: baseline.reasons,
    changedPaths: [...HISTORICAL_PATHS],
    selectedTests: [...expanded.tests],
    selectedBuildModules: [...expanded.build],
    cargoCommands: structuredClone(cargo.commands),
    offlineBrowserCommands: [
      // Exact offline targets of existing independent browser workflows.
      ['node', '--test', 'tests/browser-oracles/*.test.mjs'],
      ['node', '--test', 'tools/browser-oracles/collect.test.mjs'],
    ],
    // Real Playwright smoke, cross-engine reporting and optional workflow
    // execution are NOT covered by these offline checks or partial preview.
    preservedNotExecuted: [
      'Browser engine smoke: chromium, firefox, webkit and cross-engine report',
    ],
  };
}
