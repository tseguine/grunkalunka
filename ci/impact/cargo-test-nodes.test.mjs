import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { planImpact } from './impact.mjs';
import { checkNodes } from './canonical.mjs';
import { previewCargoScope } from './cargo-scope.mjs';

const runtimeInputs = JSON.parse(readFileSync(
  new URL('./cargo-tests/grunkalunka-runtime.json', import.meta.url), 'utf8'));
const packages = [{ name: 'grunkalunka-runtime' }, { name: 'grunkalunka-layout' }];
const declared = [
  { id: 'check:workspace-tests', command: 'cargo',
    args: ['test', '--workspace', '--locked', '--offline', '-j', '4'],
    scope: 'workspace' },
];

function simulate(changedPath) {
  const cargo = cargoTestNodes(packages, [runtimeInputs]);
  const checks = checkNodes(declared, packages.map(p => p.name), cargo.map(t => t.id));
  return planImpact({
    changedPaths: [changedPath],
    modules: [
      { id: 'grunkalunka-runtime', paths: ['crates/grunkalunka-runtime/'] },
      { id: 'grunkalunka-layout', paths: ['crates/grunkalunka-layout/'] },
      { id: 'compatibility-fixtures', paths: ['tests/compatibility/'] },
      { id: 'example-workflows', paths: ['examples/'] },
    ],
    tests: [...cargo, ...checks],
  });
}

test('external Rust include_str fixtures select the runtime package test on isolated changes', () => {
  for (const path of [
    'tests/compatibility/fixtures/affine.js',
    'tests/compatibility/fixtures/affine.ts',
    'examples/numeric-workflow/gcd.js',
  ]) {
    const impact = simulate(path);
    assert.equal(impact.full, false, JSON.stringify(impact.reasons));
    assert.ok(impact.tests.includes('cargo-test:grunkalunka-runtime'), path);
    assert.ok(impact.tests.includes('check:workspace-tests'), path);
    assert.ok(!impact.tests.includes('cargo-test:grunkalunka-layout'));
    const preview = previewCargoScope(impact, packages.map(p => p.name));
    assert.equal(preview.mode, 'scoped-preview', preview.reason);
    assert.deepEqual(preview.selectedTestPackages, ['grunkalunka-runtime']);
    assert.ok(preview.commands.test.includes('grunkalunka-runtime'));
  }
});

test('changing an unrelated package does not acquire runtime Cargo tests', () => {
  const impact = simulate('crates/grunkalunka-layout/src/lib.rs');
  assert.equal(impact.full, false);
  assert.deepEqual(impact.tests, ['cargo-test:grunkalunka-layout', 'check:workspace-tests']);
  assert.deepEqual(previewCargoScope(impact, packages.map(p => p.name)).selectedTestPackages,
    ['grunkalunka-layout']);
});

test('stale, ambiguous or duplicate package-input declarations fail closed', () => {
  for (const extras of [
    [{ package: 'not-a-package', dependsOn: ['compatibility-fixtures'] }],
    [runtimeInputs, runtimeInputs],
    [{ package: 'grunkalunka-runtime', dependsOn: [] }],
    [{ package: 'grunkalunka-runtime', dependsOn: ['compatibility-fixtures',
      'compatibility-fixtures'] }],
    [{ package: 'grunkalunka-runtime', dependsOn: ['grunkalunka-runtime'] }],
    [{ package: 'grunkalunka-runtime', dependsOn: ['compatibility-fixtures'],
      ignored: true }],
  ]) {
    assert.throws(() => cargoTestNodes(packages, extras));
  }
});

test('externally discovered inputs are selection edges only, not build edges', () => {
  const entries = cargoTestNodes(packages, [runtimeInputs]);
  const runtime = entries.find(t => t.id === 'cargo-test:grunkalunka-runtime');
  assert.deepEqual(runtime.dependsOn, ['grunkalunka-runtime',
    'compatibility-fixtures', 'example-workflows']);
  assert.deepEqual(runtime.buildDependsOn, ['grunkalunka-runtime']);
});
