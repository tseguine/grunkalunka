// Private A1 host-side presentation seam. Authorization and WIT lifting belong
// to the component host; the HTML is inert and does not resolve resources.
import { runNumeric } from '../../tools/runtime/numeric.mjs';

const decoder = new TextDecoder('utf-8', { fatal: true });
const encoder = new TextEncoder();
const MAX_SOURCE_BYTES = 4096;
const MAX_HEADINGS = 64;
const MAX_LABEL_BYTES = 4096;

function boundedInteger(value, min, max, label) {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new RangeError(`Invalid ${label}`);
  }
  return value;
}

export function escapeHtml(text) {
  if (typeof text !== 'string') throw new TypeError('Heading text must be a string');
  return text.replace(/\u0000/g, '\uFFFD').replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[character]);
}

// 'headings' is the trusted host's lifted WIT outline (not parsed Markdown or
// a caller-supplied authorization token). The source is passed explicitly.
export function renderNotePreview({ profile, sourceBytes, headings, backingReads, visibleCount }) {
  if (profile !== 'granted' && profile !== 'denied') {
    throw new TypeError('Unknown document profile');
  }
  if (!(sourceBytes instanceof Uint8Array)) {
    throw new TypeError('Note must be explicit UTF-8 bytes');
  }
  boundedInteger(sourceBytes.byteLength, 0, MAX_SOURCE_BYTES, 'source length');
  decoder.decode(sourceBytes);
  if (!Array.isArray(headings) || headings.length > MAX_HEADINGS) {
    throw new RangeError('Invalid heading list');
  }
  boundedInteger(backingReads, 0, 1, 'backing read count');
  const show = boundedInteger(visibleCount, 0, headings.length, 'visible count');

  if (profile === 'denied') {
    if (backingReads !== 0 || headings.length !== 0 || show !== 0) {
      throw new Error('Denied profile disclosed protected data');
    }
    return page('    <p role="alert">Document access denied.</p>');
  }
  if (backingReads !== 1) throw new Error('Granted profile requires exactly one backing read');

  let labelBytes = 0;
  let previousEnd = 0;
  const ancestors = [];
  const expectedSectionEnds = Array(headings.length).fill(sourceBytes.byteLength);
  for (const [i, heading] of headings.entries()) {
    if (heading === null || typeof heading !== 'object' || typeof heading.text !== 'string') {
      throw new TypeError('Invalid owned WIT heading');
    }
    boundedInteger(heading.level, 1, 6, 'heading level');
    boundedInteger(heading.start, previousEnd, sourceBytes.byteLength, 'heading start');
    boundedInteger(heading.end, heading.start, sourceBytes.byteLength, 'heading end');
    boundedInteger(heading.sectionEnd, heading.end, sourceBytes.byteLength, 'section end');
    // Actual Markdown heading constructs are nonoverlapping source intervals;
    // equal starts must not silently validate a duplicated heading range.
    if (i > 0 && heading.start === headings[i - 1].start) {
      throw new RangeError('Duplicated heading source position');
    }
    previousEnd = heading.end;
    while (ancestors.length && headings[ancestors.at(-1)].level >= heading.level) {
      expectedSectionEnds[ancestors.pop()] = heading.start;
    }
    const parent = ancestors.at(-1) ?? null;
    if (heading.parent !== parent) throw new RangeError('Invalid outline ancestry');
    ancestors.push(i);
    labelBytes += encoder.encode(heading.text).byteLength;
    if (labelBytes > MAX_LABEL_BYTES) throw new RangeError('Heading label quota exceeded');
  }
  for (const [i, heading] of headings.entries()) {
    if (heading.sectionEnd !== expectedSectionEnds[i]) {
      throw new RangeError('Invalid section boundary');
    }
  }

  if (show === 0) return page('    <p id="outline-collapsed">Outline collapsed.</p>');
  const rows = headings.slice(0, show).map(heading =>
    `      <li data-level="${heading.level}" data-start="${heading.start}" ` +
    `data-end="${heading.end}" data-section-end="${heading.sectionEnd}" ` +
    `data-parent="${heading.parent ?? -1}">${escapeHtml(heading.text)}</li>`);
  return page('    <ol id="note-outline" aria-label="Note headings">\n' +
    rows.join('\n') + '\n    </ol>');
}

function page(body) {
  return '<!doctype html>\n<html lang="en">\n' +
    '  <head><meta charset="utf-8"><title>Note preview</title></head>\n' +
    '  <body>\n    <main><h1>Note preview</h1>\n' +
    body + '\n    </main>\n  </body>\n</html>\n';
}

// A *compiled* JS/TS numeric Wasm artifact selects the visible heading count.
// This does not claim source-language DOM bindings or implicit authority.
export async function renderWithCompiledToggle({ wasm, expanded, ...document }) {
  if (typeof expanded !== 'number') throw new TypeError('expanded must be numeric');
  const visibleCount = await runNumeric(wasm, expanded, document.headings.length);
  return renderNotePreview({ ...document, visibleCount });
}
