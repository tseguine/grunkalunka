// All three original source-checked engine captures are required; a real
// disagreement is reported, not treated as standards nonconformance.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { test } from 'node:test';
import { join, resolve } from 'node:path';
import { crossEngineReport } from '../../../tests/browser-oracles/cross-engine.mjs';

const engines = ['chromium', 'firefox', 'webkit'];
test('compare three independent real-engine receipts', { timeout: 30_000 }, async () => {
  const input = process.env.BROWSER_CROSS_ENGINE_INPUT;
  const output = process.env.BROWSER_CROSS_ENGINE_OUTPUT;
  assert.ok(input && output, 'explicit artifact inputs and output directory required');
  assert.equal(process.versions.node, process.env.BROWSER_NODE_VERSION, 'pinned Node version');
  const source = await readFile(new URL('../../../tests/browser-oracles/layout-baseline.html', import.meta.url));
  const hash = createHash('sha256').update(source).digest('hex');
  const captures = {}, paths = {};
  for (const engine of engines) {
    const path = join(resolve(input), 'browser-smoke-' + engine, 'target', 'browser-smoke',
      engine, 'captures', 'first.json');
    paths[engine] = path;
    try {
      captures[engine] = JSON.parse(await readFile(path, 'utf8'));
    } catch (error) {
      captures[engine] = undefined;
      paths[engine] = { path, error: String(error) };
    }
  }
  const report = crossEngineReport(captures, hash);
  report.inputs = { source: 'tests/browser-oracles/layout-baseline.html', captures: paths };
  await mkdir(resolve(output), { recursive: true });
  await writeFile(join(resolve(output), 'report.json'),
    JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  assert.ok(['equal', 'different'].includes(report.status),
    'missing/invalid/incompatible source-checked evidence: ' + report.status);
  assert.equal(report.coverage.readyEngines, 3);
  assert.equal(report.coverage.comparedPairs, 3);
});
