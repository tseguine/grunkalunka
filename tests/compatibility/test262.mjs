import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const directory = new URL('./test262/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', directory), 'utf8'));
function retained(path, expectedHash) {
  const bytes = readFileSync(new URL(path, directory));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), expectedHash,
    `Test262 source hash mismatch: ${path}`);
  return bytes.toString('utf8');
}
retained('originals/LICENSE', manifest.licenseSha256);

export function assertionBody(checks) {
  return checks.map(check => `${check.setup} if (${check.condition}) return ${check.failure};`).join('\n') + '\nreturn 0;';
}

export const test262Programs = manifest.cases.map(item => {
  const source = retained(item.retainedPath, item.sourceSha256).split(/\r?\n/);
  assert.deepEqual(item.checks.map(check => check.sourceLine),
    source.flatMap((line, index) => line.trim().startsWith('if (') ? [index + 1] : []),
    `incomplete Test262 assertion mapping: ${item.id}`);
  for (const [index, check] of item.checks.entries()) {
    assert.equal(check.failure, index + 1);
    assert.equal(source[check.sourceLine - 1].trim(), `if (${check.originalCondition}) {`);
  }
  const body = assertionBody(item.checks);
  return [item.id, body, body, 0];
});
export const test262Source = manifest;
