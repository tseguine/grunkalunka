// Offline browser test registration is a canonical command, not an extra
// GitHub Actions workflow. The remaining hosted workflows collect real evidence.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const read = path => readFileSync(new URL('../../' + path, import.meta.url), 'utf8');

test('offline browser tests are wired into canonical full and partial CI', () => {
  const manifest = JSON.parse(read('scripts/checks/004000-browser-oracle-contracts.json'));
  const descriptor = JSON.parse(read('ci/impact/checks/browser-oracle-contracts.json'));
  assert.equal(descriptor.id, 'check:browser-oracle-contracts');
  assert.equal(descriptor.command, 'node');
  assert.deepEqual(descriptor.args, ['scripts/check-browser-oracles.mjs']);
  assert.ok(descriptor.dependsOn.includes('test:browser-oracle-contracts'));
  assert.deepEqual(manifest, {
    command: 'node', args: ['scripts/check-browser-oracles.mjs'],
  });
  // Observe the runner's actual emitted commands in both profiles. A source
  // spelling assertion would neither prove execution nor catch PR exclusion.
  for (const profile of ['full', 'pr']) {
    const commands = JSON.parse(execFileSync(process.execPath,
      ['ci/impact/collect-checks.mjs', '--profile=' + profile], {
        cwd: fileURLToPath(new URL('../../', import.meta.url)),
        encoding: 'utf8', timeout: 20_000, maxBuffer: 16 * 1024 * 1024,
      }));
    const browserCommands = commands.filter(({ command, args }) =>
      command === manifest.command && JSON.stringify(args) === JSON.stringify(manifest.args));
    assert.equal(browserCommands.length, 1,
      profile + ' CI must execute the offline browser command exactly once');
  }

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
