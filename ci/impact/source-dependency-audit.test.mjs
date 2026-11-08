import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { cargoModules, planImpact } from './impact.mjs';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { checkNodes } from './canonical.mjs';
import { discoverLiteralSourceInputs, auditSourceSelection } from './source-dependency-audit.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const directory = fileURLToPath(new URL('./', import.meta.url));
const catalog = name => readdirSync(join(directory, name))
  .filter(file => file.endsWith('.json')).sort()
  .map(file => JSON.parse(readFileSync(join(directory, name, file), 'utf8')));
const specs = catalog('source-audits');
const metadata = JSON.parse(execFileSync('cargo',
  ['metadata', '--format-version', '1', '--no-deps', '--offline'],
  { cwd: root, encoding: 'utf8', timeout: 20000, maxBuffer: 16 * 1024 * 1024 }));
const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
const modules = [...cargoModules(metadata, root), ...catalog('owners')];
const groups = catalog('targets');
const cargoTests = cargoTestNodes(packages, catalog('cargo-tests'));
const declared = catalog('checks');
const tests = [
  ...cargoTests,
  ...checkNodes(declared, packages.map(pkg => pkg.name), cargoTests.map(x => x.id)),
];
const plan = (file, overriddenGroups = groups, overriddenTests = tests) => planImpact({
  changedPaths: [file], modules, groups: overriddenGroups, tests: overriddenTests,
  global: ['.github/', 'ci/impact/', 'Cargo.toml', 'Cargo.lock', 'scripts/checks/'],
  ignored: ['docs/', 'README.md', 'LICENSE', 'AGENTS.md'],
});
const audit = (selectedSpecs = specs, choose = file => plan(file)) =>
  auditSourceSelection({
    specs: selectedSpecs,
    readSource: source => readFileSync(join(root, source), 'utf8'),
    exists: file => existsSync(join(root, file)),
    planForPath: choose,
  });

test('literal Node imports, dynamic imports and source URLs resolve independently', () => {
  const source = [
    "import { foo } from './foo.mjs';",
    "import './side-effect.mjs';",
    "const bar = await import('../build/compile.mjs');",
    "const fixtures = new URL('../../examples/test.js', import.meta.url,);",
    "const worker = new URL('./worker.mjs', import.meta.url);",
    "import 'node:fs';",
  ].join('\n');
  const discovered = discoverLiteralSourceInputs('tools/bench/numeric.mjs', source);
  assert.deepEqual(discovered.inputs, [
    'examples/test.js',
    'tools/bench/foo.mjs',
    'tools/bench/side-effect.mjs',
    'tools/bench/worker.mjs',
    'tools/build/compile.mjs',
  ]);
  assert.deepEqual(discovered.unresolved, []);
});


test('import attributes retain literal input edges and computed suffixes fail closed', () => {
  const source = [
    "const json = await import('./settings.json', { with: { type: 'json' } });",
    "const local = import('../shared/keys.mjs', attributes);",
    "const builtin = await import('node:fs', { with: { type: 'json' } });",
  ].join('\n');
  const discovered = discoverLiteralSourceInputs('tools/bench/loader.mjs', source);
  assert.deepEqual(discovered.inputs, [
    'tools/bench/settings.json',
    'tools/shared/keys.mjs',
  ]);
  assert.deepEqual(discovered.unresolved, []);

  const concatenated = discoverLiteralSourceInputs('tools/bench/loader.mjs',
    "const result = import('./prefix/' + suffix);");
  assert.deepEqual(concatenated.inputs, []);
  assert.deepEqual(concatenated.unresolved, [
    'computed import expression: tools/bench/loader.mjs',
  ]);
});

test('an import with attributes contributes to the independent impact audit', () => {
  const options = {
    specs: [{ source: 'tools/bench/loader.mjs',
      requiredChecks: ['check:benchmark-tests'] }],
    readSource: () => "await import('./settings.json', { with: { type: 'json' } });",
    exists: () => true,
  };
  const omitted = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: [] }),
  });
  assert.equal(omitted.valid, false);
  assert.deepEqual(omitted.edges, [{
    consumer: 'tools/bench/loader.mjs',
    dependency: 'tools/bench/settings.json',
    outcome: 'under-selected',
  }]);
  assert.match(omitted.errors.join('\n'), /missing check:benchmark-tests/);
  const covered = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: ['check:benchmark-tests'] }),
  });
  assert.equal(covered.valid, true, covered.errors.join('\n'));
});

test('literal Rust includes and path attributes resolve independently', () => {
  const source = [
    'const B: &[u8] = include_bytes!("../../../examples/numeric-workflow/gcd.js");',
    'const S: &str = include_str!("../../../tests/compatibility/fixtures/affine.js");',
    '#[path = "../examples/source_compile_cost.rs"]',
    'mod cost;',
  ].join('\n');
  const discovered = discoverLiteralSourceInputs(
    'crates/grunkalunka-runtime/tests/source_compile_cost_probe.rs', source);
  assert.deepEqual(discovered.inputs, [
    'crates/grunkalunka-runtime/examples/source_compile_cost.rs',
    'examples/numeric-workflow/gcd.js',
    'tests/compatibility/fixtures/affine.js',
  ]);
  assert.deepEqual(discovered.unresolved, []);
});

test('computed imports and nonliteral Rust includes are explicitly unresolved', () => {
  assert.ok(discoverLiteralSourceInputs('tools/bench/probe.mjs',
    "const mod = await import(userPath);").unresolved.some(x => x.includes('computed')));
  assert.ok(discoverLiteralSourceInputs('tools/bench/probe.mjs',
    "const file = new URL(sourcePath, import.meta.url);").unresolved.some(x =>
    x.includes('computed')));
  assert.ok(discoverLiteralSourceInputs('crates/grunkalunka-runtime/tests/probe.rs',
    'const X: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/file.js"));')
    .unresolved.some(x => x.includes('nonliteral')));
  assert.throws(() => discoverLiteralSourceInputs('tools/bench/probe.mjs',
    "import '../outside';".replace('../outside', '../../../outside')), /escapes repository/);
});

test('current benchmark and Rust fixture consumers have complete or full-fallback selection', () => {
  const result = audit();
  assert.equal(result.valid, true, JSON.stringify(result.errors));
  assert.equal(result.consumers, specs.length);
  assert.ok(result.edges.length >= 15, 'audited sources have real relative inputs');
  for (const expected of [
    ['tools/bench/numeric.mjs', 'tools/build/compile.mjs'],
    ['tools/bench/numeric.mjs', 'examples/numeric-workflow/gcd.js'],
    ['crates/grunkalunka-runtime/examples/source_compile_cost.rs',
      'tests/compatibility/fixtures/affine.js'],
    ['crates/grunkalunka-runtime/examples/core_engine_cost.rs',
      'examples/numeric-workflow/gcd.js'],
  ]) {
    assert.ok(result.edges.some(edge => edge.consumer === expected[0] &&
      edge.dependency === expected[1] && edge.outcome === 'selected'),
    'missing audited selection edge ' + expected.join(' -> '));
  }
});

test('removing a real benchmark source dependency edge is detected independently', () => {
  const altered = groups.map(group => group.id === 'test:benchmark-contracts'
    ? { ...group, dependsOn: group.dependsOn.filter(dep => dep !== 'node-build') } : group);
  const result = audit(specs.filter(s => s.source === 'tools/bench/numeric.mjs'),
    file => plan(file, altered));
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /tools\/build\/compile\.mjs.*check:benchmark-tests/);
});

test('removing a real Rust test fixture edge is detected independently', () => {
  const omitted = cargoTests.map(node => node.id === 'cargo-test:grunkalunka-runtime'
    ? { ...node, dependsOn: node.dependsOn.filter(dep => dep !== 'compatibility-fixtures') }
    : node);
  const partial = [...omitted,
    ...checkNodes(declared, packages.map(pkg => pkg.name), omitted.map(x => x.id))];
  const result = audit(specs.filter(s =>
    s.source === 'crates/grunkalunka-runtime/examples/source_compile_cost.rs'),
  file => plan(file, groups, partial));
  assert.equal(result.valid, false);
  assert.match(result.errors.join('\n'), /affine\.js.*cargo-test:grunkalunka-runtime/);
});

test('unknown source inputs and missing audits fail rather than silently succeed', () => {
  const source = 'tools/bench/numeric.mjs';
  const fail = auditSourceSelection({
    specs: [{ source, requiredChecks: ['check:benchmark-tests'] }],
    readSource: () => 'const m = await import(variable);',
    exists: () => true,
    planForPath: file => plan(file),
  });
  assert.equal(fail.valid, false);
  assert.match(fail.errors.join('\n'), /computed import/);
  assert.throws(() => auditSourceSelection({
    specs: [{ source, requiredChecks: [] }],
    readSource: () => '', exists: () => true, planForPath: file => plan(file),
  }), /invalid source audit/);
});

test('ESM re-exports are independent source inputs, including type-only exports', () => {
  const source = [
    "export { widget as view } from './view.mjs';",
    "export * from '../shared/helpers.mjs';",
    "export * as namespace from './namespace.mjs';",
    "export type { Named } from './types.ts';",
    "export { type Other } from './another.ts';",
    "export { local };",
    "export { external } from 'third-party';",
    "import { widget } from './view.mjs';",
  ].join('\n');
  const discovered = discoverLiteralSourceInputs('tools/bench/index.mjs', source);
  assert.deepEqual(discovered.inputs, [
    'tools/bench/another.ts',
    'tools/bench/namespace.mjs',
    'tools/bench/types.ts',
    'tools/bench/view.mjs',
    'tools/shared/helpers.mjs',
  ]);
  assert.deepEqual(discovered.unresolved, []);
});

test('re-exported source changes cannot silently drop the consumer check', () => {
  const spec = { source: 'tools/bench/index.mjs',
    requiredChecks: ['check:benchmark-tests'] };
  const options = {
    specs: [spec],
    readSource: () => "export * from './source.mjs';",
    exists: () => true,
  };
  const missed = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: [] }),
  });
  assert.equal(missed.valid, false);
  assert.deepEqual(missed.edges, [{
    consumer: 'tools/bench/index.mjs',
    dependency: 'tools/bench/source.mjs',
    outcome: 'under-selected',
  }]);
  assert.match(missed.errors.join('\n'), /missing check:benchmark-tests/);
  const selected = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: ['check:benchmark-tests'] }),
  });
  assert.equal(selected.valid, true, selected.errors.join('\n'));
});

test('literal Rust include! loads an audited local source while computed include! fails closed', () => {
  const source = [
    'include!("./generated/dispatch.rs");',
    'const S: &str = include_str!("../fixtures/labels.txt");',
    'const B: &[u8] = include_bytes!("../fixtures/payload.bin");',
    'include!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated.rs"));',
  ].join('\n');
  const result = discoverLiteralSourceInputs('crates/runtime/src/lib.rs', source);
  assert.deepEqual(result.inputs, [
    'crates/runtime/fixtures/labels.txt',
    'crates/runtime/fixtures/payload.bin',
    'crates/runtime/src/generated/dispatch.rs',
  ]);
  assert.deepEqual(result.unresolved, [
    'nonliteral Rust include: crates/runtime/src/lib.rs',
  ]);
});

test('Rust include! source edges participate in real check-coverage decisions', () => {
  const options = {
    specs: [{ source: 'crates/runtime/src/lib.rs',
      requiredChecks: ['cargo-test:grunkalunka-runtime'] }],
    readSource: () => 'include!("./generated/dispatch.rs");',
    exists: () => true,
  };
  const missing = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: [] }),
  });
  assert.equal(missing.valid, false);
  assert.deepEqual(missing.edges, [{
    consumer: 'crates/runtime/src/lib.rs',
    dependency: 'crates/runtime/src/generated/dispatch.rs',
    outcome: 'under-selected',
  }]);
  const selected = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: ['cargo-test:grunkalunka-runtime'] }),
  });
  assert.equal(selected.valid, true, selected.errors.join('\n'));
});

test('bare CommonJS requires contribute literal inputs and flag computed paths', () => {
  const source = [
    "const common = require('./common.cjs');",
    "require('../fixtures/config.json');",
    "require('./common.cjs');",
    "const fs = require('node:fs');",
    "const dynamic = require(fileName);",
    "object.require('./not-a-module.mjs');",
  ].join('\n');
  const found = discoverLiteralSourceInputs('tools/bench/loader.cjs', source);
  assert.deepEqual(found.inputs, [
    'tools/bench/common.cjs',
    'tools/fixtures/config.json',
  ]);
  assert.deepEqual(found.unresolved, [
    'computed require expression: tools/bench/loader.cjs',
  ]);
});

test('literal CommonJS dependencies cannot lose their consumer checks', () => {
  const options = {
    specs: [{ source: 'tools/bench/loader.cjs',
      requiredChecks: ['check:benchmark-tests'] }],
    readSource: () => "module.exports = require('./common.cjs');",
    exists: () => true,
  };
  const underSelected = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: [] }),
  });
  assert.equal(underSelected.valid, false);
  assert.match(underSelected.errors.join('\n'), /missing check:benchmark-tests/);
  const selected = auditSourceSelection({
    ...options,
    planForPath: () => ({ full: false, tests: ['check:benchmark-tests'] }),
  });
  assert.equal(selected.valid, true, selected.errors.join('\n'));
});
