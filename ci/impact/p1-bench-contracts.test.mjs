import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { planImpact } from './impact.mjs';

const read = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const group = read('./targets/p1-bench-contracts.json');
const check = read('./checks/p1-bench-tests.json');
const unitOwner = read('./owners/p1-bench-unit-tests.json');
const wrapper = read('./owners/p1-rustc-wrapper.json');
const p1Tools = read('./owners/p1-diagnostic-tools.json');
const modules = [
  unitOwner,
  wrapper,
  p1Tools,
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
  assert.deepEqual(result.build, ['p1-bench-unit-tests']); // owner node, no Cargo packages
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
  assert.deepEqual(result.build, ['p1-rustc-wrapper']); // owner node, not Cargo
});

test('P1 entrypoint is one stable canonical Node invocation with glob-discovered cases', () => {
  assert.equal(check.command, 'node');
  assert.deepEqual(check.args.slice(0, 2), ['--test', 'tools/bench/tests/*.test.mjs']);
  assert.ok(unitOwner.paths.includes('tools/bench/tests/'));
  assert.deepEqual(check.buildDependsOn, []);
});

test('P1-only diagnostic source changes skip unrelated Rust and benchmark work',()=>{
  for(const path of [
    'tools/bench/native-cargo-metadata-override.mjs',
    'tools/bench/markdown-cgu-control.mjs',
    'tools/bench/wasm-data-chunks.mjs',
    'tools/bench/wasm-section-fingerprint.mjs',
    'tools/bench/windows-eol-markdown.mjs',
  ]){
    assert.ok(p1Tools.paths.includes(path),path);
    const result=plan(path);
    assert.equal(result.full,false,path+': '+result.reasons);
    assert.deepEqual(result.tests,['check:p1-bench-tests'],path);
    assert.deepEqual(result.build,['p1-diagnostic-tools'],path);
  }
});

test('benchmark-facing helpers retain their original Cargo dependency selection',()=>{
  for(const path of ['tools/bench/portable-gcd.mjs',
    'tools/bench/core-engine-replay.mjs']){
    assert.ok(!p1Tools.paths.includes(path),path);
    const result=plan(path);
    assert.equal(result.full,false,path+': '+result.reasons);
    assert.ok(result.tests.includes('check:p1-bench-tests'),path);
    assert.ok(result.build.includes('grunkalunka-runtime'),path);
    assert.ok(result.build.includes('grunkalunka-cli'),path);
  }
});

test('diagnostic owner is exact-file only and does not shadow test/wrapper owners',()=>{
  assert.ok(p1Tools.paths.every(path=>path.startsWith('tools/bench/')&&
    path.endsWith('.mjs')&&!path.includes('/tests/')));
  assert.ok(!p1Tools.paths.includes('tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs'));
});
