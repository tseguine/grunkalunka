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
import { compareNativeFlexBridge } from '../../../tests/browser-oracles/native-flex-bridge.mjs';

const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL('../../../', import.meta.url));
const sourceFile = new URL('../../../tests/browser-oracles/native-block-bridge.html', import.meta.url);
const flexSourceFile = new URL('../../../tests/browser-oracles/native-flex-bridge.html', import.meta.url);

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
    const flexNativeFile = process.env.BROWSER_BRIDGE_FLEX_NATIVE;
    assert.ok(['chromium', 'firefox', 'webkit'].includes(engine), 'known engine required');
    assert.equal(process.versions.node, process.env.BROWSER_NODE_VERSION, 'pinned Node runtime');
    assert.equal(process.env.PLAYWRIGHT_BROWSERS_PATH, '/ms-playwright', 'managed engine image');
    assert.ok(output && nativeFile && flexNativeFile, 'explicit native receipts and evidence paths required');
    const version = require('playwright/package.json').version;
    assert.equal(version, '1.64.1', 'pinned reference-engine package');
    const out = resolve(output), nativePath = resolve(nativeFile);
    const browserTypes = await import('playwright');
    const native = JSON.parse(await readFile(nativePath, 'utf8'));
    const flexNative = JSON.parse(await readFile(resolve(flexNativeFile), 'utf8'));

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
    await t.test('real height mutation changes only child height and next sibling y', () => {
      checkMutation('native-height-sensitivity.mjs', engine, nativePath, join(out, 'height-sensitivity'));
    });

    await t.test('fixed-size row geometry, wrapping and input agree', async () => {
      const fixture = await readFile(flexSourceFile);
      const directory = join(out, 'flex');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'snapshot.html');
      await writeFile(snapshot, fixture, { flag: 'wx', mode: 0o444 });
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, fixture);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'equal',
        JSON.stringify({ invalid: report.invalid, incompatible: report.incompatible,
          differences: report.differences }));
    });

    await t.test('real flex column-gap mutation moves one box and changes one hit', async () => {
      const original = (await readFile(flexSourceFile)).toString('utf8');
      const originalRule = 'host.style.columnGap = "24px";';
      assert.equal(original.split(originalRule).length, 2, 'one reviewed CSS gap mutation site');
      const mutated = Buffer.from(original.replace(originalRule, 'host.style.columnGap = "32px";'));
      const directory = join(out, 'flex-gap-sensitivity');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'column-gap-mutation.html');
      await writeFile(snapshot, mutated, { flag: 'wx', mode: 0o444 });
      // A fresh engine executes changed source. The original native receipt
      // remains untouched, and provenance authenticates the modified bytes.
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, mutated);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'different', 'real CSS gap mutation must be detected');
      assert.equal(report.conformance, 'not-assessed');
      assert.deepEqual(report.invalid, []);
      assert.deepEqual(report.incompatible, []);
      assert.deepEqual(report.differences, [
        { kind: 'geometry', path: '$.states.columnGap.boxes.row-c.x', left: 114, right: 122 },
        { kind: 'hit', path: '$.states.columnGap.hits.gap-wrap', left: 'row-c', right: 'row-host' },
      ]);
    });

    await t.test('real flex justification mutation moves item subtrees and changes five hits', async () => {
      const original = (await readFile(flexSourceFile)).toString('utf8');
      const originalRule = '  host.style.justifyContent = "safe flex-end";\n  states.justifyEnd = sample();';
      const changedRule = '  host.style.justifyContent = "flex-start";\n  states.justifyEnd = sample();';
      assert.equal(original.split(originalRule).length, 2, 'one reviewed justifyEnd assignment');
      const mutated = Buffer.from(original.replace(originalRule, changedRule));
      const directory = join(out, 'flex-justify-sensitivity');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'justify-end-mutation.html');
      await writeFile(snapshot, mutated, { flag: 'wx', mode: 0o444 });
      // This changes one phase's real declaration, preserving the independent
      // native input and every existing phase and source-mutation control.
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, mutated);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'different', 'real CSS justification mutation must be detected');
      assert.equal(report.conformance, 'not-assessed');
      assert.deepEqual(report.invalid, []);
      assert.deepEqual(report.incompatible, []);
      assert.deepEqual(report.differences, [
        { kind: 'geometry', path: '$.states.justifyEnd.boxes.row-a.x', left: 56, right: 26 },
        { kind: 'geometry', path: '$.states.justifyEnd.boxes.row-b.x', left: 135, right: 105 },
        { kind: 'geometry', path: '$.states.justifyEnd.boxes.row-inner.x', left: 138, right: 108 },
        { kind: 'geometry', path: '$.states.justifyEnd.boxes.row-c.x', left: 204, right: 174 },
        { kind: 'hit', path: '$.states.justifyEnd.hits.a', left: 'row-host', right: 'row-a' },
        { kind: 'hit', path: '$.states.justifyEnd.hits.b', left: 'row-a', right: 'row-inner' },
        { kind: 'hit', path: '$.states.justifyEnd.hits.nowrap', left: 'row-b', right: 'row-c' },
        { kind: 'hit', path: '$.states.justifyEnd.hits.gap-wrap', left: 'row-host', right: 'row-b' },
        { kind: 'hit', path: '$.states.justifyEnd.hits.gap', left: 'row-a', right: 'row-host' },
      ]);
    });

    await t.test('real flex paint-order mutation changes one overlapping target without reflow', async () => {
      const original = (await readFile(flexSourceFile)).toString('utf8');
      const originalRule = '  after.style.position = "static";\n  states.safeOverflow = sample();';
      const changedRule = '  after.style.position = "relative";\n  states.safeOverflow = sample();';
      assert.equal(original.split(originalRule).length, 2, 'one reviewed safeOverflow paint-order assignment');
      const mutated = Buffer.from(original.replace(originalRule, changedRule));
      const directory = join(out, 'flex-paint-order-sensitivity');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'positioned-after-mutation.html');
      await writeFile(snapshot, mutated, { flag: 'wx', mode: 0o444 });
      // Only the reference source becomes positioned. Native input stays in
      // the non-positioned profile; this control does not admit native z-order.
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, mutated);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'different', 'real CSS paint-order mutation must be detected');
      assert.equal(report.conformance, 'not-assessed');
      assert.deepEqual(report.invalid, []);
      assert.deepEqual(report.incompatible, []);
      assert.deepEqual(report.differences, [
        { kind: 'hit', path: '$.states.safeOverflow.hits.gap-wrap', left: 'row-b', right: 'row-after' },
      ]);
    });

    await t.test('real per-item alignment mutation moves only its subtree and one hit', async () => {
      const original = (await readFile(flexSourceFile)).toString('utf8');
      const originalRule = '  b.style.alignSelf = "safe flex-end";\n  states.selfAlignMixed = sample();';
      const changedRule = '  b.style.alignSelf = "flex-start";\n  states.selfAlignMixed = sample();';
      assert.equal(original.split(originalRule).length, 2, 'one reviewed per-item alignment assignment');
      const mutated = Buffer.from(original.replace(originalRule, changedRule));
      const directory = join(out, 'flex-self-alignment-sensitivity');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'align-self-mutation.html');
      await writeFile(snapshot, mutated, { flag: 'wx', mode: 0o444 });
      // The parent alignment, other items and original native receipt stay
      // unchanged. Only this actual item's declaration changes in fresh source.
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, mutated);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'different', 'real per-item alignment mutation must be detected');
      assert.equal(report.conformance, 'not-assessed');
      assert.deepEqual(report.invalid, []);
      assert.deepEqual(report.incompatible, []);
      assert.deepEqual(report.differences, [
        { kind: 'geometry', path: '$.states.selfAlignMixed.boxes.row-b.y', left: 46, right: 28 },
        { kind: 'geometry', path: '$.states.selfAlignMixed.boxes.row-inner.y', left: 49, right: 31 },
        { kind: 'hit', path: '$.states.selfAlignMixed.hits.b', left: 'row-host', right: 'row-inner' },
      ]);
    });

    await t.test('real line-packing mutation moves both lines and changes two hits', async () => {
      const original = (await readFile(flexSourceFile)).toString('utf8');
      const originalRule = '  host.style.alignContent = "safe flex-end";\n  states.contentEnd = sample();';
      const changedRule = '  host.style.alignContent = "flex-start";\n  states.contentEnd = sample();';
      assert.equal(original.split(originalRule).length, 2, 'one reviewed line-packing assignment');
      const mutated = Buffer.from(original.replace(originalRule, changedRule));
      const directory = join(out, 'flex-content-alignment-sensitivity');
      await mkdir(directory, { recursive: true });
      const snapshot = join(directory, 'align-content-mutation.html');
      await writeFile(snapshot, mutated, { flag: 'wx', mode: 0o444 });
      // Item sizes and alignments, other phases and native input remain fixed.
      // A fresh browser executes the changed line-packing declaration itself.
      const capture = await collectOne(engine, {
        browserTypes, playwrightVersion: version, fixturePath: snapshot,
      });
      await writeFile(join(directory, 'capture.json'),
        JSON.stringify(capture, null, 2) + '\n', { flag: 'wx' });
      const report = compareNativeFlexBridge(flexNative, capture, mutated);
      await writeFile(join(directory, 'comparison.json'),
        JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
      assert.equal(report.status, 'different', 'real line-packing mutation must be detected');
      assert.equal(report.conformance, 'not-assessed');
      assert.deepEqual(report.invalid, []);
      assert.deepEqual(report.incompatible, []);
      assert.deepEqual(report.differences, [
        { kind: 'geometry', path: '$.states.contentEnd.boxes.row-a.y', left: 67, right: 35 },
        { kind: 'geometry', path: '$.states.contentEnd.boxes.row-b.y', left: 60, right: 28 },
        { kind: 'geometry', path: '$.states.contentEnd.boxes.row-inner.y', left: 63, right: 31 },
        { kind: 'geometry', path: '$.states.contentEnd.boxes.row-c.y', left: 108, right: 76 },
        { kind: 'hit', path: '$.states.contentEnd.hits.b', left: 'row-host', right: 'row-inner' },
        { kind: 'hit', path: '$.states.contentEnd.hits.wrap', left: 'row-a', right: 'row-c' },
      ]);
    });
  });
