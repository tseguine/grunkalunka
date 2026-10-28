import assert from 'node:assert/strict';
import test from 'node:test';
import { cargoModules, planImpact } from './impact.mjs';

const modules = [
  { id: 'core', paths: ['crates/core/'], dependsOn: ['utility'] },
  { id: 'api', paths: ['crates/api/'], dependsOn: ['core'] },
  { id: 'cli', paths: ['crates/cli/'], dependsOn: ['api'] },
  { id: 'utility', paths: ['crates/utility/'] },
];
const tests = [
  { id: 'core-tests', dependsOn: ['core'] },
  { id: 'api-tests', paths: ['tests/api/'], dependsOn: ['api'] },
  { id: 'cli-tests', dependsOn: ['cli'] },
  { id: 'utility-tests', dependsOn: ['utility'] },
];
const analyze = paths => planImpact({ changedPaths: paths, modules, tests,
  ignored: ['docs/', 'README.md'], global: ['Cargo.lock', '.github/', 'ci/impact/'] });

test('reverse closure picks tests; forward closure only adds build prerequisites', () => {
  const result = analyze(['crates/core/src/lib.rs']);
  assert.equal(result.full, false);
  assert.deepEqual(result.tests, ['api-tests', 'cli-tests', 'core-tests']);
  assert.deepEqual(result.build, ['api', 'cli', 'core', 'utility']);
  assert.ok(!result.tests.includes('utility-tests'), 'build-only deps never acquire tests');
});

test('test source changes select their own target and its build dependencies', () => {
  const result = analyze(['tests/api/contract.test.mjs']);
  assert.deepEqual(result.tests, ['api-tests']);
  assert.deepEqual(result.build, ['api', 'core', 'utility']);
});

test('ignore explicit non-code files without assuming arbitrary unknown paths are non-code', () => {
  const result = analyze(['README.md', 'docs/architecture.md']);
  assert.deepEqual(result.tests, []);
  assert.deepEqual(result.build, []);
  assert.deepEqual(result.ignoredPaths, ['README.md', 'docs/architecture.md']);
  assert.match(analyze(['future/special.rs']).reasons[0], /unattributed code change/);
});

test('global inputs always fall back to full verification', () => {
  for (const path of ['Cargo.lock', 'crates/core/Cargo.toml', 'spikes/001-frontend-semantics/Cargo.lock',
    '.github/workflows/ci.yml', 'ci/impact/owners/core.json']) {
    assert.equal(analyze([path]).full, true);
  }
});

test('ambiguous ownership and unknown declared dependencies fail closed', () => {
  const ambiguous = planImpact({ changedPaths: ['crates/core/src/lib.rs'],
    modules: [...modules, { id: 'shadow', paths: ['crates/core/'] }], tests });
  assert.equal(ambiguous.full, true);
  assert.match(ambiguous.reasons[0], /ambiguous/);
  const missing = planImpact({ changedPaths: ['crates/core/src/lib.rs'],
    modules: [...modules, { id: 'new', dependsOn: ['missing'] }], tests });
  assert.equal(missing.full, true);
  assert.match(missing.reasons[0], /unknown dependency/);
});

test('Cargo metadata attributes packages and follows dev-dependency edges', () => {
  const root = '/tmp/example';
  const metadata = {
    workspace_members: ['core-id', 'runtime-id', 'cli-id'], packages: [
      { id: 'core-id', name: 'core', manifest_path: `${root}/crates/core/Cargo.toml`, dependencies: [] },
      { id: 'runtime-id', name: 'runtime', manifest_path: `${root}/crates/runtime/Cargo.toml`,
        dependencies: [{ name: 'core', path: `${root}/crates/core`, kind: 'dev' }] },
      { id: 'cli-id', name: 'cli', manifest_path: `${root}/crates/cli/Cargo.toml`,
        dependencies: [{ name: 'runtime', path: `${root}/crates/runtime`, kind: null }] },
    ],
  };
  const cargo = cargoModules(metadata, root);
  const result = planImpact({ changedPaths: ['crates/core/src/lib.rs'], modules: cargo,
    tests: cargo.map(pkg => ({ id: `test:${pkg.id}`, dependsOn: [pkg.id] })) });
  assert.deepEqual(result.tests, ['test:cli', 'test:core', 'test:runtime']);
  assert.deepEqual(result.build, ['cli', 'core', 'runtime']);
});

test('invalid path, duplicated IDs, or malformed ownership cause full fallback', () => {
  assert.equal(analyze(['../src/lib.rs']).full, true);
  assert.equal(planImpact({ changedPaths: [], modules, tests: [{ id: 'core', dependsOn: [] }] }).full, true);
  assert.equal(planImpact({ changedPaths: [], modules: [{ id: 'bad', paths: ['../'] }], tests: [] }).full, true);
});

import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

function descriptors(folder) {
  const dir = fileURLToPath(new URL(`${folder}/`, import.meta.url));
  return readdirSync(dir).filter(name => name.endsWith('.json'))
    .map(name => JSON.parse(readFileSync(join(dir, name), 'utf8')));
}

test('checked-in metadata has complete references to known Rust modules', () => {
  const rust = ['grunkalunka-cli', 'grunkalunka-compiler', 'grunkalunka-json',
    'grunkalunka-json-guest', 'grunkalunka-layout', 'grunkalunka-markdown',
    'grunkalunka-runtime', 'grunkalunka-wasm-link'];
  const catalog = [...descriptors('owners'), ...rust.map(id => ({ id }))];
  const targets = descriptors('targets');
  const checks = descriptors('checks');
  const result = planImpact({ changedPaths: ['crates/grunkalunka-markdown/src/lib.rs'],
    modules: catalog.map(node => node.paths ? node : { ...node, paths: [`crates/${node.id}/`] }),
    groups: targets, tests: checks });
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.ok(result.tests.includes('check:markdown-portability'));
  assert.ok(!result.tests.includes('check:json-composition'));
});

test('owned code without any reachable test triggers full verification', () => {
  const orphan = { id: 'new-module', paths: ['modules/new/'] };
  const noTests = planImpact({
    changedPaths: ['modules/new/new.js'], modules: [...modules, orphan], tests,
  });
  assert.equal(noTests.full, true);
  assert.match(noTests.reasons[0], /no executable test covers changed owner: new-module/);
});

test('an untested owner fails closed even when another changed owner has tests', () => {
  const orphan = { id: 'new-module', paths: ['modules/new/'] };
  const mixed = planImpact({
    changedPaths: ['crates/core/src/lib.rs', 'modules/new/new.js'],
    modules: [...modules, orphan], tests,
  });
  assert.equal(mixed.full, true);
  assert.match(mixed.reasons[0], /no executable test covers changed owner: new-module/);
});

test('an owned test source is itself a selected test, not an orphaned module', () => {
  const selected = analyze(['tests/api/contract.test.mjs']);
  assert.equal(selected.full, false);
  assert.ok(selected.tests.includes('api-tests'));
});

test('logical groups are dependency junctions, never runnable test targets', () => {
  const orphan = { id: 'new-module', paths: ['modules/new/'] };
  const advisoryGroup = { id: 'group:new-module', dependsOn: ['new-module'] };
  const missing = planImpact({
    changedPaths: ['modules/new/new.js'], modules: [...modules, orphan],
    groups: [advisoryGroup], tests,
  });
  assert.equal(missing.full, true);
  assert.match(missing.reasons[0], /no executable test covers changed owner: new-module/);
  const covered = planImpact({
    changedPaths: ['modules/new/new.js'], modules: [...modules, orphan],
    groups: [advisoryGroup],
    tests: [...tests, { id: 'check:new-module', dependsOn: [advisoryGroup.id] }],
  });
  assert.equal(covered.full, false, JSON.stringify(covered.reasons));
  assert.ok(covered.tests.includes('check:new-module'));
  assert.ok(!covered.tests.includes(advisoryGroup.id));
});

test('separate selection and build edges do not import unrelated workspace modules', () => {
  const virtual = { id: 'all-workspace-tests', dependsOn: ['core', 'api', 'cli', 'utility'],
    buildDependsOn: [] };
  const result = planImpact({
    changedPaths: ['crates/core/src/lib.rs'],
    modules, tests: [...tests, virtual],
  });
  assert.equal(result.full, false, JSON.stringify(result.reasons));
  assert.ok(result.tests.includes('all-workspace-tests'));
  assert.deepEqual(result.build, ['api', 'cli', 'core', 'utility'],
    'other actually affected tests may still bring all required dependencies');
  const isolated = planImpact({
    changedPaths: ['crates/api/src/api.rs'],
    modules, tests: [
      { id: 'api-tests', dependsOn: ['api'] },
      virtual,
    ],
  });
  assert.equal(isolated.full, false, JSON.stringify(isolated.reasons));
  assert.deepEqual(isolated.build, ['api', 'core', 'utility'],
    'unaffected CLI must not be built by a virtual workspace check');
  assert.deepEqual(isolated.tests, ['all-workspace-tests', 'api-tests']);
});

test('unknown or malformed build-only edges trigger conservative full fallback', () => {
  const broken = [
    { id: 'all', dependsOn: ['core'], buildDependsOn: ['nonexistent'] },
    { id: 'all', dependsOn: ['core'], buildDependsOn: 'core' },
    { id: 'all', dependsOn: ['core'], buildDependsOn: [null] },
  ];
  for (const target of broken) {
    const result = planImpact({
      changedPaths: ['crates/core/src/lib.rs'], modules, tests: [...tests, target],
    });
    assert.equal(result.full, true, JSON.stringify(target));
  }
});
