import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { access, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compileSource } from '../../tools/build/compile.mjs';
import { escapeHtml, renderNotePreview, renderWithCompiledToggle } from '../../examples/note-preview/render.mjs';
import { parseArguments, runPreview } from '../../examples/note-preview/run.mjs';
import { buildCommands } from '../../examples/note-preview/build.mjs';
import { packageNotePreview, parsePackageArguments } from '../../examples/note-preview/package.mjs';
import { prepareNoteScene } from '../../examples/note-preview/scene.mjs';

const bytes = Buffer.from('# Root\r\n## 子😀\n', 'utf8');
const headings = [
  { level: 1, start: 0, end: 8, sectionEnd: bytes.length, parent: null, text: 'Root' },
  { level: 2, start: 8, end: bytes.length, sectionEnd: bytes.length, parent: 0, text: '子😀' },
];
const grant = () => ({ profile: 'granted', sourceBytes: bytes, headings, backingReads: 1 });
const compiler = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url));

function excerpt(doc, visibleCount) {
  return renderNotePreview({ ...doc, visibleCount });
}

test('trusted lifted outline displays source ranges, hierarchy and Unicode', () => {
  const html = excerpt(grant(), 2);
  assert.match(html, /^<!doctype html>\n<html lang="en">/);
  assert.match(html, /data-level="1" data-start="0" data-end="8" data-section-end="19" data-parent="-1">Root<\/li>/);
  assert.match(html, /data-level="2" data-start="8" data-end="19" data-section-end="19" data-parent="0">子😀<\/li>/);
  assert.ok(html.indexOf('>Root</li>') < html.indexOf('>子😀</li>'));
  assert.doesNotMatch(html, /href=|src=/);
});

test('collapsed and partial outlines produce distinct deterministic HTML', () => {
  const expanded = excerpt(grant(), 2);
  const partial = excerpt(grant(), 1);
  const collapsed = excerpt(grant(), 0);
  assert.equal(excerpt(grant(), 2), expanded);
  assert.match(collapsed, /Outline collapsed/);
  assert.doesNotMatch(collapsed, /<li/);
  assert.match(partial, />Root<\/li>/);
  assert.doesNotMatch(partial, /子😀/);
  assert.notEqual(expanded, partial);
  assert.notEqual(partial, collapsed);
});

test('denial is a distinct static result with no protected backing reads', () => {
  const denied = { profile: 'denied', sourceBytes: Buffer.from('# Private sentinel\n'),
    headings: [], backingReads: 0 };
  const html = excerpt(denied, 0);
  assert.match(html, /role="alert">Document access denied/);
  assert.doesNotMatch(html, /Private sentinel|note-outline/);
  assert.throws(() => excerpt({ ...denied, headings }, 0), /Denied profile/);
  assert.throws(() => excerpt({ ...denied, backingReads: 1 }, 0), /Denied profile/);
  assert.throws(() => excerpt({ ...denied, profile: 'unknown' }, 0), /profile/);
  assert.throws(() => excerpt({ ...grant(), backingReads: 0 }, 0), /requires exactly one/);
});

test('inert heading labels escape markup, attributes and NUL', () => {
  assert.equal(escapeHtml('<img src="x" onerror=\'bad\'>&\u0000'),
    '&lt;img src=&quot;x&quot; onerror=&#39;bad&#39;&gt;&amp;\uFFFD');
  const sourceBytes = Buffer.from('# Escape\n');
  const html = excerpt({ profile: 'granted', sourceBytes, backingReads: 1,
    headings: [{ level: 1, start: 0, end: sourceBytes.length, sectionEnd: sourceBytes.length,
      parent: null, text: '<script>"x" & \'y\'</script>\u0000' }] }, 1);
  assert.match(html, /&lt;script&gt;&quot;x&quot; &amp; &#39;y&#39;&lt;\/script&gt;�/);
  assert.doesNotMatch(html, /<script>|onerror=/);
  assert.throws(() => escapeHtml(42), /string/);
});

test('malformed spans, duplicate starts, ancestry, section ends and quotas fail closed', () => {
  const cases = [
    { ...grant(), headings: [{ ...headings[0], start: 9 }] },
    { ...grant(), headings: [headings[0], { ...headings[1], start: 0 }] },
    { ...grant(), headings: [headings[0], { ...headings[1], start: 7 }] },
    { ...grant(), headings: [headings[0], { ...headings[1], parent: null }] },
    { ...grant(), headings: [{ ...headings[0], sectionEnd: 8 }, headings[1]] },
    { ...grant(), headings: [{ ...headings[0], end: bytes.length + 1 }] },
    { ...grant(), headings: [{ ...headings[0], text: 'é'.repeat(2049) }] },
    { ...grant(), headings: Array.from({ length: 65 }, () => headings[0]) },
    { ...grant(), sourceBytes: Buffer.alloc(4097) },
    { ...grant(), sourceBytes: Uint8Array.of(0xff) },
    { ...grant(), headings: [], visibleCount: 1 },
  ];
  for (const [index, doc] of cases.entries()) {
    assert.throws(() => excerpt(doc, doc.visibleCount ?? doc.headings.length), undefined, `case ${index}`);
  }
  // Section ends are recomputed from actual heading levels; a sibling closes
  // its parent's section at the sibling's start, but a child does not.
  const sibling = { level: 1, start: 8, end: bytes.length, sectionEnd: bytes.length,
    parent: null, text: 'Sibling' };
  assert.match(excerpt({ ...grant(), headings: [
    { ...headings[0], sectionEnd: 8 }, sibling,
  ] }, 2), />Sibling<\/li>/);
});

test('actual compiled JS/TS Wasm switches HTML and changed source changes HTML', { timeout: 120_000 }, async () => {
  for (const language of ['js', 'ts']) {
    const source = await readFile(new URL(`../../examples/note-preview/toggle.${language}`, import.meta.url), 'utf8');
    const { wasm } = await compileSource({ compiler, source, language });
    assert.ok(WebAssembly.validate(wasm), 'compiler emitted real Wasm');
    const collapsed = await renderWithCompiledToggle({ wasm, expanded: 0, ...grant() });
    const shown = await renderWithCompiledToggle({ wasm, expanded: 1, ...grant() });
    const negative = await renderWithCompiledToggle({ wasm, expanded: -1, ...grant() });
    assert.notEqual(collapsed, shown);
    assert.equal(collapsed, negative);
    assert.doesNotMatch(collapsed, /data-level=/);
    assert.match(shown, /data-level="2"/);
    const denied = await renderWithCompiledToggle({ wasm, expanded: 1,
      profile: 'denied', sourceBytes: bytes, headings: [], backingReads: 0 });
    assert.doesNotMatch(denied, /Root|子😀/);
    assert.match(denied, /access denied/);
    const changed = source.replace('expanded > 0 ? count : 0', 'expanded > 0 ? 0 : count');
    assert.notEqual(changed, source);
    const { wasm: mutated } = await compileSource({ compiler, source: changed, language });
    const mutatedHtml = await renderWithCompiledToggle({ wasm: mutated, expanded: 1, ...grant() });
    assert.equal(mutatedHtml, collapsed, 'changed source must change visible output');
    await assert.rejects(compileSource({ compiler, source: 'export function run( {', language }));
  }
});

const projectRoot = fileURLToPath(new URL('../../', import.meta.url));
const targetRoot = resolve(projectRoot, process.env.CARGO_TARGET_DIR ?? 'target');
const nativeExample = name => join(targetRoot, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));

function checkedCommand(command, args, input) {
  const result = spawnSync(command, args, {
    cwd: projectRoot, input, encoding: 'utf8', shell: false,
    timeout: 180_000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null, command + ' terminated by a signal');
  assert.equal(result.status, 0, result.stderr || command + ' failed');
  return result.stdout;
}

function nativeOutlineRows(output) {
  if (!output) return [];
  return output.trimEnd().split('\n').map(line => {
    const matched = /^([1-6]) (\d+) (\d+) (\d+) (-?\d+) (\d+) ([a-f0-9]*)$/.exec(line);
    assert.ok(matched, 'native outline must use the unchanged typed row protocol');
    const [, level, start, end, sectionEnd, parent, length, textHex] = matched;
    const textBytes = Buffer.from(textHex, 'hex');
    assert.equal(textBytes.length, Number(length), 'native label byte length');
    return {
      level: Number(level), start: Number(start), end: Number(end),
      sectionEnd: Number(sectionEnd), parent: Number(parent) === -1 ? null : Number(parent),
      text: new TextDecoder('utf-8', { fatal: true }).decode(textBytes),
    };
  });
}

test('real Markdown/WIT guest drives native-parity granted and denied HTML through compiled JS Wasm',
  { timeout: 300_000 }, async () => {
    const guestBuild = ['build', '-p', 'grunkalunka-markdown', '--example',
      'outline_guest', '--locked', '--offline', '-j', '4'];
    // Reuses the existing guest, adapter/WIT and native library. No new
    // GitHub workflow or fixture-only fake of a permitted WIT read.
    checkedCommand('cargo', guestBuild);
    checkedCommand('cargo', [...guestBuild, '--target', 'wasm32-unknown-unknown']);
    checkedCommand('cargo', ['build', '-p', 'grunkalunka-runtime', '--example',
      'a1_note_preview', '--locked', '--offline', '-j', '4']);
    checkedCommand('cargo', ['build', '-p', 'grunkalunka-layout', '--example',
      'a1_note_scene', '--locked', '--offline', '-j', '4']);

    const driver = nativeExample('a1_note_preview');
    const layoutDriver = nativeExample('a1_note_scene');
    const native = nativeExample('outline_guest');
    const guest = join(targetRoot, 'wasm32-unknown-unknown', 'debug', 'examples',
      'outline_guest.wasm');
    const source = await readFile(new URL('../../examples/note-preview/toggle.js', import.meta.url),
      'utf8');
    const { wasm } = await compileSource({ compiler, source, language: 'js' });
    assert.ok(WebAssembly.validate(wasm));
    const changedSource = source.replace('expanded > 0 ? count : 0', 'expanded > 0 ? 0 : count');
    assert.notEqual(changedSource, source);
    const { wasm: changedWasm } = await compileSource({ compiler, source: changedSource, language: 'js' });
    const fixtures = [
      '# Root\r\n### 子😀\n[relative](../private.md)\n',
      '# &lt;script&gt;bad()&lt;/script&gt;\n## Café\n',
      '# Sensitive literal \u0000 marker\n',
    ];
    for (const note of fixtures) {
      const bytes = Buffer.from(note, 'utf8');
      const report = JSON.parse(checkedCommand(driver, [guest], bytes));
      assert.equal(report.schemaVersion, 1);
      assert.deepEqual(report.profiles.map(item => item.profile), ['denied', 'granted']);
      const [denied, granted] = report.profiles;
      assert.deepEqual(denied.headings, []);
      assert.equal(Object.hasOwn(denied, 'sourceHex'), false,
        'denied host output cannot disclose the original note');
      assert.deepEqual([denied.attempts, denied.backingReads, denied.cleanups], [1, 0, 1]);
      assert.deepEqual([granted.attempts, granted.backingReads, granted.cleanups], [1, 1, 1]);
      assert.match(granted.sourceHex, /^(?:[a-f0-9]{2})*$/);
      assert.deepEqual(Buffer.from(granted.sourceHex, 'hex'), bytes);
      const headings = granted.headings.map(heading => {
        assert.match(heading.textHex, /^(?:[a-f0-9]{2})*$/);
        const text = new TextDecoder('utf-8', { fatal: true })
          .decode(Buffer.from(heading.textHex, 'hex'));
        return {
          level: heading.level, start: heading.start, end: heading.end,
          sectionEnd: heading.sectionEnd, parent: heading.parent, text,
        };
      });
      assert.deepEqual(headings, nativeOutlineRows(checkedCommand(native, [], bytes)),
        'actual WIT-owned headings must match same native Markdown parser');
      const shown = await renderWithCompiledToggle({
        wasm, expanded: 1, profile: 'granted',
        sourceBytes: Buffer.from(granted.sourceHex, 'hex'), headings,
        backingReads: granted.backingReads,
      });
      const collapsed = await renderWithCompiledToggle({
        wasm, expanded: 0, profile: 'granted', sourceBytes: bytes, headings,
        backingReads: granted.backingReads,
      });
      const inaccessible = await renderWithCompiledToggle({
        wasm, expanded: 1, profile: 'denied', sourceBytes: Buffer.alloc(0),
        headings: [], backingReads: denied.backingReads,
      });
      assert.notEqual(shown, collapsed, 'source-compiled toggle must change HTML');
      assert.match(shown, /<ol id="note-outline"/);
      assert.match(collapsed, /Outline collapsed/);
      assert.match(inaccessible, /role="alert">Document access denied/);
      assert.doesNotMatch(inaccessible, /<ol|sourceHex|Sensitive|Root|Café/);
      assert.doesNotMatch(shown, /href=|src=|<script>/,
        'projected text and relative Markdown links remain inert');

      // The same real WIT result now drives native semantic layout and pixels.
      const document = { profile: 'granted', sourceBytes: bytes,
        headings, backingReads: granted.backingReads };
      const nativeExpanded = await prepareNoteScene({ wasm, expanded: 1, ...document });
      const nativeCollapsed = await prepareNoteScene({ wasm, expanded: 0, ...document });
      const nativeDenied = await prepareNoteScene({ wasm, expanded: 1,
        profile: 'denied', sourceBytes: Buffer.alloc(0),
        headings: [], backingReads: denied.backingReads });
      const sourceMutated = await prepareNoteScene({ wasm: changedWasm, expanded: 1, ...document });
      assert.equal(nativeExpanded.html, shown);
      assert.equal(nativeCollapsed.html, collapsed);
      assert.equal(nativeDenied.html, inaccessible);
      assert.equal(sourceMutated.html, collapsed);

      const expandedFrame = parseNativeScene(
        checkedCommand(layoutDriver, ['--report'], nativeExpanded.input));
      const collapsedFrame = parseNativeScene(
        checkedCommand(layoutDriver, ['--report'], nativeCollapsed.input));
      const deniedFrame = parseNativeScene(
        checkedCommand(layoutDriver, ['--report'], nativeDenied.input));
      assert.equal(expandedFrame.profile, 'granted');
      assert.equal(collapsedFrame.profile, 'granted');
      assert.equal(deniedFrame.profile, 'denied');
      for (const frame of [expandedFrame, collapsedFrame, deniedFrame]) {
        assert.deepEqual([frame.width, frame.height], [320, 160]);
        assert.ok(frame.scrollHeight >= 160);
      }
      assert.deepEqual(expandedFrame.anchors.map(anchor => anchor.text),
        ['Note preview', ...headings.map(heading => heading.text)],
        'native anchors must retain the real WIT labels');
      assert.deepEqual(collapsedFrame.anchors.map(anchor => anchor.text),
        ['Note preview', 'Outline collapsed.']);
      assert.deepEqual(deniedFrame.anchors.map(anchor => anchor.text),
        ['Note preview', 'Document access denied.']);
      for (const [index, heading] of headings.entries()) {
        const anchor = expandedFrame.anchors[index + 1];
        assert.equal(anchor.node, index + 4);
        assert.deepEqual([anchor.start, anchor.end, anchor.level],
          [heading.start, heading.end, heading.level]);
        assert.equal(anchor.text, heading.text);
      }
      assert.equal(expandedFrame.anchors[1].x, 24 + (headings[0].level - 1) * 8);
      assert.equal(expandedFrame.anchors[1].y, 40);
      const expandedPpm = checkedScenePpm(layoutDriver, nativeExpanded.input);
      const collapsedPpm = checkedScenePpm(layoutDriver, nativeCollapsed.input);
      const deniedPpm = checkedScenePpm(layoutDriver, nativeDenied.input);
      const changedPpm = checkedScenePpm(layoutDriver, sourceMutated.input);
      assert.deepEqual(ppmRgb(expandedPpm, 24, 40), [87, 164, 118]);
      assert.deepEqual(ppmRgb(collapsedPpm, 24, 40), [160, 160, 160]);
      assert.deepEqual(ppmRgb(deniedPpm, 24, 40), [192, 80, 80]);
      assert.notDeepEqual(expandedPpm, collapsedPpm);
      assert.notDeepEqual(expandedPpm, deniedPpm);
      assert.deepEqual(changedPpm, collapsedPpm,
        'mutated compiled JS must change native RGB pixels, not only HTML');
      assert.doesNotMatch(nativeDenied.input.toString('utf8'), /Root|Sensitive|Café|子/);
      assert.ok(!deniedFrame.anchors.some(anchor =>
        headings.some(heading => anchor.text === heading.text)));
    }
  });

function parseNativeScene(output) {
  const [header, ...rows] = output.trimEnd().split('\n');
  const fields = header.split('\t');
  assert.equal(fields.length, 6);
  assert.equal(fields[0], 'A1FRAME1');
  const [, profile, width, height, scrollHeight, count] = fields;
  assert.ok(profile === 'granted' || profile === 'denied');
  assert.equal(rows.length, Number(count));
  const anchors = rows.map(row => {
    const columns = row.split('\t');
    assert.equal(columns.length, 10);
    const [tag, node, x, y, w, h, start, end, level, textHex] = columns;
    assert.equal(tag, 'T');
    assert.match(textHex, /^(?:[a-f0-9]{2})*$/);
    return {
      node: Number(node), x: Number(x), y: Number(y),
      width: Number(w), height: Number(h),
      start: Number(start), end: Number(end), level: Number(level),
      text: new TextDecoder('utf-8', { fatal: true })
        .decode(Buffer.from(textHex, 'hex')),
    };
  });
  return { profile, width: Number(width), height: Number(height),
    scrollHeight: Number(scrollHeight), anchors };
}

function checkedScenePpm(executable, input) {
  const result = spawnSync(executable, ['--ppm'], {
    cwd: projectRoot, input, shell: false, encoding: null,
    timeout: 30_000, killSignal: 'SIGKILL', maxBuffer: 512 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr.toString('utf8'));
  const header = Buffer.from('P6\n320 160\n255\n');
  assert.deepEqual(result.stdout.subarray(0, header.length), header);
  assert.equal(result.stdout.length, header.length + 320 * 160 * 3);
  return result.stdout;
}

function ppmRgb(ppm, x, y) {
  const offset = Buffer.byteLength('P6\n320 160\n255\n') + (320 * y + x) * 3;
  return [...ppm.subarray(offset, offset + 3)];
}

test('native A1 scene rejects forged denial and malformed WIT rows before painting',
  { timeout: 120_000 }, () => {
    const executable = nativeExample('a1_note_scene');
    const cases = [
      'A1SCENE1\tdenied\t0\t1\t0\n1\t0\t1\t1\t-1\t61\n',
      'A1SCENE1\tdenied\t1\t0\t0\n',
      'A1SCENE1\tgranted\t4097\t0\t1\n',
      'A1SCENE1\tgranted\t20\t1\t0\n1\t0\t5\t20\t-1\t61\n',
      'A1SCENE1\tgranted\t20\t1\t1\n7\t0\t5\t20\t-1\t61\n',
      'A1SCENE1\tgranted\t20\t1\t1\n1\t0\t5\t20\t0\t61\n',
      'A1SCENE1\tgranted\t20\t1\t1\n1\t0\t5\t20\t-1\tff\n',
      'A1SCENE1\tgranted\t20\t1\t1\n1\t0\t5\t20\t-1\t61\nextra\n',
    ];
    for (const [index, input] of cases.entries()) {
      const result = spawnSync(executable, ['--ppm'], {
        cwd: projectRoot, input, shell: false, encoding: null,
        timeout: 30_000, killSignal: 'SIGKILL', maxBuffer: 256 * 1024,
      });
      if (result.error) throw result.error;
      assert.equal(result.signal, null);
      assert.notEqual(result.status, 0, 'rejected scene case ' + index);
      assert.equal(result.stdout.length, 0, 'failed scene must not paint pixels');
    }
  });


test('single-profile runnable A1 host emits HTML, native PPM and bounded debug receipts',
  { timeout: 180_000 }, async t => {
    const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-run-'));
    t.after(() => rm(cwd, { recursive: true, force: true }));
    const note = join(cwd, 'unicode.md');
    const output = join(cwd, 'granted-result');
    await writeFile(note, '# Shared docs\r\n## 子😀\n[private](../secret.md)\n');
    const result = await runPreview({ profile: 'granted', note, out: output,
      expanded: 1, debug: true });
    assert.equal(result.profile, 'granted');
    assert.equal(result.headings, 2);
    assert.equal(result.visible, 2);
    const html = await readFile(join(output, 'index.html'), 'utf8');
    assert.match(html, /Shared docs/);
    assert.match(html, /子😀/);
    assert.doesNotMatch(html, /href=|src=|<script>/);
    const ppm = await readFile(join(output, 'preview.ppm'));
    assert.deepEqual(ppmRgb(ppm, 24, 40), [87, 164, 118]);
    const debug = JSON.parse(await readFile(join(output, 'debug.json'), 'utf8'));
    assert.equal(debug.schemaVersion, 1);
    assert.equal(debug.profile, 'granted');
    assert.deepEqual([debug.attemptedReads, debug.backingReads, debug.cleanups], [1, 1, 1]);
    assert.deepEqual([debug.headingCount, debug.visibleHeadingCount], [2, 2]);
    assert.match(debug.nativeFrame, /^A1FRAME1\tgranted\t/);
    assert.ok(!JSON.stringify(debug).includes(note),
      'debug receipt does not contain the source filesystem path');
    assert.deepEqual((await readdir(output)).sort(),
      ['debug.json', 'index.html', 'preview.ppm']);

    // Do not let another run overwrite previously published results.
    await assert.rejects(
      runPreview({ profile: 'granted', note, out: output }), /already exists/);
    assert.equal(await readFile(join(output, 'index.html'), 'utf8'), html);
  });

test('CLI from unrelated cwd selects only denied WIT profile without reading note path',
  { timeout: 180_000 }, async t => {
    const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-denial-'));
    t.after(() => rm(cwd, { recursive: true, force: true }));
    const sourceThatMustNotBeOpened = join(cwd, 'missing-secret.md');
    const runner = fileURLToPath(new URL('../../examples/note-preview/run.mjs', import.meta.url));
    const output = join(cwd, 'denied-result');
    const result = spawnSync(process.execPath, [
      runner, '--profile', 'denied', '--note', sourceThatMustNotBeOpened,
      '--out', output, '--debug',
    ], { cwd, encoding: 'utf8', shell: false,
      timeout: 120_000, maxBuffer: 1024 * 1024 });
    if (result.error) throw result.error;
    assert.equal(result.signal, null);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(result.stdout).profile, 'denied');
    const html = await readFile(join(output, 'index.html'), 'utf8');
    assert.match(html, /Document access denied/);
    assert.doesNotMatch(html, /Shared docs|secret|<ol/);
    const debug = JSON.parse(await readFile(join(output, 'debug.json'), 'utf8'));
    assert.equal(debug.profile, 'denied');
    assert.deepEqual([debug.attemptedReads, debug.backingReads, debug.cleanups,
      debug.headingCount, debug.visibleHeadingCount], [1, 0, 1, 0, 0]);
    assert.doesNotMatch(debug.nativeFrame, /Shared docs|secret|子😀/);
    assert.deepEqual(ppmRgb(await readFile(join(output, 'preview.ppm')), 24, 40),
      [192, 80, 80]);
    assert.ok(!JSON.stringify(debug).includes(sourceThatMustNotBeOpened));

    const collapsedPath = join(cwd, 'collapsed-result');
    await assert.rejects(
      runPreview({ profile: 'granted', note: null, out: collapsedPath }),
      /Invalid note-preview/);
    await assert.rejects(access(collapsedPath), { code: 'ENOENT' });
  });

test('A1 runner rejects malformed args and oversized or invalid UTF-8 notes without publishing',
  { timeout: 120_000 }, async t => {
    const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-invalid-'));
    t.after(() => rm(cwd, { recursive: true, force: true }));
    for (const args of [
      [], ['--profile', 'granted'], ['--profile', 'other', '--out', 'out'],
      ['--profile', 'denied', '--profile', 'granted', '--out', 'out'],
      ['--profile', 'granted', '--note', 'source.md', '--out'],
      ['--profile', 'denied', '--out', 'out', '--unknown'],
    ]) {
      assert.throws(() => parseArguments(args), /usage/);
    }
    const large = join(cwd, 'large.md');
    const invalid = join(cwd, 'invalid.md');
    await writeFile(large, Buffer.alloc(4097, 0x41));
    await writeFile(invalid, Buffer.from([0xff]));
    for (const [note, label] of [[large, 'oversized'], [invalid, 'invalid']]) {
      const out = join(cwd, label);
      await assert.rejects(runPreview({ profile: 'granted', note, out }));
      await assert.rejects(access(out), { code: 'ENOENT' });
    }
    const denied = join(cwd, 'denied-minimal');
    const receipt = await runPreview({ profile: 'denied', out: denied });
    assert.equal(receipt.profile, 'denied');
    assert.deepEqual((await readdir(denied)).sort(), ['index.html', 'preview.ppm']);
  });


test('A1 development build covers exactly the required existing guest, hosts and compiler', () => {
  assert.deepEqual(buildCommands.map(args => args.join(' ')), [
    'build -p grunkalunka-cli --locked --offline',
    'build -p grunkalunka-markdown --example outline_guest --locked --offline',
    'build -p grunkalunka-markdown --example outline_guest --target wasm32-unknown-unknown --locked --offline',
    'build -p grunkalunka-runtime --example a1_note_preview --locked --offline',
    'build -p grunkalunka-layout --example a1_note_scene --locked --offline',
  ]);
});


test('A1 host bundle retains portable Wasm and runs independently of the source checkout',
  { timeout: 180_000 }, async t => {
    const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka-a1-bundle-'));
    t.after(() => rm(cwd, { recursive: true, force: true }));
    const bundle = join(cwd, 'package é & spaces');
    const note = join(cwd, 'source.md');
    await writeFile(note, '# Bundled heading\r\n## 子😀\n[relative](../hidden.md)\n');

    assert.deepEqual(parsePackageArguments(['--out', bundle]), { out: bundle });
    for (const args of [[], ['--out'], ['--out', bundle, '--debug'], ['--out', '']]) {
      assert.throws(() => parsePackageArguments(args), /usage/);
    }
    const packaged = await packageNotePreview({ out: bundle });
    assert.equal(packaged.destination, bundle);
    assert.equal(packaged.manifest.schemaVersion, 1);
    assert.equal(packaged.manifest.kind, 'a1-note-preview-host-bundle');
    assert.deepEqual(packaged.manifest.host, {
      platform: process.platform, arch: process.arch,
      nodeMajor: Number(process.versions.node.split('.')[0]),
    });
    const entries = packaged.manifest.files;
    assert.equal(entries.length, 12);
    assert.deepEqual(entries.filter(item => item.role === 'portable-wasm')
      .map(item => item.path).sort(), [
      'artifacts/toggle.wasm',
      'target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm',
    ]);
    assert.deepEqual(await readFile(join(bundle, 'target',
      'wasm32-unknown-unknown/debug/examples/outline_guest.wasm')),
    await readFile(join(targetRoot, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm')));
    const selector = await readFile(join(bundle, 'artifacts/toggle.wasm'));
    assert.ok(WebAssembly.validate(selector));
    assert.deepEqual(WebAssembly.Module.imports(new WebAssembly.Module(selector)), []);

    // Neither compiler binary, build API nor JS toggle source is required
    // to invoke the packaged result. All source imports resolve within bundle.
    await assert.rejects(access(join(bundle, 'target/debug',
      'grunkalunka' + (process.platform === 'win32' ? '.exe' : ''))),
    { code: 'ENOENT' });
    await assert.rejects(access(join(bundle, 'tools/build/compile.mjs')), { code: 'ENOENT' });
    await assert.rejects(access(join(bundle, 'examples/note-preview/toggle.js')), { code: 'ENOENT' });
    const entry = join(bundle, 'examples/note-preview/bundle-entry.mjs');
    function packagedRun(args, selectedEntry = entry) {
      const result = spawnSync(process.execPath, [selectedEntry, ...args], {
        cwd, shell: false, encoding: 'utf8', timeout: 120_000,
        maxBuffer: 1024 * 1024,
        env: { ...process.env, CARGO_TARGET_DIR: join(cwd, 'nonexistent-target') },
      });
      if (result.error) throw result.error;
      assert.equal(result.signal, null);
      return result;
    }

    const granted = join(cwd, 'installed-grant');
    const invocation = packagedRun([
      '--profile', 'granted', '--note', note, '--out', granted, '--debug',
    ]);
    assert.equal(invocation.status, 0, invocation.stderr);
    assert.equal(JSON.parse(invocation.stdout).visible, 2);
    const html = await readFile(join(granted, 'index.html'), 'utf8');
    assert.match(html, /Bundled heading/);
    assert.match(html, /子😀/);
    assert.doesNotMatch(html, /href=|src=/);
    const ppm = await readFile(join(granted, 'preview.ppm'));
    assert.deepEqual(ppmRgb(ppm, 24, 40), [87, 164, 118]);
    const info = JSON.parse(await readFile(join(granted, 'debug.json'), 'utf8'));
    assert.deepEqual([info.attemptedReads, info.backingReads, info.cleanups], [1, 1, 1]);
    assert.ok(!JSON.stringify(info).includes(note));

    const denied = join(cwd, 'installed-denied');
    const denial = packagedRun([
      '--profile', 'denied', '--note', join(cwd, 'missing-secret.md'),
      '--out', denied, '--debug',
    ]);
    assert.equal(denial.status, 0, denial.stderr);
    assert.match(await readFile(join(denied, 'index.html'), 'utf8'), /Document access denied/);
    assert.deepEqual(ppmRgb(await readFile(join(denied, 'preview.ppm')), 24, 40),
      [192, 80, 80]);
    const deniedInfo = JSON.parse(await readFile(join(denied, 'debug.json'), 'utf8'));
    assert.deepEqual([deniedInfo.attemptedReads, deniedInfo.backingReads, deniedInfo.cleanups],
      [1, 0, 1]);
    assert.ok(!JSON.stringify(deniedInfo).includes('missing-secret'));

    // A symlinked entry path must still run the CLI. Node may canonicalize
    // import.meta.url while process.argv[1] retains the symlinked path; on
    // macOS, even tmpdir() commonly traverses /var -> /private/var.
    if (process.platform !== 'win32') {
      const alias = join(cwd, 'linked-bundle');
      await symlink(bundle, alias, 'dir');
      const linkedOutput = join(cwd, 'linked-denied');
      const linked = packagedRun([
        '--profile', 'denied', '--out', linkedOutput,
      ], join(alias, 'examples/note-preview/bundle-entry.mjs'));
      assert.equal(linked.status, 0, linked.stderr);
      assert.equal(JSON.parse(linked.stdout).profile, 'denied');
      assert.match(await readFile(join(linkedOutput, 'index.html'), 'utf8'),
        /Document access denied/);
    }

    // Packaging refuses an existing destination and cannot damage it.
    await assert.rejects(packageNotePreview({ out: bundle }), /already exists/);
    assert.deepEqual(await readFile(join(granted, 'preview.ppm')), ppm);

    // Corrupted Wasm must be rejected *before* loading app runtime modules or
    // touching a note/output path. This is accidental corruption detection,
    // not authenticated publisher identity.
    await writeFile(join(bundle, 'artifacts/toggle.wasm'), Buffer.from([0, 97]));
    const afterCorruption = join(cwd, 'should-not-exist');
    const failure = packagedRun([
      '--profile', 'granted', '--note', note, '--out', afterCorruption,
    ]);
    assert.notEqual(failure.status, 0);
    assert.match(failure.stderr, /checksum mismatch/);
    await assert.rejects(access(afterCorruption), { code: 'ENOENT' });
  });
