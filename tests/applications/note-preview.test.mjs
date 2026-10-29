import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compileSource } from '../../tools/build/compile.mjs';
import { escapeHtml, renderNotePreview, renderWithCompiledToggle } from '../../examples/note-preview/render.mjs';

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
