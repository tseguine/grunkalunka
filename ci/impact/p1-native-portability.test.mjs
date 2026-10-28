import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { planImpact } from './impact.mjs';

const read = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const native = read('./checks/p1-native-portability.json');
const target = read('./targets/p1-native-portability.json');
const modules = [
  { id: 'grunkalunka-cli', paths: ['crates/grunkalunka-cli/'],
    dependsOn: ['grunkalunka-compiler'] },
  { id: 'grunkalunka-compiler', paths: ['crates/grunkalunka-compiler/'] },
  { id: 'grunkalunka-runtime', paths: ['crates/grunkalunka-runtime/'] },
  { id: 'example-workflows', paths: ['examples/'], dependsOn: ['grunkalunka-cli'] },
  { id: 'node-bench', paths: ['tools/bench/'],
    dependsOn: ['grunkalunka-runtime', 'grunkalunka-cli'] },
];
const select = path => planImpact({
  changedPaths: [path], modules, groups: [target], tests: [native],
});

test('portability test-only change selects CLI prerequisite, not a full workspace', () => {
  const result = select('tests/build/p1-native-portability.test.mjs');
  assert.equal(result.full, false);
  assert.deepEqual(result.tests, ['check:p1-native-portability']);
  assert.deepEqual(result.build, ['grunkalunka-cli', 'grunkalunka-compiler']);
});

test('compiler or affine source changes select the real native portability test', () => {
  for (const path of ['crates/grunkalunka-compiler/src/lib.rs',
    'crates/grunkalunka-cli/src/main.rs', 'examples/numeric-workflow/affine.js',
    'tools/bench/native-build-identity.mjs']) {
    const result = select(path);
    assert.equal(result.full, false, path + ': ' + result.reasons);
    assert.ok(result.tests.includes('check:p1-native-portability'), path);
  }
});

test('native portability has one real canonical test command and explicit CLI build input', () => {
  assert.deepEqual(native.args, ['--test', 'tests/build/p1-native-portability.test.mjs']);
  assert.deepEqual(target.buildDependsOn, ['grunkalunka-cli']);
});
