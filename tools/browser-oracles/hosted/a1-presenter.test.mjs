// Reuses the existing three-engine smoke environment without native builds.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { access } from 'node:fs/promises';
import { resolve } from 'node:path';
import test from 'node:test';
import { runA1Presenter } from '../a1-presenter.mjs';

const require = createRequire(import.meta.url);
test('production A1 presenter executes real browser input and reconnect contracts',
  { timeout: 150_000 }, async () => {
    const engine = process.env.BROWSER_ENGINE;
    assert.ok(['chromium', 'firefox', 'webkit'].includes(engine));
    assert.equal(process.versions.node, process.env.BROWSER_NODE_VERSION, 'pinned Node version');
    assert.equal(process.env.PLAYWRIGHT_BROWSERS_PATH, '/ms-playwright', 'pinned browser image');
    await access(process.env.PLAYWRIGHT_BROWSERS_PATH);
    assert.ok(process.env.BROWSER_A1_EVIDENCE_DIR, 'fresh presenter evidence directory required');
    const playwrightVersion = require('playwright/package.json').version;
    assert.equal(playwrightVersion, '1.64.1', 'image and Playwright package revision');
    const browserTypes = await import('playwright');
    const report = await runA1Presenter(engine, { browserTypes, playwrightVersion,
      outputDir: resolve(process.env.BROWSER_A1_EVIDENCE_DIR) });
    assert.equal(report.status, 'passed');
    assert.equal(report.controls.length, 1);
    assert.equal(report.controls[0].detected, true);
  });
