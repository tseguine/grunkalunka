import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { browserOfflineTestFiles } from '../../scripts/check-browser-oracles.mjs';

test('canonical browser command discovers both oracle and collector contracts', () => {
  const paths = browserOfflineTestFiles();
  assert.ok(paths.includes('tests/browser-oracles/compare.test.mjs'));
  assert.ok(paths.includes('tests/browser-oracles/check-runner.test.mjs'));
  assert.ok(paths.includes('tools/browser-oracles/collect.test.mjs'));
  assert.ok(paths.includes('tools/browser-oracles/native-bridge-sensitivity.test.mjs'));
  assert.ok(paths.includes('tools/browser-oracles/native-hit-sensitivity.test.mjs'));
  assert.ok(paths.every(path => path.endsWith('.test.mjs')));
  assert.ok(!paths.some(path => path.includes('/hosted/')),
    'tests requiring browser downloads must not enter offline checks');
  assert.equal(new Set(paths).size, paths.length);
  assert.deepEqual(paths, [...paths].sort());
});

test('new nested offline tests are discovered and hosted tests excluded', () => {
  const root = mkdtempSync(join(tmpdir(), 'grunkalunka-browser-discovery-'));
  try {
    for (const folder of [
      'tests/browser-oracles/nested',
      'tools/browser-oracles/hosted',
      'tools/browser-oracles/node_modules/sample',
    ]) {
      mkdirSync(join(root, folder), { recursive: true });
    }
    writeFileSync(join(root, 'tests/browser-oracles/nested/added.test.mjs'), '');
    writeFileSync(join(root, 'tools/browser-oracles/hosted/live.test.mjs'), '');
    writeFileSync(join(root, 'tools/browser-oracles/node_modules/sample/vendor.test.mjs'), '');
    assert.deepEqual(browserOfflineTestFiles(root),
      ['tests/browser-oracles/nested/added.test.mjs']);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
