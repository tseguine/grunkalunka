import assert from 'node:assert/strict';
import { programSources } from './program-sources.mjs';

// An origin describes the exact original source, independently of whether that
// source still belongs to the numeric rejection catalog after a promotion.
export function resolveNumericOrigin(fixture, rejected, programs) {
  if (fixture.numericOrigin === null) return null;
  const id = fixture.numericOrigin;
  assert.equal(typeof id, 'string', 'numeric origin ID');
  const negatives = rejected.filter(row => row[0] === id);
  const positives = programs.filter(row => row[0] === id);
  assert.equal(negatives.length + positives.length, 1, `unique numeric origin: ${id}`);
  const status = negatives.length ? 'rejected' : 'supported';
  const sourceJS = negatives.length ? negatives[0][1] : programSources(positives[0]).sourceJS;
  assert.equal(fixture.sourceJS, sourceJS, `unchanged numeric origin source: ${id}`);
  return {id, status, sourceJS};
}

// Reviewed finite profile totals; a fast catalog test detects stale counts before
// the process-heavy differential checker is started.
export const primitiveProfileCounts = Object.freeze({
  pairedPrograms: 200, rejectedPairs: 1, numericOriginCases: 16,
  numericSupportedOriginCases: 2, rejections: 2, comparisons: 11256,
});
