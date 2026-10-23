// Grunkalunka adapter; extracted expressions retain the adjacent Apache-2.0 terms.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const directory = new URL('./typescript/', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('manifest.json', directory), 'utf8'));
const bytes = await readFile(new URL('expressions.txt', directory));
assert.equal(createHash('sha256').update(bytes).digest('hex'), manifest.excerptSha256,
  'TypeScript excerpt hash mismatch');
const expressions = bytes.toString('utf8').trimEnd().split('\n');
assert.equal(expressions.length, 32);
assert.equal(manifest.cases.length, expressions.length);
assert.equal(new Set(manifest.cases.map(item => item.id)).size, expressions.length);
assert.equal(new Set(manifest.cases.map(item => item.line)).size, expressions.length);
assert.equal(manifest.cases.filter(item => item.expectation === 'admitted').length, 24);
assert.equal(manifest.cases.filter(item => item.expectation === 'rejected').length, 8);

export const typescriptPrograms = [];
export const typescriptRejected = [];
for (const [index, item] of manifest.cases.entries()) {
  assert.match(item.id, /^ts-r[a-d][1-8]$/);
  assert.ok(Number.isSafeInteger(item.line) && item.line > 0);
  const expression = expressions[index];
  const match = /^([ab0]) ([*/%\-]) ([ab0])$/.exec(expression);
  assert.ok(match, `unexpected extracted expression: ${item.id}`);
  assert.equal(item.expectation, match[2] === '%' ? 'rejected' : 'admitted');
  const body = `return ${expression};`;
  if (item.expectation === 'admitted') typescriptPrograms.push([item.id, body, body]);
  else typescriptRejected.push([item.id,
    `export function run(a: number, b: number): number { ${body} }`, 'ts',
    'only binary +, -, *, /, &, |, ^, <<, >> and >>> are admitted']);
}

export const typescriptSource = {
  ...manifest,
  sourceUrl: `${manifest.repository}/blob/${manifest.revision}/${manifest.sourcePath}`,
  baselineUrl: `${manifest.repository}/blob/${manifest.revision}/${manifest.baselinePath}`,
  adaptation: 'Selected expressions returned from run(a,b); both parameters restricted to primitive numbers. No TypeScript type-checking or full-fixture conformance claim.',
};
