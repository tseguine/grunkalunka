import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { parsePackageArgs, planPrCargoTests, runPrCargoTests } from '../../scripts/pr-cargo-tests.mjs';

const runtime = 'grunkalunka-runtime';
const classification = {
  schemaVersion: 1, package: runtime,
  fast: { reason: 'representative protocol coverage', targets: ['quick-a', 'quick-b'] },
  masterOnly: { reason: 'exhaustive matrix', targets: ['matrix-a'] },
};
const metadata = {
  workspace_members: ['layout-id', 'compiler-id', 'runtime-id'],
  packages: [
    { id: 'layout-id', name: 'grunkalunka-layout', targets: [{ name: 'layout', kind: ['lib'] }] },
    { id: 'compiler-id', name: 'grunkalunka-compiler', targets: [{ name: 'compiler', kind: ['lib'] }] },
    { id: 'runtime-id', name: runtime, targets: [
      { name: 'grunkalunka_runtime', kind: ['lib'] },
      ...['quick-a', 'quick-b', 'matrix-a'].map(name => ({ name, kind: ['test'] })),
    ] },
    // Cargo metadata can contain dependencies outside the selected workspace.
    { id: 'dependency-id', name: 'external-dependency', targets: [] },
  ],
};
const plan = (overrides = {}) => planPrCargoTests({ metadata, classification, ...overrides });

test('package arguments support explicit selections and reject malformed or repeated names', () => {
  assert.equal(parsePackageArgs([]), null);
  assert.deepEqual(parsePackageArgs(['--package', runtime, '--package', 'grunkalunka-layout']),
    [runtime, 'grunkalunka-layout']);
  for (const args of [null, ['--package'], ['--workspace', runtime], ['--package', ''],
    ['--package', '-p'], ['--package', 'runtime:*'], ['--package', runtime, '--package', runtime]]) {
    assert.throws(() => parsePackageArgs(args), /usage|duplicate/);
  }
});

test('PR Cargo plan keeps all other package tests and only classified runtime integration targets', () => {
  const result = plan();
  assert.equal(result.profile, 'pr');
  assert.deepEqual(result.selectedPackages, ['grunkalunka-compiler', 'grunkalunka-layout', runtime]);
  const otherArgs = ['test', '--locked', '--offline', '-j', '4',
    '-p', 'grunkalunka-compiler', '-p', 'grunkalunka-layout'];
  const runtimeArgs = ['test', '--locked', '--offline', '-j', '4', '-p', runtime,
    '--lib', '--test', 'quick-a', '--test', 'quick-b'];
  assert.deepEqual(result.commands, [
    { phase: 'compile', command: 'cargo', args: [...otherArgs, '--no-run'], timeoutMs: 600_000 },
    { phase: 'compile', command: 'cargo', args: [...runtimeArgs, '--no-run'], timeoutMs: 600_000 },
    { phase: 'test', command: 'cargo', args: otherArgs, timeoutMs: 120_000 },
    { phase: 'test', command: 'cargo', args: runtimeArgs, timeoutMs: 120_000 },
  ]);
  assert.ok(result.commands.every(command => !command.args.includes('--workspace')));
  assert.ok(result.commands.every(command => !command.args.includes('matrix-a')));
  assert.ok(result.commands.every(command => !command.args.includes('external-dependency')));
  assert.deepEqual(result.runtime.masterOnly.targets, ['matrix-a']);
});

test('scoped Cargo plans never add tests for unselected build dependencies', () => {
  for (const packages of [[runtime], ['grunkalunka-layout']]) {
    const result = plan({ packages });
    assert.deepEqual(result.selectedPackages, packages);
    assert.equal(result.commands.length, 2);
    assert.deepEqual(result.commands.map(command => command.phase), ['compile', 'test']);
    assert.deepEqual(result.commands.map(command => command.timeoutMs), [600_000, 120_000]);
    for (const command of result.commands) {
      assert.deepEqual(command.args.filter((_, index, args) => args[index - 1] === '-p'), packages);
    }
  }
  for (const packages of [[], ['unknown-package'], ['external-dependency'], [runtime, runtime]]) {
    assert.throws(() => plan({ packages }), /selected Cargo packages/);
  }
});

test('new and stale runtime targets fail classification even when another package is selected', () => {
  const newTarget = structuredClone(metadata);
  newTarget.packages.find(pkg => pkg.name === runtime).targets.push({ name: 'new-matrix', kind: ['test'] });
  assert.throws(() => plan({ metadata: newTarget, packages: ['grunkalunka-layout'] }),
    /unclassified.*new-matrix/);
  const removedTarget = structuredClone(metadata);
  removedTarget.packages.find(pkg => pkg.name === runtime).targets.pop();
  assert.throws(() => plan({ metadata: removedTarget }), /stale.*matrix-a/);
});

test('invalid target policy, overlapping groups and missing library coverage are rejected', () => {
  for (const change of [
    value => { value.fast.targets.push('matrix-a'); },
    value => { value.fast.targets.push(value.fast.targets[0]); },
    value => { value.fast.targets = []; },
    value => { value.masterOnly.reason = ' '; },
    value => { value.masterOnly.extra = true; },
    value => { value.schemaVersion = 2; },
    value => { value.package = 'grunkalunka-layout'; },
  ]) {
    const value = structuredClone(classification);
    change(value);
    assert.throws(() => plan({ classification: value }), /invalid|duplicate/);
  }
  const withoutLib = structuredClone(metadata);
  withoutLib.packages.find(pkg => pkg.name === runtime).targets.shift();
  assert.throws(() => plan({ metadata: withoutLib }), /library target is missing/);
});

test('metadata must identify each workspace package and runtime target unambiguously', () => {
  for (const change of [
    value => { value.workspace_members.push('missing-id'); },
    value => { value.workspace_members.push(value.workspace_members[0]); },
    value => { value.packages.push(value.packages[0]); },
    value => { value.packages[0].name = value.packages[1].name; },
    value => { value.packages.find(pkg => pkg.name === runtime).targets.push({ name: 'quick-a', kind: ['test'] }); },
    value => { value.packages.find(pkg => pkg.name === runtime).targets[0].kind = []; },
  ]) {
    const value = structuredClone(metadata);
    change(value);
    assert.throws(() => plan({ metadata: value }), /invalid|duplicate|missing/);
  }
});

test('checked-in classification accounts for every current runtime integration source', () => {
  const actual = readdirSync(new URL('../../crates/grunkalunka-runtime/tests/', import.meta.url))
    .filter(name => name.endsWith('.rs')).map(name => name.slice(0, -3)).sort();
  const policy = JSON.parse(readFileSync(new URL('./pr-cargo-tests.json', import.meta.url), 'utf8'));
  // This source inventory regression supplements, rather than substitutes for,
  // the real cargo metadata validation performed on each native CI execution.
  assert.equal(policy.fast.targets.length, 21);
  assert.ok(policy.fast.targets.includes('core_engine_process_memory'),
    'native memory sampling and its deterministic recovery checks run on PRs');
  assert.deepEqual([...policy.fast.targets, ...policy.masterOnly.targets].sort(), actual);
  for (const deferred of ['async-suspension', 'promise-notifications', 'managed-strings', 'runtime-closures']) {
    assert.ok(policy.masterOnly.targets.includes(deferred), deferred);
  }
  const actualMetadata = structuredClone(metadata);
  actualMetadata.packages.find(pkg => pkg.name === runtime).targets = [
    { name: 'grunkalunka_runtime', kind: ['lib'] },
    ...actual.map(name => ({ name, kind: ['test'] })),
  ];
  const result = plan({ metadata: actualMetadata, classification: policy, packages: [runtime] });
  assert.ok(result.commands.every(command => command.args.includes('--lib')));
  const runtimeCommands = result.commands.filter(command => command.phase === 'test');
  assert.equal(runtimeCommands.length, 2, '21 fast tests should fit two bounded runs');
  assert.equal(result.commands.length, 4, 'compile each bounded group first');
  assert.deepEqual(runtimeCommands.map(command => command.timeoutMs), [120_000, 120_000]);
  assert.ok(runtimeCommands[0].args.includes('--lib'));
  assert.ok(!runtimeCommands[1].args.includes('--lib'));
  const selected = runtimeCommands.flatMap(command => command.args
    .filter((_, index, args) => args[index - 1] === '--test'));
  assert.deepEqual(selected, policy.fast.targets, 'every fast target runs once in stable order');
  assert.equal(new Set(selected).size, selected.length, 'no duplicate fast targets');
});

test('compile or execution failures stop immediately without retries or exhaustive fallback', () => {
  for (const failureAt of [0, 2, 3]) {
    const calls = [];
    assert.throws(() => runPrCargoTests(plan(), (command, args, options) => {
      calls.push({ command, args, options });
      if (calls.length - 1 === failureAt) throw new Error('native command failed');
      return 1;
    }), /native command failed/);
    assert.equal(calls.length, failureAt + 1);
    assert.ok(calls.every(call => !call.args.includes('--workspace')));
  }
  const unbounded = plan();
  unbounded.commands[0].timeoutMs = 0;
  assert.throws(() => runPrCargoTests(unbounded, () => assert.fail('must not run')), /command budget/);
});

test('production helper bounds discovery, compilation and execution, and fails on real invoke errors', () => {
  const helper = new URL('../../scripts/pr-cargo-tests.mjs', import.meta.url);
  const policy = JSON.parse(readFileSync(new URL('./pr-cargo-tests.json', import.meta.url), 'utf8'));
  const actualMetadata = structuredClone(metadata);
  actualMetadata.packages.find(pkg => pkg.name === runtime).targets = [
    { name: 'grunkalunka_runtime', kind: ['lib'] },
    ...[...policy.fast.targets, ...policy.masterOnly.targets].map(name => ({ name, kind: ['test'] })),
  ];
  for (const failure of ['none', 'metadata', 'compile', 'test', 'timeout']) {
    // Exercise main -> metadata -> plan -> run -> invoke in an isolated child.
    // No native test execution or Cargo availability is claimed by this fixture.
    const source = `
      import childProcess from 'node:child_process';
      import { syncBuiltinESMExports } from 'node:module';
      const calls = [];
      childProcess.spawnSync = (command, args, options) => {
        calls.push({ command, args, timeout: options.timeout, killSignal: options.killSignal });
        const phase = args[0] === 'metadata' ? 'metadata' : args.includes('--no-run') ? 'compile' : 'test';
        if (${JSON.stringify(failure)} === 'timeout' && phase === 'test') {
          return { status: null, signal: 'SIGKILL', error: new Error('ETIMEDOUT') };
        }
        if (phase === ${JSON.stringify(failure)}) return { status: 7, stderr: 'native failure' };
        return { status: 0, stdout: phase === 'metadata' ? ${JSON.stringify(JSON.stringify(actualMetadata))} : '' };
      };
      syncBuiltinESMExports();
      process.argv = [process.execPath, ${JSON.stringify(fileURLToPath(helper))}, '--package', ${JSON.stringify(runtime)}];
      try { await import(${JSON.stringify(helper.href)}); }
      finally { console.log('CARGO SPAWNS ' + JSON.stringify(calls)); }
    `;
    const result = spawnSync(process.execPath, ['--input-type=module', '--eval', source], {
      encoding: 'utf8', timeout: 10_000,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    const line = result.stdout.split('\n').find(line => line.startsWith('CARGO SPAWNS '));
    assert.ok(line, result.stdout + result.stderr);
    const calls = JSON.parse(line.slice('CARGO SPAWNS '.length));
    const expectedCount = failure === 'metadata' ? 1 : failure === 'compile' ? 2 : 4;
    assert.equal(calls.length, expectedCount);
    assert.deepEqual(calls.map(call => call.timeout),
      [30_000, 600_000, 600_000, 120_000].slice(0, expectedCount));
    assert.ok(calls.every(call => call.killSignal === 'SIGKILL'));
    assert.deepEqual(calls[0].args,
      ['metadata', '--format-version', '1', '--no-deps', '--locked', '--offline']);
    assert.equal(result.status, failure === 'none' ? 0 : 1, result.stderr);
    if (failure === 'none') assert.match(result.stdout, /PASS PR CARGO /);
    else assert.doesNotMatch(result.stdout, /PASS PR CARGO /);
  }
});
