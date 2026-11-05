// Actual browser execution of the production A1 presenter with controlled HTTP
// frames. No native app, source compiler, WIT service or renderer is substituted
// by a claim of end-to-end execution: this report covers the presenter boundary.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { offsetPresenterPointer, readPresenterAssets, startA1PresenterFixture } from './a1-presenter-fixture.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const TIMEOUT = 10_000;

async function ready(page) {
  await page.waitForFunction(() =>
    document.getElementById('note-frame').getAttribute('aria-busy') === 'false' &&
    !document.getElementById('reload-note').disabled, null, { timeout: TIMEOUT });
}

async function observation(page) {
  return page.evaluate(() => {
    const canvas = document.getElementById('note-frame');
    const select = document.getElementById('note-choice');
    const reconnect = document.getElementById('reconnect-view');
    const rect = canvas.getBoundingClientRect();
    return { url: location.href, fragment: location.hash,
      storedAuthority: /^[A-Za-z0-9_-]{32,128}$/.test(sessionStorage.getItem('grunkalunka:a1:session') ?? ''),
      localStorageEntries: localStorage.length, activeElement: document.activeElement?.id ?? '',
      canvas: { width: canvas.width, height: canvas.height, cssWidth: rect.width, cssHeight: rect.height,
        pixels: [[0, 0], [159, 79], [319, 159]].map(([x, y]) =>
          Array.from(canvas.getContext('2d').getImageData(x, y, 1, 1).data)),
        expanded: canvas.getAttribute('aria-expanded'), busy: canvas.getAttribute('aria-busy'),
        disabled: canvas.getAttribute('aria-disabled') },
      selected: select.value, selectionDisabled: select.disabled,
      labels: Array.from(select.options, option => option.textContent),
      injectedElements: select.querySelectorAll('b, img, script').length,
      reloadDisabled: document.getElementById('reload-note').disabled,
      scrollControls: ['scroll-start', 'scroll-up', 'scroll-down', 'scroll-end'].map(id => {
        const button = document.getElementById(id);
        const box = button.getBoundingClientRect();
        return { id, tag: button.tagName, label: button.textContent, disabled: button.disabled,
          controls: button.getAttribute('aria-controls'),
          x: box.x, y: box.y, width: box.width, height: box.height };
      }),
      reconnect: reconnect ? { hidden: reconnect.hidden, disabled: reconnect.disabled,
        busy: reconnect.getAttribute('aria-busy') } : null,
      status: document.getElementById('session-status').textContent,
      pageScrollY: scrollY, pageHeight: document.documentElement.scrollHeight,
      viewportHeight: innerHeight };
  });
}

function assertPixels(view, rgb) {
  assert.deepEqual([view.canvas.width, view.canvas.height], [320, 160]);
  assert.deepEqual(view.canvas.pixels, Array.from({ length: 3 }, () => [...rgb, 255]));
}

async function pointer(page) {
  const box = await page.locator('#note-frame').boundingBox();
  assert.ok(box);
  // Interior points are deliberate: the expected native target is the literal
  // (24,16), independently of the production pointer conversion implementation.
  await page.mouse.click(box.x + box.width * 24.25 / 320, box.y + box.height * 16.25 / 160);
  await ready(page);
}

function lastRequest(fixture, path) {
  const entry = fixture.observations().filter(item => item.path === path).at(-1);
  assert.ok(entry, 'missing browser request ' + path);
  return entry;
}

export async function runA1Presenter(engine, { browserTypes, playwrightVersion, outputDir,
  executablePath } = {}) {
  if (!['chromium', 'firefox', 'webkit'].includes(engine) ||
      typeof browserTypes?.[engine]?.launch !== 'function' ||
      typeof playwrightVersion !== 'string' || !playwrightVersion ||
      typeof outputDir !== 'string' || !outputDir ||
      (executablePath !== undefined && (typeof executablePath !== 'string' || !executablePath))) {
    throw new TypeError('Explicit browser engine, automation version and fresh output directory required');
  }
  const output = resolve(outputDir);
  await mkdir(dirname(output), { recursive: true });
  await mkdir(output); // Never overwrite previous evidence.
  const receipt = { schemaVersion: 1, kind: 'a1-production-presenter-controlled-http',
    scope: 'Real browser presenter and transport; controlled frames; no native/WIT end-to-end claim',
    engine, playwrightVersion, node: process.versions.node,
    platform: process.platform, arch: process.arch,
    revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    proofSources: await Promise.all([
      'tools/browser-oracles/a1-presenter-fixture.mjs',
      'tools/browser-oracles/a1-presenter.mjs',
      'tools/browser-oracles/hosted/a1-presenter.test.mjs',
    ].map(async path => ({ path,
      sha256: createHash('sha256').update(await readFile(join(root, path))).digest('hex') }))),
    stages: [], controls: [], status: 'incomplete' };
  let browser;
  let fixture;
  let controlFixture;
  try {
    browser = await browserTypes[engine].launch({ headless: true, timeout: 20_000,
      ...(executablePath === undefined ? {} : { executablePath }) });
    receipt.browserVersion = browser.version();
    const context = await browser.newContext({ viewport: { width: 1000, height: 800 },
      deviceScaleFactor: 1, hasTouch: true });
    const page = await context.newPage();
    page.setDefaultTimeout(TIMEOUT);
    const pageErrors = [];
    page.on('pageerror', error => pageErrors.push(error.message));
    fixture = await startA1PresenterFixture();
    receipt.productionSources = fixture.provenance;
    const networkUrls = [];
    page.on('request', request => networkUrls.push(request.url()));
    const record = async name => {
      const view = await observation(page);
      receipt.stages.push({ name, ...view });
      assert.equal(view.localStorageEntries, 0, name + ': local storage stays empty');
      return view;
    };

    await page.goto(fixture.url, { waitUntil: 'domcontentloaded' });
    await ready(page);
    let view = await record('initial-sync');
    assert.equal(view.fragment, '');
    assert.equal(view.url, fixture.origin + '/');
    assert.equal(view.storedAuthority, true);
    assert.equal(view.localStorageEntries, 0);
    assert.deepEqual([view.canvas.cssWidth, view.canvas.cssHeight], [640, 320]);
    assertPixels(view, [20, 40, 60]);
    assert.deepEqual(view.scrollControls.map(button => [button.tag, button.label, button.controls]), [
      ['BUTTON', 'Top', 'note-frame'], ['BUTTON', 'Page up', 'note-frame'],
      ['BUTTON', 'Page down', 'note-frame'], ['BUTTON', 'Bottom', 'note-frame'],
    ]);
    assert.ok(view.scrollControls.every(button => button.height >= 44));
    assert.equal(new Set(view.scrollControls.map(button => button.y)).size, 1, 'wide four-column controls');
    assert.deepEqual(view.scrollControls.map(button => button.disabled), [true, true, false, false]);
    assert.deepEqual(fixture.observations().map(item => [item.path, item.input, item.sequence]),
      [['/sync', {}, 0]]);

    await page.reload({ waitUntil: 'domcontentloaded' });
    await ready(page);
    view = await record('refresh-with-tab-authority');
    assert.equal(view.fragment, '');
    assert.equal(view.storedAuthority, true);
    assertPixels(view, [20, 40, 60]);
    assert.deepEqual(fixture.observations().map(item => [item.path, item.sequence, item.generation]),
      [['/sync', 0, 1], ['/sync', 0, 2]]);

    await pointer(page);
    view = await record('double-size-pointer');
    assert.deepEqual(lastRequest(fixture, '/input').input, { x: 24, y: 16 });
    assert.equal(lastRequest(fixture, '/input').requestGeneration, '2');
    assert.equal(view.activeElement, 'note-frame');
    assert.equal(view.canvas.expanded, 'false');
    assertPixels(view, [40, 20, 60]);
    assert.ok(view.scrollControls.every(button => button.disabled), 'collapsed view cannot scroll');
    await page.locator('#note-frame').press('Enter');
    await ready(page);
    view = await record('keyboard-enter');
    assert.deepEqual(lastRequest(fixture, '/input').input, { x: 24, y: 16 });
    assertPixels(view, [20, 40, 60]);

    const doubledBox = await page.locator('#note-frame').boundingBox();
    await page.mouse.move(doubledBox.x + 320, doubledBox.y + 160);
    await page.mouse.wheel(0, 64);
    await page.waitForFunction(() => document.getElementById('session-status').textContent.includes('10%'));
    await ready(page);
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 32 });
    assertPixels(await record('double-size-wheel'), [20, 40, 62]);
    await page.locator('#note-frame').press('Home');
    await ready(page);
    assertPixels(await record('return-to-top-before-resize'), [20, 40, 60]);

    await page.setViewportSize({ width: 378, height: 800 });
    await page.waitForFunction(() => document.getElementById('note-frame').getBoundingClientRect().width === 320);
    await pointer(page);
    view = await record('native-size-pointer');
    assert.deepEqual([view.canvas.cssWidth, view.canvas.cssHeight], [320, 160]);
    assert.deepEqual(lastRequest(fixture, '/input').input, { x: 24, y: 16 });
    assertPixels(view, [40, 20, 60]);
    await page.locator('#note-frame').press('Space');
    await ready(page);
    assertPixels(await record('keyboard-space'), [20, 40, 60]);

    const box = await page.locator('#note-frame').boundingBox();
    const beforeWheel = await observation(page);
    assert.ok(beforeWheel.pageHeight > beforeWheel.pageScrollY + beforeWheel.viewportHeight,
      'the page must have scroll room for preventDefault to be observable');
    await page.mouse.move(box.x + 160, box.y + 80);
    await page.mouse.wheel(0, 64);
    await page.waitForFunction(() => document.getElementById('session-status').textContent.includes('20%'));
    await ready(page);
    view = await record('wheel');
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 64 });
    assert.equal(view.pageScrollY, beforeWheel.pageScrollY, 'native scrolling consumes the browser wheel');
    assertPixels(view, [20, 40, 64]);
    for (const [key, y, blue] of [['ArrowDown', 80, 65], ['PageDown', 240, 75],
      ['End', 320, 80], ['Home', 0, 60]]) {
      await page.locator('#note-frame').press(key);
      await ready(page);
      assert.deepEqual(lastRequest(fixture, '/scroll').input, { y });
      assertPixels(await record('keyboard-' + key), [20, 40, blue]);
    }

    view = await observation(page);
    assert.deepEqual(view.scrollControls.map(button => button.disabled), [true, true, false, false]);
    assert.equal(view.scrollControls[0].y, view.scrollControls[1].y);
    assert.equal(view.scrollControls[2].y, view.scrollControls[3].y);
    assert.ok(view.scrollControls[2].y > view.scrollControls[0].y, 'narrow two-column controls');
    assert.ok(view.scrollControls.every(button => button.height >= 44));
    await page.locator('#scroll-down').tap();
    await ready(page);
    view = await record('touch-page-down');
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 160 });
    assertPixels(view, [20, 40, 70]);
    assert.ok(view.scrollControls.every(button => !button.disabled));
    await page.locator('#scroll-end').press('Enter');
    await ready(page);
    view = await record('button-enter-bottom');
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 320 });
    assertPixels(view, [20, 40, 80]);
    assert.deepEqual(view.scrollControls.map(button => button.disabled), [false, false, true, true]);
    await page.locator('#scroll-up').press('Space');
    await ready(page);
    view = await record('button-space-page-up');
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 160 });
    assert.equal(view.activeElement, 'scroll-up', 'keyboard focus stays on the enabled HTML button');
    assertPixels(view, [20, 40, 70]);
    await page.locator('#scroll-start').tap();
    await ready(page);
    view = await record('touch-top');
    assert.deepEqual(lastRequest(fixture, '/scroll').input, { y: 0 });
    assertPixels(view, [20, 40, 60]);
    assert.deepEqual(view.scrollControls.map(button => button.disabled), [true, true, false, false]);

    const releaseSelection = fixture.holdNextSelectionFailure();
    await page.locator('#note-choice').selectOption('1');
    await page.waitForFunction(() => document.getElementById('note-choice').disabled &&
      document.getElementById('session-status').textContent === 'Opening note…');
    view = await record('selection-pending');
    assert.equal(view.selected, '0');
    assert.ok(view.scrollControls.every(button => button.disabled), 'pending replacement cannot scroll');
    assertPixels(view, [20, 40, 60]);
    releaseSelection();
    await ready(page);
    view = await record('selection-failed');
    assert.equal(view.selected, '0');
    assert.equal(view.status, 'Fixture note is unavailable.');
    assertPixels(view, [20, 40, 60]);
    await page.locator('#note-choice').selectOption('1');
    await ready(page);
    view = await record('selection-committed');
    assert.equal(view.selected, '1');
    assert.equal(view.labels[1], '<b>Second & "note"</b>.md');
    assert.equal(view.injectedElements, 0);
    assertPixels(view, [60, 80, 100]);
    await page.locator('#reload-note').click();
    await ready(page);
    view = await record('reload-selected-note');
    assert.equal(view.selected, '1');
    assertPixels(view, [60, 80, 106]);
    await page.locator('#note-choice').selectOption('0');
    await ready(page);

    const beforeLoss = fixture.state();
    const inputsBeforeLoss = fixture.observations().filter(item => item.path === '/input').length;
    fixture.loseNextInputReply();
    await page.locator('#note-frame').press('Enter');
    await page.locator('#reconnect-view').waitFor({ state: 'visible' });
    await page.waitForFunction(() => !document.getElementById('reconnect-view').disabled);
    view = await record('committed-reply-lost');
    assert.equal(view.canvas.disabled, 'true');
    assert.equal(view.reloadDisabled, true);
    assert.ok(view.scrollControls.every(button => button.disabled), 'disconnected view cannot scroll');
    assertPixels(view, [20, 40, 60]);
    assert.equal(fixture.state().sequence, beforeLoss.sequence + 1);
    assert.equal(fixture.state().expanded, 0);
    await page.locator('#reconnect-view').click();
    await ready(page);
    view = await record('reconnected-without-replay');
    assertPixels(view, [40, 20, 60]);
    assert.equal(fixture.state().sequence, beforeLoss.sequence + 1);
    assert.equal(fixture.observations().filter(item => item.path === '/input').length, inputsBeforeLoss + 1);
    assert.equal(fixture.state().inputGeneration, 3);
    await pointer(page);
    assert.equal(lastRequest(fixture, '/input').requestGeneration, '3');
    assertPixels(await record('input-after-reconnect'), [20, 40, 60]);

    const beforeStale = fixture.state().sequence;
    fixture.advanceGeneration();
    await page.locator('#note-frame').press('Enter');
    await page.locator('#reconnect-view').waitFor({ state: 'visible' });
    await page.waitForFunction(() => !document.getElementById('reconnect-view').disabled);
    view = await record('stale-input-generation');
    assert.equal(lastRequest(fixture, '/input').status, 409);
    assert.equal(fixture.state().sequence, beforeStale);
    assert.equal(view.canvas.disabled, 'true');
    await page.locator('#reconnect-view').click();
    await ready(page);
    assertPixels(await record('generation-reconciled'), [20, 40, 60]);
    assert.equal(fixture.state().inputGeneration, 5);
    assert.equal(fixture.state().sequence, beforeStale);
    assert.deepEqual(pageErrors, []);
    assert.ok(networkUrls.every(url => !url.includes('#') && !url.includes('?') &&
      !url.includes(new URL(fixture.url).hash.slice(1))), 'session authority never enters request URLs');
    receipt.requests = fixture.observations();
    receipt.finalState = fixture.state();
    await page.screenshot({ path: join(output, 'presenter.png'), fullPage: true });
    await context.close();

    const assets = await readPresenterAssets();
    controlFixture = await startA1PresenterFixture({ clientSource: offsetPresenterPointer(assets.client) });
    const controlContext = await browser.newContext({ viewport: { width: 1000, height: 800 },
      deviceScaleFactor: 1, hasTouch: true });
    const controlPage = await controlContext.newPage();
    controlPage.setDefaultTimeout(TIMEOUT);
    await controlPage.goto(controlFixture.url, { waitUntil: 'domcontentloaded' });
    await ready(controlPage);
    await pointer(controlPage);
    const changed = lastRequest(controlFixture, '/input').input;
    assert.deepEqual(changed, { x: 25, y: 16 });
    assert.notDeepEqual(changed, { x: 24, y: 16 });
    assert.notEqual(controlFixture.provenance.served.client, fixture.provenance.served.client);
    assert.deepEqual(controlFixture.provenance.sources, fixture.provenance.sources);
    receipt.controls.push({ name: 'served-pointer-x-plus-one', expectedOriginal: { x: 24, y: 16 },
      observed: changed, detected: true, sources: controlFixture.provenance });
    await controlContext.close();
    receipt.status = 'passed';
    return receipt;
  } catch (error) {
    receipt.status = 'failed';
    receipt.error = error.message;
    throw error;
  } finally {
    try {
      const cleanup = await Promise.allSettled([browser?.close(), controlFixture?.close(), fixture?.close()]);
      const failures = cleanup.filter(result => result.status === 'rejected');
      if (failures.length) {
        receipt.status = 'failed';
        receipt.cleanupErrors = failures.map(result => String(result.reason));
        throw new Error('Presenter browser or fixture cleanup failed');
      }
    } finally {
      await writeFile(join(output, 'observations.json'), JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
    }
  }
}
