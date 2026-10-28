import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { planImpact } from './impact.mjs';

const read = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const group = read('./targets/p1-bench-contracts.json');
const check = read('./checks/p1-bench-tests.json');
const wrapper = read('./owners/p1-rustc-wrapper.json');
const modules = [
  wrapper,
  { id: 'node-bench', paths: ['tools/bench/'],
    dependsOn: ['grunkalunka-runtime', 'grunkalunka-markdown', 'grunkalunka-cli'] },
  { id: 'build-test-sources', paths: ['tests/build/'] },
  { id: 'example-workflows', paths: ['examples/'], dependsOn: ['grunkalunka-cli'] },
  { id: 'grunkalunka-runtime', paths: ['crates/grunkalunka-runtime/'] },
  { id: 'grunkalunka-markdown', paths: ['crates/grunkalunka-markdown/'] },
  { id: 'grunkalunka-cli', paths: ['crates/grunkalunka-cli/'] },
];
const plan = path => planImpact({ changedPaths: [path], modules,
  groups: [group], tests: [check] });

test('P1 Node tests are selected for a changed test without requesting a Cargo build', () => {
  const result = plan('tools/bench/tests/markdown-cgu-control.test.mjs');
  assert.equal(result.full, false);
  assert.deepEqual(result.tests, ['check:p1-bench-tests']);
  assert.deepEqual(result.build, []);
});

test('P1 implementation, fixture and Markdown changes select the same canonical test', () => {
  for (const path of [
    'tools/bench/markdown-cgu-control.mjs',
    'tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs',
    'tests/build/fixtures/p1-rust-standalone/std_alloc.rs',
    'examples/numeric-workflow/affine.js',
    'crates/grunkalunka-markdown/src/lib.rs',
  ]) {
    const result = plan(path);
    assert.equal(result.full, false, `${path}: ${result.reasons}`);
    assert.ok(result.tests.includes('check:p1-bench-tests'), path);
  }
});

test('changing only the standalone Rust wrapper runs P1 tests without Cargo builds', () => {
  const result = plan('tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs');
  assert.equal(result.full, false);
  assert.deepEqual(result.tests, ['check:p1-bench-tests']);
  assert.deepEqual(result.build, []);
});

test('P1 entrypoint is one stable canonical Node invocation with glob-discovered cases', () => {
  assert.equal(check.command, 'node');
  assert.deepEqual(check.args.slice(0, 2), ['--test', 'tools/bench/tests/*.test.mjs']);
  assert.ok(check.paths.includes('tools/bench/tests/'));
  assert.deepEqual(check.buildDependsOn, []);
});
