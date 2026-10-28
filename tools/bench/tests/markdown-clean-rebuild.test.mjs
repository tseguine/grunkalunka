import assert from 'node:assert/strict';
import test from 'node:test';
import { classifyCleanRebuilds } from '../markdown-clean-rebuild.mjs';

const hashes = {
  a: 'a'.repeat(64), b: 'b'.repeat(64), c: 'c'.repeat(64),
};
const snapshot = (phase, sha256 = hashes.a, bytes = 735367) =>
  ({ phase, sha256, bytes });
const sequence = () => [
  snapshot('first-clean'),
  snapshot('unchanged-invocation'),
  snapshot('second-clean'),
];

test('same artifact across clean compiles is reported as a stable observation', () => {
  assert.deepEqual(classifyCleanRebuilds(sequence()), {
    unchangedInvocationEqual: true,
    cleanBuildsEqual: true,
    classification: 'stable-within-host-rebuild',
  });
});

test('different results after a clean rebuild are recorded rather than suppressed', () => {
  const observations = sequence();
  observations[2] = snapshot('second-clean', hashes.b);
  assert.deepEqual(classifyCleanRebuilds(observations), {
    unchangedInvocationEqual: true,
    cleanBuildsEqual: false,
    classification: 'changed-after-clean',
  });
});

test('a changed unchanged-input invocation is not misreported as a clean-only difference', () => {
  const observations = sequence();
  observations[1] = snapshot('unchanged-invocation', hashes.b);
  observations[2] = snapshot('second-clean', hashes.c);
  const result = classifyCleanRebuilds(observations);
  assert.equal(result.unchangedInvocationEqual, false);
  assert.equal(result.cleanBuildsEqual, false);
  assert.equal(result.classification, 'changed-without-clean');
});

test('artifact byte-size changes are distinguished even if a digest were reused', () => {
  const observations = sequence();
  observations[2].bytes -= 8;
  assert.equal(classifyCleanRebuilds(observations).cleanBuildsEqual, false);
});

test('rejects missing, misordered or malformed snapshots', () => {
  for (const observations of [
    [], sequence().slice(1), [...sequence(), snapshot('second-clean')],
    [sequence()[1], sequence()[0], sequence()[2]],
  ]) assert.throws(() => classifyCleanRebuilds(observations), /Invalid/);
  for (const bad of [
    { sha256: '', bytes: 100 },
    { sha256: 'f'.repeat(63), bytes: 100 },
    { sha256: hashes.a, bytes: 8 },
    { sha256: hashes.a, bytes: 16 * 1024 * 1024 + 1 },
    { sha256: hashes.a, bytes: 2.5 },
    { sha256: hashes.a, bytes: null },
  ]) {
    const observations = sequence();
    observations[2] = { phase: 'second-clean', ...bad };
    assert.throws(() => classifyCleanRebuilds(observations), /Invalid/);
  }
});
