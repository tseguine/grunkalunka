import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import {
  commandTimeoutMs, loadCanonicalCommands, parseCheckProfile, projectPrCommands,
} from '../../scripts/check-catalog.mjs';
import { verifyCanonicalChecks, verifyPrChecks } from './canonical.mjs';
import { loadMasterCheckGroups, partitionMasterChecks } from '../../scripts/master-check-groups.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const catalogDirectory = fileURLToPath(new URL('../../scripts/checks/', import.meta.url));
const descriptorDirectory = fileURLToPath(new URL('./checks/', import.meta.url));

test('canonical manifest commands match the independent impact descriptors', () => {
  const names = readdirSync(catalogDirectory)
    .filter(name => name.endsWith('.json'))
    .map(name => name.replace(/^\d{6}-/, '').replace(/\.json$/, ''));
  const descriptors = readdirSync(descriptorDirectory)
    .filter(name => name.endsWith('.json'))
    .map(name => JSON.parse(readFileSync(join(descriptorDirectory, name), 'utf8')));
  assert.equal(new Set(names).size, names.length, 'no duplicate logical check names');
  assert.deepEqual(names.slice().sort(), descriptors.map(d => d.id.slice('check:'.length)).sort());
  const actual = loadCanonicalCommands();
  assert.deepEqual(verifyCanonicalChecks(actual, descriptors),
    { commands: actual.length, mapped: actual.length });
  const pr = loadCanonicalCommands(undefined, { profile: 'pr' });
  assert.deepEqual(verifyPrChecks(pr, descriptors), { commands: pr.length, mapped: pr.length });
});

test('actual canonical inventory capture preserves manifest order', () => {
  for (const profile of ['full', 'pr']) {
    const captured = JSON.parse(execFileSync(process.execPath,
      ['ci/impact/collect-checks.mjs', '--profile=' + profile],
      { cwd: root, encoding: 'utf8', timeout: 10000 }));
    assert.deepEqual(captured, loadCanonicalCommands(undefined, { profile }));
  }
});

function temporaryCatalog(t) {
  const directory = mkdtempSync(join(tmpdir(), 'grunkalunka-check-catalog-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const put = (file, data) => writeFileSync(join(directory, file), data);
  return { directory, put };
}

test('manifest catalog is sorted and supports sparse insertion positions', t => {
  const { directory, put } = temporaryCatalog(t);
  put('009000-b.json', JSON.stringify({ command: 'cargo', args: ['test'] }));
  put('001000-a.json', JSON.stringify({ command: 'node', args: ['--test', 'a.test.mjs'] }));
  put('README.md', 'Documentation is not a check.');
  assert.deepEqual(loadCanonicalCommands(directory), [
    { command: 'node', args: ['--test', 'a.test.mjs'] },
    { command: 'cargo', args: ['test'] },
  ]);
});

test('missing, malformed, duplicate or unexpectedly named catalog entries fail closed', t => {
  const { directory, put } = temporaryCatalog(t);
  assert.throws(() => loadCanonicalCommands(directory), /empty/);
  put('001000-one.json', JSON.stringify({ command: 'node', args: ['one.mjs'] }));
  assert.equal(loadCanonicalCommands(directory).length, 1);
  put('002000-two.json', JSON.stringify({ command: 'node', args: ['one.mjs'] }));
  assert.throws(() => loadCanonicalCommands(directory), /duplicate/);
  put('002000-two.json', JSON.stringify({ command: 'shell', args: ['echo', 'hi'] }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: [] }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: ['test'], extra: true }));
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check command/);
  put('002000-two.json', 'not-json');
  assert.throws(() => loadCanonicalCommands(directory), /invalid canonical-check manifest/);
  put('002000-two.json', JSON.stringify({ command: 'cargo', args: ['test'] }));
  put('unexpected.mjs', '// unexpected code');
  assert.throws(() => loadCanonicalCommands(directory), /unsupported canonical-check catalog entry/);
});

test('PR exclusions and bounded variants leave the original full inventory intact', t => {
  const { directory, put } = temporaryCatalog(t);
  const unchanged = { command: 'node', args: ['fast.mjs'] };
  const slow = { command: 'node', args: ['slow.mjs'],
    pr: { reason: 'Long exhaustive integration runs on master.', commands: [] } };
  const original = { command: 'cargo', args: ['test', '--workspace'] };
  const variants = [
    { command: 'cargo', args: ['test', '-p', 'small', '--no-run'] },
    { command: 'cargo', args: ['test', '-p', 'small'] },
  ];
  put('001000-fast.json', JSON.stringify(unchanged));
  put('002000-slow.json', JSON.stringify(slow));
  put('003000-variants.json', JSON.stringify({ ...original,
    pr: { reason: 'Compile before the bounded representative test.', commands: variants } }));
  assert.deepEqual(loadCanonicalCommands(directory), [
    unchanged, { command: slow.command, args: slow.args }, original,
  ]);
  assert.deepEqual(loadCanonicalCommands(directory, { profile: 'pr' }), [unchanged, ...variants]);
  const descriptor = { id: 'check:fast', dependsOn: ['source'], ...unchanged };
  const projected = projectPrCommands(descriptor);
  projected[0].args[0] = 'changed.mjs';
  assert.deepEqual(descriptor.args, ['fast.mjs'], 'projection does not mutate registration');
});

test('all policies are validated even when their command would be excluded', t => {
  const { directory, put } = temporaryCatalog(t);
  const base = { command: 'node', args: ['slow.mjs'] };
  for (const pr of [
    null, false, 'master-only', {}, { reason: 'why', commands: null },
    { reason: '', commands: [] }, { reason: '  ', commands: [] },
    { reason: 'why', commands: [], extra: true },
    { reason: 'why', commands: [{ command: 'shell', args: ['ignored'] }] },
    { reason: 'why', commands: [{ command: 'node', args: [] }] },
    { reason: 'why', commands: [{ command: 'node', args: ['bad\0argument'] }] },
    { reason: 'why', commands: [{ command: 'node', args: ['fast.mjs'], extra: true }] },
  ]) {
    put('001000-policy.json', JSON.stringify({ ...base, pr }));
    for (const profile of ['full', 'pr']) {
      assert.throws(() => loadCanonicalCommands(directory, { profile }), /invalid canonical-check command/);
    }
  }
  put('001000-policy.json', JSON.stringify({ command: 'shell', args: ['never'],
    pr: { reason: 'Excluded command must still be valid on master.', commands: [] } }));
  assert.throws(() => loadCanonicalCommands(directory, { profile: 'pr' }), /invalid canonical/);
});

test('PR variants cannot duplicate another check or smuggle in another policy', t => {
  const { directory, put } = temporaryCatalog(t);
  const fast = { command: 'node', args: ['fast.mjs'] };
  put('001000-fast.json', JSON.stringify(fast));
  put('002000-duplicate.json', JSON.stringify({ command: 'node', args: ['slow.mjs'],
    pr: { reason: 'Conflicting ownership is invalid.', commands: [fast] } }));
  for (const profile of ['full', 'pr']) {
    assert.throws(() => loadCanonicalCommands(directory, { profile }), /duplicate canonical PR/);
  }
  assert.throws(() => projectPrCommands({ ...fast,
    pr: { reason: 'No duplicate variant.', commands: [fast, fast] } }), /duplicate/);
  assert.throws(() => projectPrCommands({ ...fast,
    pr: { reason: 'No nested policy.', commands: [{ ...fast, pr: { reason: 'nested', commands: [] } }] } }),
  /invalid canonical command/);
});

test('a wholly master-only catalog has an explicit empty PR projection', t => {
  const { directory, put } = temporaryCatalog(t);
  const original = { command: 'node', args: ['slow.mjs'] };
  put('001000-slow.json', JSON.stringify({ ...original,
    pr: { reason: 'Master-only exhaustive experiment.', commands: [] } }));
  assert.deepEqual(loadCanonicalCommands(directory), [original]);
  assert.deepEqual(loadCanonicalCommands(directory, { profile: 'pr' }), []);
});

test('unknown profiles and extra CLI arguments cannot select a different inventory', () => {
  assert.equal(parseCheckProfile([]), 'full');
  assert.equal(parseCheckProfile(['--profile=full']), 'full');
  assert.equal(parseCheckProfile(['--profile=pr']), 'pr');
  for (const args of [null, ['pr'], ['--profile='], ['--profile=other'],
    ['--profile=pr', '--profile=full']]) assert.throws(() => parseCheckProfile(args), /usage/);
  assert.throws(() => loadCanonicalCommands(undefined, { profile: 'unknown' }), /unknown canonical check profile/);
});

test('PR test budgets are short and compilation or exact bounded aggregates are distinct', () => {
  const cases = [
    ['node', ['--test', 'fast.test.mjs'], 120000],
    ['cargo', ['test', '--workspace'], 120000],
    ['cargo', ['build', '--workspace'], 600000],
    ['cargo', ['clippy', '--all-targets'], 600000],
    ['cargo', ['test', '--no-run', '-p', 'small'], 600000],
    ['cargo', ['test', '--', '--no-run'], 120000],
    ['node', ['scripts/pr-cargo-tests.mjs'], 0],
    ['node', ['scripts/pr-cargo-tests.mjs', '--package', 'small', '--package', 'other'], 0],
    ['node', ['scripts/pr-cargo-tests.mjs', '--package'], 120000],
    ['node', ['scripts/pr-cargo-tests.mjs', '--other', 'small'], 120000],
    ['node', ['./scripts/pr-cargo-tests.mjs'], 120000],
    ['node', ['other-pr-cargo-tests.mjs'], 120000],
  ];
  for (const [command, args, expected] of cases) {
    assert.equal(commandTimeoutMs(command, args, 'pr'), expected, JSON.stringify([command, ...args]));
    assert.equal(commandTimeoutMs(command, args), 0, 'full profile keeps its original child execution policy');
  }
  assert.throws(() => commandTimeoutMs('node', ['x.mjs'], 'unknown'), /unknown canonical check profile/);
});


test('parallel master groups partition the entire canonical inventory exactly once', () => {
  const canonical = loadCanonicalCommands();
  const grouped = loadMasterCheckGroups(canonical);
  const keys = commands => commands.map(command => JSON.stringify([command.command, ...command.args]));
  assert.deepEqual(keys([...grouped.values()].flat()).sort(), keys(canonical).sort());
  assert.equal(grouped.size, 4);
  for (const checks of grouped.values()) assert.ok(checks.length > 0);
  assert.ok(grouped.get('rust-tests').some(c => c.command === 'cargo' && c.args[0] === 'test'));
  const rust = grouped.get('rust-tests');
  const spikeFetch = rust.findIndex(c => c.command === 'cargo' && c.args[0] === 'fetch'
    && c.args.includes('spikes/004-tsrs-type-query/Cargo.toml'));
  const spikeTests = rust.findIndex(c => c.command === 'cargo' && c.args[0] === 'test'
    && c.args.includes('spikes/004-tsrs-type-query/Cargo.toml'));
  assert.ok(spikeFetch >= 0 && spikeTests > spikeFetch,
    'offline spike tests must fetch their own dependencies first');
  assert.ok(grouped.get('style').some(c => c.command === 'cargo' && c.args[0] === 'clippy'));
  assert.ok(grouped.get('applications').some(c => c.args[0] === 'scripts/poc.mjs'));
  assert.ok(grouped.get('applications').some(c =>
    c.args.some(arg => arg.startsWith('tests/applications/'))));
});

test('master check partition refuses stale descriptors and unknown group tags', () => {
  const canonical = loadCanonicalCommands();
  const directory = fileURLToPath(new URL('./checks/', import.meta.url));
  const descriptors = readdirSync(directory).map(file =>
    JSON.parse(readFileSync(join(directory, file), 'utf8')));
  assert.throws(() => partitionMasterChecks(canonical, descriptors.slice(1)), /stale|unmapped/);
  const bad = structuredClone(descriptors);
  bad.find(d => d.id === 'check:workspace-tests').masterGroup = 'omitted';
  assert.throws(() => partitionMasterChecks(canonical, bad), /unknown master CI group/);
});
