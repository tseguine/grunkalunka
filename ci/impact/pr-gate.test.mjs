import assert from 'node:assert/strict';
import test from 'node:test';
import {
  choosePrChecks, chooseWithFallback, runDecision,
} from './pr-gate.mjs';

const host = { eventName: 'pull_request', runnerOS: 'Windows', enabled: true };
const packages = [
  'grunkalunka-layout', 'grunkalunka-compiler', 'grunkalunka-runtime',
  'grunkalunka-cli', 'grunkalunka-json',
];
const declared = [
  { id: 'check:node-execution', command: 'node', args: [
    '--test', 'tests/build/execution.test.mjs',
  ] },
  { id: 'check:workspace-format', command: 'cargo',
    args: ['fmt', '--all', '--check'], scope: 'workspace' },
  { id: 'check:workspace-clippy', command: 'cargo',
    args: ['clippy', '--workspace', '--all-targets', '--locked', '--offline',
      '-j', '4', '--', '-D', 'warnings'], scope: 'workspace' },
  { id: 'check:workspace-tests', command: 'cargo',
    args: ['test', '--workspace', '--locked', '--offline', '-j', '4'],
    scope: 'workspace' },
  { id: 'check:compiler-compatibility-replay', command: 'node',
    args: ['scripts/poc.mjs'] },
  { id: 'check:markdown-portability', command: 'node',
    args: ['scripts/check-markdown-portability.mjs'] },
];
const observed = declared.map(({ command, args }) => ({ command, args: [...args] }));
const externalWorkflows = {
  advisoryOnly: true, scope: 'independent browser-oracle workflows only',
  valid: true, expectedToTrigger: [], requiredNotTriggered: [], errors: [],
};

function plan(paths, tests, build) {
  const selected = [...new Set(tests)].sort();
  return {
    mode: 'shadow', advisoryOnly: true, full: false, reasons: [],
    changedPaths: paths,
    changedOwners: ['grunkalunka-compiler'],
    ignoredPaths: [],
    build: [...build].sort(),
    tests: selected,
    selectedCommands: selected.filter(t => t.startsWith('check:')),
    canonicalCoverage: { commands: declared.length, mapped: declared.length },
    externalWorkflows: structuredClone(externalWorkflows),
  };
}
const compilerTests = [
  'cargo-test:grunkalunka-runtime', 'cargo-test:grunkalunka-compiler',
  'check:workspace-format', 'check:workspace-clippy', 'check:workspace-tests',
  'check:node-execution', 'check:compiler-compatibility-replay',
];
const compiler = () => plan([
  'crates/grunkalunka-compiler/src/ir.rs',
  'crates/grunkalunka-runtime/tests/async-suspension.rs',
], compilerTests, ['grunkalunka-compiler', 'grunkalunka-runtime']);

const choose = input => choosePrChecks({
  ...host, declared, observed, workspacePackages: packages,
  plan: input ?? compiler(),
});

test('compiler/runtime changes select their Cargo tests and original Node checks', () => {
  const result = choose(compiler());
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.selectedPackages, ['grunkalunka-compiler', 'grunkalunka-runtime']);
  assert.deepEqual(result.commands, [
    ['cargo', 'build', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime'],
    ['node', '--test', 'tests/build/execution.test.mjs'],
    ['cargo', 'fmt', '--all', '--check'],
    ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime', '--', '-D', 'warnings'],
    ['cargo', 'test', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime'],
    ['node', 'scripts/poc.mjs'],
  ]);
  assert.ok(!result.commands.flat().includes('scripts/check.mjs'));
  assert.ok(!result.commands.flat().includes('scripts/check-markdown-portability.mjs'));
});

test('Node-only changes select Node checks with a Cargo build prerequisite', () => {
  const input = plan(['tools/build/compile.mjs'], [
    'check:node-execution',
  ], ['grunkalunka-compiler', 'grunkalunka-cli']);
  const result = choose(input);
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.commands, [
    ['cargo', 'build', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-cli', '-p', 'grunkalunka-compiler'],
    ['node', '--test', 'tests/build/execution.test.mjs'],
  ]);
});

test('layout-only changes select only layout Cargo tests without hardcoded path rules', () => {
  const input = plan(['crates/grunkalunka-layout/src/lib.rs'], [
    'cargo-test:grunkalunka-layout',
    'check:workspace-format',
    'check:workspace-clippy',
    'check:workspace-tests',
  ], ['grunkalunka-layout']);
  const result = choose(input);
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.selectedPackages, ['grunkalunka-layout']);
  assert.deepEqual(result.commands[0],
    ['cargo', 'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout']);
  assert.deepEqual(result.commands.at(-1),
    ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout']);
});

test('all-doc PRs can select no commands after validated planning', () => {
  const input = plan(['docs/async.md'], [], []);
  input.changedOwners = [];
  input.ignoredPaths = ['docs/async.md'];
  const result = choose(input);
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.commands, []);
});

test('independent browser workflow expectations stay visible but are not claimed executed', () => {
  const input = compiler();
  input.externalWorkflows.expectedToTrigger = ['workflow:browser-smoke'];
  const result = choose(input);
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.independentWorkflowsExpected, ['workflow:browser-smoke']);
  assert.equal(result.externalJobsVerified, false);
});

test('Linux, Windows and macOS use identical dependency-selected PR commands', () => {
  const original = choose(compiler());
  for (const runnerOS of ['Linux', 'Windows', 'macOS']) {
    const result = choosePrChecks({
      ...host, runnerOS, plan: compiler(), declared, observed,
      workspacePackages: packages,
    });
    assert.deepEqual(result, original, runnerOS);
    assert.equal(result.mode, 'partial');
    assert.ok(!result.commands.flat().includes('scripts/check.mjs'));
  }
});

test('master pushes, manual runs, disabled selection and unknown platforms stay full', () => {
  for (const value of [
    ...['Linux', 'Windows', 'macOS'].flatMap(runnerOS => [
      { eventName: 'push', runnerOS, enabled: true },
      { eventName: 'workflow_dispatch', runnerOS, enabled: true },
      { eventName: 'pull_request', runnerOS, enabled: false },
    ]),
    { eventName: 'pull_request', runnerOS: 'FreeBSD', enabled: true },
    { eventName: 'pull_request', runnerOS: 'windows', enabled: true },
    { eventName: 'pull_request', runnerOS: undefined, enabled: true },
  ]) {
    const result = choosePrChecks({
      ...value, plan: compiler(), declared, observed, workspacePackages: packages,
    });
    assert.equal(result.mode, 'full', JSON.stringify(value));
    assert.deepEqual(result.commands, [
      ['cargo', 'build', '--workspace', '--locked', '--offline', '-j', '4'],
      ['node', 'scripts/check.mjs'],
    ]);
  }
});

test('ambiguous, stale or missing selection safely runs full CI', () => {
  const mutators = [
    p => { p.full = true; p.reasons = ['unattributed path']; },
    p => { p.mode = 'authoritative'; },
    p => { p.advisoryOnly = false; },
    p => { p.canonicalCoverage.mapped--; },
    p => { p.selectedCommands = ['check:workspace-tests']; },
    p => { p.tests.push('test:unresolved-group'); },
    p => { p.tests.push('cargo-test:unknown-package'); },
    p => { p.tests.push('external:unverified-workflow'); },
    p => { p.build = ['not-a-workspace-package']; },
    p => { p.build = []; },
    p => { p.externalWorkflows.requiredNotTriggered.push('workflow:browser-smoke'); },
    p => { p.externalWorkflows.valid = false; },
    p => { p.externalWorkflows.errors.push('unsupported paths'); },
    p => { delete p.externalWorkflows; },
    p => { delete p.changedPaths; },
    p => { p.changedPaths = []; },
    p => { p.changedPaths.push(p.changedPaths[0]); },
    p => { p.tests.push(p.tests[0]); },
  ];
  for (const mutate of mutators) {
    const input = compiler();
    mutate(input);
    const result = choose(input);
    assert.equal(result.mode, 'full', mutate.toString());
  }
  for (const malformed of [null, {}, { full: false }, { mode: 'shadow' }]) {
    assert.equal(choosePrChecks({
      ...host, plan: malformed, declared, observed, workspacePackages: packages,
    }).mode, 'full');
  }
  const stale = observed.slice(1);
  assert.equal(choosePrChecks({
    ...host, plan: compiler(), declared, observed: stale, workspacePackages: packages,
  }).mode, 'full');
});

test('planner crashes and invalid JSON lead to the full command contract', () => {
  const result = chooseWithFallback({
    ...host, capture: () => { throw new Error('git merge-base failed'); },
  });
  assert.equal(result.mode, 'full');
  assert.match(result.reason, /planning unavailable/);
  assert.deepEqual(result.commands.at(-1), ['node', 'scripts/check.mjs']);
});

test('selected command failure is terminal and is not turned into full fallback', () => {
  const choice = choose(compiler());
  const called = [];
  assert.throws(() => runDecision(choice, (cmd, args) => {
    called.push([cmd, ...args]);
    if (args[0] === 'test') throw new Error('test failure');
    return 2;
  }), /test failure/);
  assert.equal(called.length, 5);
  assert.ok(!called.some(x => x.includes('scripts/check.mjs')));
});

test('invalid executable decisions are rejected before execution', () => {
  const choice = choose(compiler());
  choice.commands[0][0] = 'sh';
  assert.throws(() => runDecision(choice, () => {}), /invalid selected executable/);
  const full = choosePrChecks({
    eventName: 'push', runnerOS: 'Windows', enabled: true,
  });
  full.commands.pop();
  assert.throws(() => runDecision(full, () => {}), /full CI command contract/);
});
