import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
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
    scope: 'workspace', pr: { reason: 'bounded runtime coverage', commands: [
      { command: 'node', args: ['scripts/pr-cargo-tests.mjs'] },
    ] } },
  { id: 'check:compiler-compatibility-replay', command: 'node',
    args: ['scripts/poc.mjs'], pr: { reason: 'exhaustive replay runs on master', commands: [] } },
  { id: 'check:markdown-portability', command: 'node',
    args: ['scripts/check-markdown-portability.mjs'] },
];
const observed = declared.map(({ command, args }) => ({ command, args: [...args] }));
const prObserved = observed.filter(check => check.args[0] !== 'scripts/poc.mjs')
  .map(check => check.command === 'cargo' && check.args[0] === 'test'
    ? { command: 'node', args: ['scripts/pr-cargo-tests.mjs'] } : structuredClone(check));
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

const choose = (input, overrides = {}) => choosePrChecks({
  ...host, declared, observed, prObserved, workspacePackages: packages,
  plan: input ?? compiler(),
  ...overrides,
});

test('compiler/runtime changes select bounded Cargo coverage and defer exhaustive checks', () => {
  const result = choose(compiler());
  assert.equal(result.mode, 'partial', result.reason);
  assert.equal(result.profile, 'pr');
  assert.deepEqual(result.selectedPackages, ['grunkalunka-compiler', 'grunkalunka-runtime']);
  assert.deepEqual(result.commands, [
    ['cargo', 'build', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime'],
    ['node', '--test', 'tests/build/execution.test.mjs'],
    ['cargo', 'fmt', '--all', '--check'],
    ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime', '--', '-D', 'warnings'],
    ['node', 'scripts/pr-cargo-tests.mjs', '--package', 'grunkalunka-compiler',
      '--package', 'grunkalunka-runtime'],
  ]);
  assert.ok(result.selectedTests.includes('check:compiler-compatibility-replay'));
  assert.ok(!result.commands.flat().includes('scripts/poc.mjs'));
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
    ['node', 'scripts/pr-cargo-tests.mjs', '--package', 'grunkalunka-layout']);
});

test('Cargo test selection does not acquire tests of build-only dependencies', () => {
  const input = plan(['crates/grunkalunka-runtime/src/lib.rs'], [
    'cargo-test:grunkalunka-runtime', 'check:workspace-tests',
  ], ['grunkalunka-compiler', 'grunkalunka-runtime']);
  const result = choose(input);
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.commands, [
    ['cargo', 'build', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-runtime'],
    ['node', 'scripts/pr-cargo-tests.mjs', '--package', 'grunkalunka-runtime'],
  ]);
});

test('selected master-only nodes remain visible without executing their full commands', () => {
  const result = choose(plan(['scripts/poc.mjs'], ['check:compiler-compatibility-replay'], []));
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.selectedTests, ['check:compiler-compatibility-replay']);
  assert.deepEqual(result.commands, []);
});

test('bounded variants retain descriptor order inside the selected canonical command', () => {
  const changed = structuredClone(declared);
  changed[0].pr = { reason: 'separate bounded cases', commands: [
    { command: 'node', args: ['--test', 'tests/build/fast-one.test.mjs'] },
    { command: 'node', args: ['--test', 'tests/build/fast-two.test.mjs'] },
  ] };
  const result = choose(plan(['tools/build/compile.mjs'], ['check:node-execution'], []), {
    declared: changed, prObserved: [...changed[0].pr.commands, ...prObserved.slice(1)],
  });
  assert.equal(result.mode, 'partial', result.reason);
  assert.deepEqual(result.commands, changed[0].pr.commands.map(({ command, args }) => [command, ...args]));
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
      ...host, runnerOS, plan: compiler(), declared, observed, prObserved,
      workspacePackages: packages,
    });
    assert.deepEqual(result, original, runnerOS);
    assert.equal(result.mode, 'partial');
    assert.ok(!result.commands.flat().includes('scripts/check.mjs'));
  }
});

test('complete PR fallbacks stay bounded while master pushes and manual runs retain full checks', () => {
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
      ...value, plan: compiler(), declared, observed, prObserved, workspacePackages: packages,
    });
    assert.equal(result.mode, 'full', JSON.stringify(value));
    assert.deepEqual(result.commands, [
      ['cargo', 'build', '--workspace', '--locked', '--offline', '-j', '4'],
      ['node', 'scripts/check.mjs', ...(value.eventName === 'pull_request' ? ['--profile=pr'] : [])],
    ]);
    assert.equal(result.profile, value.eventName === 'pull_request' ? 'pr' : 'full');
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
      ...host, plan: malformed, declared, observed, prObserved, workspacePackages: packages,
    }).mode, 'full');
  }
  const stale = observed.slice(1);
  assert.equal(choosePrChecks({
    ...host, plan: compiler(), declared, observed: stale, prObserved, workspacePackages: packages,
  }).mode, 'full');
});

test('missing or stale PR inventory falls back to the complete bounded PR profile', () => {
  for (const inventory of [undefined, observed, prObserved.slice(1), [...prObserved, prObserved[0]]]) {
    const result = choose(compiler(), { prObserved: inventory });
    assert.equal(result.mode, 'full');
    assert.equal(result.profile, 'pr');
    assert.deepEqual(result.commands.at(-1), ['node', 'scripts/check.mjs', '--profile=pr']);
  }
});

test('workspace Cargo projection cannot bypass the bounded test helper', () => {
  const changed = structuredClone(declared);
  delete changed.find(check => check.id === 'check:workspace-tests').pr;
  const changedObserved = prObserved.map(check => check.args[0] === 'scripts/pr-cargo-tests.mjs'
    ? observed.find(check => check.command === 'cargo' && check.args[0] === 'test') : check);
  const result = choose(compiler(), { declared: changed, prObserved: changedObserved });
  assert.equal(result.mode, 'full');
  assert.match(result.reason, /lost their bounded PR helper/);
  assert.deepEqual(result.commands.at(-1), ['node', 'scripts/check.mjs', '--profile=pr']);
});

test('planner crashes and invalid JSON lead to the full command contract', () => {
  const result = chooseWithFallback({
    ...host, capture: () => { throw new Error('git merge-base failed'); },
  });
  assert.equal(result.mode, 'full');
  assert.match(result.reason, /planning unavailable/);
  assert.equal(result.profile, 'pr');
  assert.deepEqual(result.commands.at(-1), ['node', 'scripts/check.mjs', '--profile=pr']);
});

test('selected command failure is terminal and is not turned into full fallback', () => {
  const choice = choose(compiler());
  const called = [];
  assert.throws(() => runDecision(choice, (cmd, args) => {
    called.push([cmd, ...args]);
    if (args[0] === 'scripts/pr-cargo-tests.mjs') throw new Error('test failure');
    return 2;
  }), /test failure/);
  assert.equal(called.length, 5);
  assert.ok(!called.some(x => x.includes('scripts/check.mjs')));
});

test('full fallback gives the aggregate the job budget while bounding its build', () => {
  const choice = choosePrChecks({ eventName: 'push', runnerOS: 'Linux', enabled: true });
  const called = [];
  const receipts = runDecision(choice, (command, args, options = { timeout: 45 * 60_000 }) => {
    called.push({ command, args, options });
    // A healthy full run can exceed the selected-command watchdog. The
    // workflow still owns the total job deadline, as it does on master.
    const elapsedMs = command === 'node' ? 50 * 60_000 : 60_000;
    if (options.timeout !== 0 && elapsedMs >= options.timeout) {
      throw new Error('aggregate interrupted by selected-command watchdog');
    }
    return elapsedMs;
  });
  assert.deepEqual(called.map(call => call.options), [
    { timeout: 45 * 60_000 }, { timeout: 0 },
  ]);
  assert.equal(receipts.length, 2);
  assert.equal(receipts[1].elapsedMs, 50 * 60_000);
  assert.equal(receipts[1].status, 'passed');
});

test('partial PR execution applies test and compilation bounds, delegating only fixed Cargo aggregates', () => {
  const choice = choose(compiler());
  choice.commands.push(['node', 'scripts/check.mjs']);
  const called = [];
  runDecision(choice, (command, args, options) => {
    called.push({ command, args, options });
    return 1;
  });
  assert.equal(called.length, choice.commands.length);
  assert.deepEqual(called.map(call => call.options.timeout), [
    600_000, 120_000, 120_000, 600_000, 0, 120_000,
  ]);
});

test('complete PR fallback keeps the aggregate job budget and bounds its build to ten minutes', () => {
  const result = chooseWithFallback({
    ...host, capture: () => { throw new Error('unavailable planner'); },
  });
  const calls = [];
  runDecision(result, (command, args, options) => {
    calls.push({ command, args, timeout: options.timeout });
    return 1;
  });
  assert.deepEqual(calls, [
    { command: 'cargo', args: ['build', '--workspace', '--locked', '--offline', '-j', '4'], timeout: 600_000 },
    { command: 'node', args: ['scripts/check.mjs', '--profile=pr'], timeout: 0 },
  ]);
});

test('production complete execution forwards each event profile and propagates aggregate failure', () => {
  const gate = new URL('./pr-gate.mjs', import.meta.url);
  for (const { eventName, aggregateStatus } of ['workflow_dispatch', 'pull_request']
    .flatMap(eventName => [0, 7].map(aggregateStatus => ({ eventName, aggregateStatus })))) {
    // Isolate the subprocess stand-in in a child so the real main/execute/invoke
    // path runs without changing this test process or compiling native code.
    const source = `
      import childProcess from 'node:child_process';
      import { syncBuiltinESMExports } from 'node:module';
      const calls = [];
      childProcess.spawnSync = (command, args, options) => {
        calls.push({ command, args, timeout: options.timeout, killSignal: options.killSignal });
        return { status: command === process.execPath ? ${aggregateStatus} : 0 };
      };
      syncBuiltinESMExports();
      process.argv = [process.execPath, ${JSON.stringify(fileURLToPath(gate))}];
      process.env.GITHUB_EVENT_NAME = ${JSON.stringify(eventName)};
      process.env.PARTIAL_PR_CI = 'false';
      try {
        await import(${JSON.stringify(gate.href)});
      } finally {
        console.log('PR GATE SPAWNS ' + JSON.stringify(calls));
      }
    `;
    const result = spawnSync(process.execPath, ['--input-type=module', '--eval', source], {
      encoding: 'utf8', timeout: 10_000,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    const receipt = result.stdout.split('\n').find(line => line.startsWith('PR GATE SPAWNS '));
    assert.ok(receipt, result.stdout + result.stderr);
    assert.deepEqual(JSON.parse(receipt.slice('PR GATE SPAWNS '.length)), [
      { command: 'cargo', args: ['build', '--workspace', '--locked', '--offline', '-j', '4'],
        timeout: eventName === 'pull_request' ? 600_000 : 45 * 60_000, killSignal: 'SIGKILL' },
      { command: process.execPath,
        args: ['scripts/check.mjs', ...(eventName === 'pull_request' ? ['--profile=pr'] : [])], timeout: 0 },
    ]);
    assert.equal(result.status, aggregateStatus === 0 ? 0 : 1, result.stderr);
    if (aggregateStatus === 0) {
      assert.match(result.stdout, /PASS PR CI /);
      assert.match(result.stdout, new RegExp('"profile":"' + (eventName === 'pull_request' ? 'pr' : 'full') + '"'));
    }
    else {
      assert.doesNotMatch(result.stdout, /PASS PR CI /);
      assert.match(result.stderr, /node scripts\/check\.mjs(?: --profile=pr)? failed: 7/);
    }
  }
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
