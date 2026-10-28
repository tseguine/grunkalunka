import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import {
  compareBuildPaths, summarizePathControls,
} from '../markdown-linux-checkout-path.mjs';

const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const section = (id, payload) => {
  assert.ok(payload.length < 128, 'small regression fixture');
  return Buffer.from([id, payload.length, ...payload]);
};
const code = bodies => section(10, [bodies.length,
  ...bodies.flatMap(body => [body.length, ...body])]);
const custom = (name, bytes) => {
  const label = Buffer.from(name);
  return section(0, [label.length, ...label, ...bytes]);
};
const inspect = (bodies, option = null) => {
  const sections = [code(bodies)];
  if (option) sections.push(custom('name', option));
  return inspectWasmSections(Buffer.concat([magic, ...sections]),
    { codeBodyDigests: true });
};

test('identical source outputs have equal sections and function-body digests', () => {
  const a = inspect([[0, 11], [0, 1, 11]]);
  const result = compareBuildPaths(a, structuredClone(a));
  assert.equal(result.sameArtifactBytes, true);
  assert.deepEqual(result.changedSections, []);
  assert.equal(result.producerFunctionCount, 2);
  assert.equal(result.receiverFunctionCount, 2);
  assert.equal(result.matchingAtSameOrdinal, 2);
  assert.equal(result.matchingIgnoringOrdinal, 2);
  assert.equal(result.producerUnmatched, 0);
  assert.equal(result.receiverUnmatched, 0);
});

test('function order changes alone cannot be misclassified as changed bodies', () => {
  const first = inspect([[0, 11], [0, 1, 11], [0, 2, 11]]);
  const second = inspect([[0, 2, 11], [0, 11], [0, 1, 11]]);
  const result = compareBuildPaths(first, second);
  assert.equal(result.sameArtifactBytes, false);
  assert.deepEqual(result.changedSections, ['section:10']);
  assert.equal(result.matchingAtSameOrdinal, 0);
  assert.equal(result.matchingIgnoringOrdinal, 3);
  assert.equal(result.producerUnmatched, 0);
});

test('genuine raw code byte changes survive ignoring function order', () => {
  const first = inspect([[0, 11], [0, 1, 11]]);
  const second = inspect([[0, 11], [0, 2, 11]]);
  const result = compareBuildPaths(first, second);
  assert.equal(result.matchingAtSameOrdinal, 1);
  assert.equal(result.matchingIgnoringOrdinal, 1);
  assert.equal(result.producerUnmatched, 1);
  assert.equal(result.receiverUnmatched, 1);
});

test('custom section differences are separate from code body identity', () => {
  const first = inspect([[0, 11]], [1]);
  const second = inspect([[0, 11]], [2]);
  const result = compareBuildPaths(first, second);
  assert.deepEqual(result.changedSections, ['custom:name']);
  assert.equal(result.matchingAtSameOrdinal, 1);
  assert.equal(result.matchingIgnoringOrdinal, 1);
  assert.equal(result.sameArtifactBytes, false);
});

test('different function counts mark ordinal alignment unavailable', () => {
  const first = inspect([[0, 11], [0, 1, 11]]);
  const second = inspect([[0, 11]]);
  const result = compareBuildPaths(first, second);
  assert.equal(result.matchingAtSameOrdinal, null);
  assert.equal(result.matchingIgnoringOrdinal, 1);
  assert.equal(result.producerUnmatched, 1);
  assert.equal(result.receiverUnmatched, 0);
});

test('the four-build matrix distinguishes path and remapping effects', () => {
  const a = inspect([[0, 11]]);
  const b = inspect([[0, 1, 11]]);
  const matrix = summarizePathControls({
    defaultA: a, defaultB: b, remappedA: a, remappedB: a,
  });
  assert.equal(matrix.defaultAcrossPaths.sameArtifactBytes, false);
  assert.equal(matrix.remappedAcrossPaths.sameArtifactBytes, true);
  assert.equal(matrix.samePathDefaultToRemapped.a.sameArtifactBytes, true);
  assert.equal(matrix.samePathDefaultToRemapped.b.sameArtifactBytes, false);
  assert.throws(() => summarizePathControls({
    defaultA: a, defaultB: b, remappedA: a,
  }), /complete four-build matrix/);
});

test('corrupt or oversized fingerprint metadata is rejected', () => {
  const first = inspect([[0, 11]]), second = structuredClone(first);
  second.codeBodies.bodyDigestsBase64 = 'bogus!';
  assert.throws(() => compareBuildPaths(first, second), /canonical raw body fingerprints/);
  second.codeBodies.bodyDigestsBase64 =
    Buffer.from([1]).toString('base64');
  assert.throws(() => compareBuildPaths(first, second), /canonical raw body fingerprints/);
  second.codeBodies.bodyDigestsBase64 = first.codeBodies.bodyDigestsBase64;
  second.codeBodies.lengths[0] = -1;
  assert.throws(() => compareBuildPaths(first, second), /canonical raw body fingerprints/);
  second.codeBodies.lengths = [2];
  second.artifact.bytes = 16 * 1024 * 1024 + 1;
  assert.throws(() => compareBuildPaths(first, second), /fingerprint envelope/);
});
