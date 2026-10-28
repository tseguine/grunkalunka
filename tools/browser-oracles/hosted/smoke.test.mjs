// Hosted reference-engine tests. Offline canonical tests never import Playwright.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { access } from 'node:fs/promises';
import { resolve } from 'node:path';
import { runSmoke } from '../smoke.mjs';

const require = createRequire(import.meta.url);
test('live browser observations, repeatability and mutation contracts',
  { timeout: 240_000 }, async () => {
    const engine = process.env.BROWSER_ENGINE;
    assert.ok(['chromium', 'firefox', 'webkit'].includes(engine), 'known reference engine required');
    assert.equal(process.versions.node, process.env.BROWSER_NODE_VERSION, 'pinned Node version');
    assert.equal(process.env.PLAYWRIGHT_BROWSERS_PATH, '/ms-playwright', 'pinned browser image');
    await access(process.env.PLAYWRIGHT_BROWSERS_PATH);
    assert.ok(process.env.BROWSER_EVIDENCE_DIR, 'fresh evidence directory required');
    const version = require('playwright/package.json').version;
    assert.equal(version, '1.64.1', 'image and Playwright package revision');
    const browserTypes = await import('playwright');
    const summary = await runSmoke(engine, {
      browserTypes, playwrightVersion: version,
      outputDir: resolve(process.env.BROWSER_EVIDENCE_DIR),
    });
    assert.equal(summary.engine, engine);
    assert.equal(summary.selectedBaseline, 'passed');
  });
