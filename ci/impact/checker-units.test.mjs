// Selection uses the live descriptor graph; the unit command must not inherit
// the native prerequisites of the source checks whose decisions it exercises.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { loadCanonicalCommands } from '../../scripts/check-catalog.mjs';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { checkNodes } from './canonical.mjs';
import { cargoModules, planImpact } from './impact.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const json = file => JSON.parse(readFileSync(new URL(file, import.meta.url), 'utf8'));
const catalog = directory => readdirSync(new URL(directory, import.meta.url))
  .filter(file => file.endsWith('.json')).sort().map(file => json(`${directory}/${file}`));
const declared = catalog('./checks');
const unit = declared.find(check => check.id === 'check:checker-units');
const metadata = JSON.parse(execFileSync('cargo',
  ['metadata', '--format-version', '1', '--no-deps', '--offline'],
  { cwd: root, encoding: 'utf8', timeout: 10_000, stdio: ['ignore', 'pipe', 'pipe'] }));
const workspace = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
const cargoTests = cargoTestNodes(workspace, catalog('./cargo-tests'));
const modules = [...cargoModules(metadata, root), ...catalog('./owners')];
const groups = catalog('./targets');
const tests = [...cargoTests, ...checkNodes(declared, workspace.map(pkg => pkg.name),
  cargoTests.map(check => check.id))];
const plan = path => planImpact({ changedPaths: [path], modules, groups, tests });

test('checker units run once and unfiltered in both profiles', () => {
  assert.deepEqual(unit.paths, ['tests/checks/']);
  assert.deepEqual(unit.buildDependsOn, []);
  assert.deepEqual(unit.args, ['--test', 'tests/checks/*.test.mjs']);
  for (const profile of ['full', 'pr']) {
    const commands = loadCanonicalCommands(undefined, { profile });
    assert.deepEqual(commands.filter(command => command.args.includes('tests/checks/*.test.mjs')),
      [{ command: unit.command, args: unit.args }]);
    assert.equal(commands.filter(command => command.args.includes('ci/impact/*.test.mjs')).length, 1);
    const native = commands.filter(command => command.args.includes('scripts/check-document-outline.mjs'));
    assert.deepEqual(native, [{ command: 'node', args: profile === 'pr'
      ? ['scripts/check-document-outline.mjs', '--pr'] : ['scripts/check-document-outline.mjs'] }]);
  }
});

for (const path of ['tests/checks/document-outline-profile.test.mjs',
  'tests/checks/document-outline-reports.test.mjs', 'tests/checks/future-fixture.mjs']) {
  test(`checker-only changes need no native build: ${path}`, () => {
    const result = plan(path);
    assert.equal(result.full, false, result.reasons.join('; '));
    assert.deepEqual(result.changedOwners, [unit.id]);
    assert.deepEqual(result.tests, [unit.id]);
    assert.deepEqual(result.build, []);
  });
}

test('real document-check edits retain units and the existing native feature checks', () => {
  const result = plan('scripts/check-document-outline.mjs');
  assert.equal(result.full, false, result.reasons.join('; '));
  assert.ok(result.tests.includes(unit.id));
  assert.ok(result.tests.includes('check:document-outline'));
  assert.ok(!result.tests.includes('check:impact-planner-regressions'));
  assert.ok(result.build.includes('grunkalunka-runtime'));
  assert.ok(result.build.includes('grunkalunka-markdown'));
});

test('unknown source paths still require full verification', () => {
  const result = plan('unregistered-checker/input.mjs');
  assert.equal(result.full, true);
  assert.match(result.reasons.join('; '), /unattributed code change/);
});

test('missing unit registration cannot silently omit changed checker tests', () => {
  const result = planImpact({ changedPaths: ['tests/checks/document-outline-profile.test.mjs'],
    modules, groups, tests: tests.filter(check => check.id !== unit.id) });
  assert.equal(result.full, true);
  assert.match(result.reasons.join('; '), /unattributed code change/);
});
