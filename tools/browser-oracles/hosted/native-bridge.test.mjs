// Real Chromium/Firefox/WebKit versus independent Rust layout and hit targets.
// All source and capture bytes are retained; offline synthetic receipts are not
// substitutes for this optional, hosted node:test verification.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { test } from 'node:test';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { collectOne } from '../collect.mjs';
import { compareNativeBlockBridge } from '../../../tests/browser-oracles/native-block-bridge.mjs';

const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL('../../../', import.meta.url));
const sourceFile = new URL('../../../tests/browser-oracles/native-block-bridge.html', import.meta.url);

function checkMutation(tool, engine, nativeFile, outputDir) {
  const executable = fileURLToPath(new URL('../' + tool, import.meta.url));
  const child = spawnSync(process.execPath, [executable, engine, nativeFile, outputDir], {
    cwd: root, encoding: 'utf8', timeout: 120_000,
  });
  if (child.error) throw child.error;
  assert.equal(child.signal, null, child.stderr);
  assert.equal(child.status, 0, tool + ' failed:\n' + child.stderr + '\n' + child.stdout);
}

test('live native/browser parity and independent negative controls',
  { timeout: 240_000 }, async t => {
    const engine = process.env.BROWSER_ENGINE;
    const output = process.env.BROWSER_BRIDGE_OUTPUT;
    const nativeFile = process.env.BROWSER_BRIDGE_NATIVE;
    assert.ok(['chromium', 'firefox', 'webkit'].includes(engine), 'known engine required');
    assert.equal(process.versions.node, process.env.BROWSER_NODE_VERSION, 'pinned Node runtime');
    assert.equal(process.env.PLAYWRIGHT_BROWSERS_PATH, '/ms-playwright', 'managed engine image');
    assert.ok(output && nativeFile, 'explicit native receipt and evidence paths required');
    const version = require('playwright/package.json').version;
    assert.equal(version, '1.64.1', 'pinned reference-engine package');
    const out = resolve(output), nativePath = resolve(nativeFile);
    const browserTypes = await import('playwright');
    const native = JSON.parse(await readFile(nativePath, 'utf8'));

    await t.test('unchanged native geometry and DOM targets agree', async () => {
      const fixture = await readFile(sourceFile);
      await mkdir(join(out, 'captures'), { recursive: true });
      const snapshot = join(out, 'snapshot.html');
      await writeFile(snapshot, fixture, { flag: 'wx', mode: 0o444 });
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(out, 'captures', engine + '.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeBlockBridge(native, capture, fixture);
      await writeFile(join(out, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'equal',
        JSON.stringify({ invalid: report.invalid, incompatible: report.incompatible,
          differences: report.differences }));
    });

    await t.test('real width mutation changes exactly two border widths', () => {
      checkMutation('native-bridge-sensitivity.mjs', engine, nativePath, join(out, 'sensitivity'));
    });
    await t.test('real pointer-events mutation changes exactly six hit targets', () => {
      checkMutation('native-hit-sensitivity.mjs', engine, nativePath, join(out, 'hit-sensitivity'));
    });
  });
