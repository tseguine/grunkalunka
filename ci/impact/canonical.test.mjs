import assert from 'node:assert/strict';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { planImpact } from './impact.mjs';
import { previewCargoScope } from './cargo-scope.mjs';

const directory = fileURLToPath(new URL('./', import.meta.url));
const readCatalog = folder => readdirSync(join(directory, folder))
  .filter(name => name.endsWith('.json')).map(name =>
    JSON.parse(readFileSync(join(directory, folder, name), 'utf8')));
const declared = readCatalog('checks');

test('the real canonical runner can be enumerated without executing child checks', () => {
  const observed = JSON.parse(execFileSync(process.execPath, ['ci/impact/collect-checks.mjs'],
    { cwd: fileURLToPath(new URL('../../', import.meta.url)),
      encoding: 'utf8', timeout: 10000 }));
  assert.ok(observed.length > 0);
  // Drift is an advisory full-build fallback, never a second gate for other PRs.
  // The strict mismatch behavior is tested against controlled synthetic inputs.
  try {
    const coverage = verifyCanonicalChecks(observed, declared);
    assert.equal(coverage.commands, observed.length);
  } catch (error) {
    assert.match(error.message, /canonical/);
  }
});

test('missing, stale, duplicate, and malformed command declarations fail closed', () => {
  const actual = [{ command: 'node', args: ['a.mjs'] }, { command: 'cargo', args: ['test'] }];
  const expected = actual.map((cmd, index) => ({ id: 'check:' + index, ...cmd }));
  assert.equal(verifyCanonicalChecks(actual, expected).mapped, 2);
  for (const corrupted of [
    expected.slice(1), [...expected, { ...expected[0], id: 'check:extra' }],
    [{ ...expected[0], args: ['other.mjs'] }, expected[1]],
    [{ ...expected[0], args: [null] }, expected[1]],
    [{ ...expected[0], id: expected[1].id }, expected[1]],
  ]) assert.throws(() => verifyCanonicalChecks(actual, corrupted));
  assert.throws(() => verifyCanonicalChecks([...actual, actual[0]], expected), /duplicate/);
});

test('workspace commands inherit newly discovered Cargo package IDs', () => {
  const expanded = checkNodes(declared, ['existing', 'brand-new']);
  for (const id of ['check:workspace-format', 'check:workspace-clippy', 'check:workspace-tests']) {
    const check = expanded.find(check => check.id === id);
    assert.deepEqual(check.dependsOn, ['existing', 'brand-new']);
    assert.deepEqual(check.buildDependsOn, [],
      'workspace-wide checks are virtual umbrellas, not full-build prerequisites');
  }
});

test('workspace-test wrapper follows Cargo test nodes with external fixture inputs', () => {
  const extended = checkNodes(declared, ['grunkalunka-runtime', 'grunkalunka-layout'],
    ['cargo-test:grunkalunka-runtime', 'cargo-test:grunkalunka-layout']);
  const workspace = extended.find(node => node.id === 'check:workspace-tests');
  assert.deepEqual(workspace.dependsOn, [
    'grunkalunka-runtime', 'grunkalunka-layout',
    'cargo-test:grunkalunka-runtime', 'cargo-test:grunkalunka-layout',
  ]);
  assert.deepEqual(workspace.buildDependsOn, []);
  const formatting = extended.find(node => node.id === 'check:workspace-format');
  assert.deepEqual(formatting.dependsOn, ['grunkalunka-runtime', 'grunkalunka-layout']);
  assert.throws(() => checkNodes(declared, ['grunkalunka-runtime'],
    ['cargo-test:grunkalunka-runtime', 'cargo-test:grunkalunka-runtime']), /Cargo/);
});

const cargo = ['grunkalunka-cli', 'grunkalunka-compiler', 'grunkalunka-runtime',
  'grunkalunka-json', 'grunkalunka-json-guest', 'grunkalunka-wasm-link',
  'grunkalunka-markdown', 'grunkalunka-layout', 'grunkalunka-string-spike'];
const localDeps = {
  'grunkalunka-cli': ['grunkalunka-compiler', 'grunkalunka-runtime'],
  'grunkalunka-runtime': ['grunkalunka-compiler', 'grunkalunka-wasm-link'],
  'grunkalunka-json-guest': ['grunkalunka-json'],
};
const modules = [...cargo.map(id => ({ id, paths: [
  id === 'grunkalunka-string-spike' ? 'spikes/003-lossless-string-literals/' :
    'crates/' + id + '/'], dependsOn: localDeps[id] ?? [] })),
  ...readCatalog('owners')];
const groups = readCatalog('targets');
const tests = [...cargo.map(id => ({ id: 'cargo-test:' + id, dependsOn: [id] })),
  ...checkNodes(declared, cargo)];
const analyze = paths => planImpact({
  changedPaths: paths, modules, groups, tests, global: ['ci/impact/'], ignored: ['docs/'],
});

test('isolated layout code selects workspace checks but no unrelated integrations', () => {
  const result = analyze(['crates/grunkalunka-layout/src/lib.rs']);
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.ok(result.tests.includes('check:workspace-tests'));
  assert.ok(result.tests.includes('cargo-test:grunkalunka-layout'));
  assert.ok(!result.tests.includes('check:compiler-compatibility-replay'));
  assert.ok(!result.tests.includes('check:json-composition'));
});

test('specific test/script ownership overrides broad directory ownership', () => {
  const result = analyze(['scripts/check-json-composition.mjs']);
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.ok(result.tests.includes('check:json-composition'));
  assert.ok(!result.tests.includes('check:compiler-compatibility-replay'));
  assert.ok(result.build.includes('grunkalunka-json-guest'));
});

test('unknown code paths force full verification alongside attributed files', () => {
  const result = analyze(['crates/grunkalunka-layout/src/lib.rs',
    'spikes/004-tsrs-type-query/src/main.rs']);
  assert.equal(result.full, true);
  assert.match(result.reasons[0], /unattributed/);
});

test('a non-Cargo owner with only a descriptive group fails closed', () => {
  const result = analyze(['tools/browser-oracles/smoke.mjs']);
  assert.equal(result.full, true);
  assert.match(result.reasons[0], /no executable test covers changed owner: browser-oracles/);
});

test('an actual canonical check is reached through a logical group', () => {
  const result = analyze(['crates/grunkalunka-json/src/lib.rs']);
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.ok(result.tests.includes('check:json-composition'));
  assert.ok(!result.tests.includes('test:json-integration'));
});

test('real layout change selects package tests without building the whole workspace', () => {
  const result = analyze(['crates/grunkalunka-layout/src/lib.rs']);
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.deepEqual(result.build, ['grunkalunka-layout']);
  assert.ok(result.tests.includes('check:workspace-tests'));
  assert.ok(result.tests.includes('cargo-test:grunkalunka-layout'));
  const preview = previewCargoScope(result, cargo);
  assert.equal(preview.mode, 'scoped-preview', preview.reason);
  assert.deepEqual(preview.selectedBuildPackages, ['grunkalunka-layout']);
  assert.deepEqual(preview.selectedTestPackages, ['grunkalunka-layout']);
  assert.deepEqual(preview.commands.test,
    ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout']);
});
