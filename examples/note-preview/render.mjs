// A1 presentation-only experiment. Host authorization and typed WIT lifting are external.
// No DOM/browser implementation or filesystem capability is implied by this module.
const decoder = new TextDecoder('utf-8', { fatal: true });
const encoder = new TextEncoder();
const MAX_INPUT = 4096;
const MAX_HEADINGS = 64;
const MAX_LABEL_BYTES = 4096;

function boundedInt(value, min, max, name) {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new RangeError("Invalid " + name);
  }
  return value;
}

export function escapeHtml(text) {
  if (typeof text !== 'string') throw new TypeError('Expected heading text');
  return text.replace(/\u0000/g, '\uFFFD').replace(/[&<>"']/g, character => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  })[character]);
}

export function renderNotePreview({ profile, sourceBytes, headings, backingReads, visibleCount }) {
  if (profile !== 'granted' && profile !== 'denied') {
    throw new TypeError('Expected granted or denied profile');
  }
  if (!(sourceBytes instanceof Uint8Array)) {
    throw new TypeError('Expected explicit UTF-8 note bytes');
  }
  boundedInt(sourceBytes.byteLength, 0, MAX_INPUT, 'note bytes');
  decoder.decode(sourceBytes);
  if (!Array.isArray(headings) || headings.length > MAX_HEADINGS) {
    throw new RangeError('Invalid heading list');
  }
  boundedInt(backingReads, 0, 1, 'backing reads');
  const show = boundedInt(visibleCount, 0, headings.length, 'visible headings');

  if (profile === 'denied') {
    if (backingReads !== 0 || headings.length !== 0 || show !== 0) {
      throw new Error('Denied profile must contain no disclosed headings or reads');
    }
  } else if (backingReads !== 1) {
    throw new Error('Granted preview requires one authorized backing read');
  }

  let totalLabelBytes = 0;
  let lastStart = 0;
  for (const [index, entry] of headings.entries()) {
    if (!entry || typeof entry !== 'object' || typeof entry.text !== 'string') {
      throw new TypeError('Expected owned heading records');
    }
    boundedInt(entry.level, 1, 6, 'heading level');
    boundedInt(entry.start, 0, sourceBytes.byteLength, 'heading start');
    boundedInt(entry.end, entry.start, sourceBytes.byteLength, 'heading end');
    boundedInt(entry.sectionEnd, entry.end, sourceBytes.byteLength, 'section end');
    if (index && entry.start < lastStart) throw new RangeError('Headings must be source ordered');
    lastStart = entry.start;
    if (entry.parent !== null) {
      boundedInt(entry.parent, 0, index - 1, 'parent index');
      if (headings[entry.parent]?.level >= entry.level) {
        throw new RangeError('Parent heading must have a smaller level');
      }
    }
    totalLabelBytes += encoder.encode(entry.text).byteLength;
    if (totalLabelBytes > MAX_LABEL_BYTES) throw new RangeError('Heading projection quota');
  }

  let body;
  if (profile === 'denied') {
    body = '    <p role="alert">Document access denied.</p>';
  } else if (show === 0) {
    body = '    <p id="outline-collapsed">Outline collapsed.</p>';
  } else {
    const items = headings.slice(0, show).map(entry =>
      '      <li data-level="' + entry.level + '" data-start="' + entry.start +
      '" data-end="' + entry.end + '" data-section-end="' + entry.sectionEnd +
      '" data-parent="' + (entry.parent ?? -1) + '">' + escapeHtml(entry.text) + '</li>');
    body = '    <ol id="note-outline" aria-label="Note headings">\n' +
      items.join('\n') + '\n    </ol>';
  }

  return '<!doctype html>\n' +
    '<html lang="en">\n' +
    '  <head><meta charset="utf-8"><title>Note preview</title></head>\n' +
    '  <body>\n' +
    '    <main><h1>Note preview</h1>\n' +
    body + '\n' +
    '    </main>\n' +
    '  </body>\n' +
    '</html>\n';
}
