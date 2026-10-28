// Offline browser test registration is a canonical command, not an extra
// GitHub Actions workflow. The remaining hosted workflows collect real evidence.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';
import { loadCanonicalCommands } from '../../scripts/check-catalog.mjs';

const read = path => readFileSync(new URL('../../' + path, import.meta.url), 'utf8');

test('offline browser tests are wired into canonical full and partial CI', () => {
  const runner = read('scripts/check.mjs');
  const manifest = JSON.parse(read('scripts/checks/004000-browser-oracle-contracts.json'));
  const descriptor = JSON.parse(read('ci/impact/checks/browser-oracle-contracts.json'));
  assert.equal(descriptor.id, 'check:browser-oracle-contracts');
  assert.equal(descriptor.command, 'node');
  assert.deepEqual(descriptor.args, ['scripts/check-browser-oracles.mjs']);
  assert.ok(descriptor.dependsOn.includes('test:browser-oracle-contracts'));
  assert.ok(runner.includes('loadCanonicalCommands()'),
    'the full CI runner must execute the manifest catalog');
  assert.deepEqual(manifest, {
    command: 'node', args: ['scripts/check-browser-oracles.mjs'],
  });
  const browserCommands = loadCanonicalCommands().filter(({ command, args }) =>
    command === manifest.command && JSON.stringify(args) === JSON.stringify(manifest.args));
  assert.equal(browserCommands.length, 1,
    'full CI must register the same offline browser command exactly once');

  const group = JSON.parse(read('ci/impact/targets/browser-oracle-contracts.json'));
  assert.ok(group.dependsOn.includes('browser-oracles'));
  const source = read('scripts/check-browser-oracles.mjs');
  assert.ok(source.includes("'tests/browser-oracles'"));
  assert.ok(source.includes("'tools/browser-oracles'"));
  assert.ok(source.includes("entry.name.endsWith('.test.mjs')"));
  assert.ok(source.includes("['--test', ...files]"));
});

test('retired offline-only workflows cannot silently bypass canonical testing', () => {
  for (const path of [
    '.github/workflows/browser-collector.yml',
    '.github/workflows/browser-oracles.yml',
    '.github/workflows/impact-browser-mixed.yml',
    'ci/impact/workflows/browser-collector.json',
    'ci/impact/workflows/browser-oracles.json',
  ]) assert.equal(existsSync(new URL('../../' + path, import.meta.url)), false, path);
});

test('real engine workflows remain independently required', () => {
  for (const [workflow, command] of [
    ['browser-smoke', 'node --test tools/browser-oracles/hosted/smoke.test.mjs'],
    ['browser-native-bridge', 'node --test tools/browser-oracles/hosted/native-bridge.test.mjs'],
  ]) {
    const source = read('.github/workflows/' + workflow + '.yml').replace(/\r\n/g, '\n');
    assert.ok(source.includes('  pull_request:\n'));
    assert.ok(source.includes('  push:\n'));
    assert.ok(source.includes('  workflow_dispatch:\n'));
    assert.ok(source.includes(command));
    assert.ok(source.includes('engine: [chromium, firefox, webkit]'));
  }
});
