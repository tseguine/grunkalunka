import assert from 'node:assert/strict';
import { test } from 'node:test';
import { escapeHtml, renderNotePreview } from '../../examples/note-preview/render.mjs';

const sourceBytes = Buffer.from('# Root\r\n## 子😀\n', 'utf8');
const headings = [
  { level: 1, start: 0, end: 8, sectionEnd: sourceBytes.length,
    parent: null, text: 'Root' },
  { level: 2, start: 8, end: sourceBytes.length, sectionEnd: sourceBytes.length,
    parent: 0, text: '子😀' },
];
const granted = (visibleCount = headings.length) => ({
  profile: 'granted', sourceBytes, headings, backingReads: 1, visibleCount,
});

test('granted note preview displays typed headings with independent byte positions', () => {
  const html = renderNotePreview(granted());
  assert.match(html, /^<!doctype html>\n<html lang="en">/);
  assert.match(html, /<ol id="note-outline" aria-label="Note headings">/);
  assert.match(html, /data-level="1" data-start="0" data-end="8" data-section-end="19" data-parent="-1">Root<\/li>/);
  assert.match(html, /data-level="2" data-start="8" data-end="19" data-section-end="19" data-parent="0">子😀<\/li>/);
  assert.ok(html.indexOf('>Root</li>') < html.indexOf('>子😀</li>'));
});

test('collapsed state changes the actual rendered content without losing source', () => {
  const expanded = renderNotePreview(granted(2));
  const one = renderNotePreview(granted(1));
  const collapsed = renderNotePreview(granted(0));
  assert.ok(expanded.includes('子😀'));
  assert.ok(!one.includes('子😀') && one.includes('>Root</li>'));
  assert.ok(collapsed.includes('Outline collapsed.'));
  assert.ok(!collapsed.includes('<li'));
  assert.notEqual(expanded, collapsed);
});

test('denied profile renders no note data and refuses disclosed protected content', () => {
  const confidential = Buffer.from('# Private sentinel\n', 'utf8');
  const html = renderNotePreview({
    profile: 'denied', sourceBytes: confidential,
    headings: [], backingReads: 0, visibleCount: 0,
  });
  assert.match(html, /role="alert">Document access denied/);
  assert.ok(!html.includes('Private sentinel'));
  assert.ok(!html.includes('<ol'));
  assert.throws(() => renderNotePreview({
    profile: 'denied', sourceBytes: confidential, headings,
    backingReads: 0, visibleCount: 0,
  }), /Denied profile/);
  assert.throws(() => renderNotePreview({
    profile: 'denied', sourceBytes: confidential, headings: [],
    backingReads: 1, visibleCount: 0,
  }), /Denied profile/);
});

test('escaped labels cannot inject markup or attributes', () => {
  assert.equal(escapeHtml('<img src="x" onerror=\'bad\'>&\u0000'),
    '&lt;img src=&quot;x&quot; onerror=&#39;bad&#39;&gt;&amp;\uFFFD');
  const html = renderNotePreview({
    profile: 'granted', sourceBytes: Buffer.from('anything'),
    headings: [{ level: 1, start: 0, end: 8, sectionEnd: 8,
      parent: null, text: '<script>alert("x")</script>' }],
    backingReads: 1, visibleCount: 1,
  });
  assert.ok(html.includes('&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;'));
  assert.ok(!html.includes('<script>'));
});

test('malformed grants, byte ranges, list ordering and excessive output fail closed', () => {
  const bad = [
    { ...granted(), profile: 'unknown' },
    { ...granted(), backingReads: 0 },
    { ...granted(), visibleCount: 3 },
    { ...granted(), headings: headings.map((h, i) => i ? { ...h, parent: 1 } : h) },
    { ...granted(), headings: headings.map((h, i) => i ? { ...h, start: 0 } : h) },
    { ...granted(), headings: [{ ...headings[0], end: sourceBytes.length + 1 }] },
    { ...granted(), headings: [{ ...headings[0], text: 'é'.repeat(2049) }] },
    { ...granted(), sourceBytes: Buffer.alloc(4097) },
    { ...granted(), sourceBytes: Uint8Array.of(0xff) },
    { ...granted(), headings: Array.from({ length: 65 }, () => headings[0]) },
  ];
  for (const input of bad) {
    assert.throws(() => renderNotePreview(input), undefined, String(input.profile));
  }
  assert.throws(() => escapeHtml(42), /Expected heading text/);
});
