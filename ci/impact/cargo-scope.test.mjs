import assert from 'node:assert/strict';
import test from 'node:test';
import { previewCargoScope } from './cargo-scope.mjs';

const packages = ['grunkalunka-cli', 'grunkalunka-compiler',
  'grunkalunka-runtime', 'grunkalunka-wasm-link', 'grunkalunka-layout'];

function impact({ full = false, tests = [], build = [], reasons = [] } = {}) {
  return { full, tests, build, reasons };
}

test('layout-only changes propose scoped Cargo tests rather than workspace tests', () => {
  const preview = previewCargoScope(impact({
    tests: ['cargo-test:grunkalunka-layout', 'check:workspace-tests',
      'check:workspace-clippy', 'check:workspace-format'],
    build: ['grunkalunka-layout'],
  }), packages);
  assert.equal(preview.mode, 'scoped-preview');
  assert.deepEqual(preview.selectedTestPackages, ['grunkalunka-layout']);
  assert.deepEqual(preview.commands.build, [
    'cargo', 'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout']);
  assert.deepEqual(preview.commands.test, [
    'cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout']);
  assert.deepEqual(preview.commands.format, ['cargo', 'fmt', '--all', '--check']);
});

test('downward build prerequisites never add their own tests', () => {
  const preview = previewCargoScope(impact({
    tests: ['cargo-test:grunkalunka-runtime', 'cargo-test:grunkalunka-cli',
      'check:workspace-tests'],
    build: ['grunkalunka-runtime', 'grunkalunka-cli',
      'grunkalunka-compiler', 'grunkalunka-wasm-link'],
  }), packages);
  assert.deepEqual(preview.selectedTestPackages,
    ['grunkalunka-cli', 'grunkalunka-runtime']);
  assert.deepEqual(preview.selectedBuildPackages, [
    'grunkalunka-cli', 'grunkalunka-compiler',
    'grunkalunka-runtime', 'grunkalunka-wasm-link']);
  assert.ok(!preview.commands.test.includes('grunkalunka-wasm-link'));
  assert.ok(!preview.commands.test.includes('grunkalunka-compiler'));
});

test('non-Cargo Node checks may require Cargo builds but no Cargo tests', () => {
  const preview = previewCargoScope(impact({
    tests: ['check:node-compile-integration'], build: ['node-build', 'grunkalunka-cli'],
  }), packages);
  assert.equal(preview.mode, 'scoped-preview');
  assert.deepEqual(preview.selectedBuildPackages, ['grunkalunka-cli']);
  assert.equal(preview.commands.test, null);
  assert.equal(preview.commands.clippy, null);
});

test('only explicitly ignored non-code files produce an empty preview', () => {
  const preview = previewCargoScope(impact({ tests: [], build: [] }), packages);
  assert.equal(preview.mode, 'scoped-preview');
  assert.deepEqual(preview.commands,
    { format: null, build: null, clippy: null, test: null });
});

test('existing full fallback remains authoritative', () => {
  const preview = previewCargoScope(impact({
    full: true, reasons: ['unattributed code change: unknown.rs'],
  }), packages);
  assert.equal(preview.mode, 'full');
  assert.match(preview.reason, /unattributed code change/);
  assert.equal(preview.commands, null);
});

test('unregistered Cargo test package never gets silently dropped', () => {
  const preview = previewCargoScope(impact({
    tests: ['cargo-test:not-in-workspace', 'check:workspace-tests'],
    build: ['not-in-workspace'],
  }), packages);
  assert.equal(preview.mode, 'full');
  assert.match(preview.reason, /unknown Cargo test package/);
});

test('malformed, incomplete and inconsistent package selections fail closed', () => {
  const cases = [
    [impact({ tests: ['cargo-test:grunkalunka-layout'], build: ['grunkalunka-layout'] }), packages],
    [impact({ tests: ['check:workspace-tests'], build: ['grunkalunka-layout'] }), packages],
    [impact({ tests: ['cargo-test:grunkalunka-layout', 'check:workspace-tests'], build: [] }), packages],
    [impact({ tests: ['test:description-only'], build: [] }), packages],
    [impact({ tests: [null], build: [] }), packages],
    [impact({ tests: [], build: [false] }), packages],
    [impact({ tests: [], build: [] }), [...packages, 'grunkalunka-layout']],
    [impact({ tests: [], build: [] }), []],
  ];
  for (const [selection, p] of cases) {
    assert.equal(previewCargoScope(selection, p).mode, 'full');
  }
});
