import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const directory = new URL('./benchmarks-game/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', directory), 'utf8'));
function retained(path, expectedHash) {
  const bytes = readFileSync(new URL(path, directory));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), expectedHash,
    `Benchmarks Game source hash mismatch: ${path}`);
  return bytes.toString('utf8');
}
retained('original.js', manifest.extractedSha256);
retained('LICENSE.md', manifest.licenseSha256);
const body = retained('adapted.txt', manifest.adaptedSha256);
const reference = retained('reference.txt', manifest.referenceSha256);
export const benchmarkPrograms = [[manifest.id, body, body, undefined, reference]];
export const benchmarkVectors = manifest.vectors.map(vector => ({
  ...vector, a: vector.a === 'NaN' ? NaN : vector.a, b: vector.b === 'NaN' ? NaN : vector.b,
}));
export const benchmarkSource = manifest;
