// A1-only host bridge from already-authorized WIT headings to the existing
// native semantic layout/paint example. The protocol conveys no note source.
import { runNumeric } from '../../tools/runtime/numeric.mjs';
import { renderNotePreview } from './render.mjs';

export async function prepareNoteScene({ wasm, expanded, ...document }) {
  const visibleCount = await runNumeric(wasm, expanded, document.headings.length);
  // Fail closed before creating any native layout input: this is the existing
  // bounded presentation/authority contract, not a second policy evaluator.
  const html = renderNotePreview({ ...document, visibleCount });
  const granted = document.profile === 'granted';
  const visible = granted ? document.headings.slice(0, visibleCount) : [];
  const sourceLength = granted ? document.sourceBytes.byteLength : 0;
  const lines = [
    ['A1SCENE1', document.profile, sourceLength, visible.length, document.backingReads].join('\t'),
    ...visible.map(heading => [
      heading.level, heading.start, heading.end, heading.sectionEnd,
      heading.parent ?? -1, Buffer.from(heading.text, 'utf8').toString('hex'),
    ].join('\t')),
  ];
  return { html, visibleCount, input: Buffer.from(lines.join('\n') + '\n', 'utf8') };
}
