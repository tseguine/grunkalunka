import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import {
  inspectWasmSections, compareMarkdownGuests,
} from '../wasm-section-fingerprint.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const segment = (id, payload) => {
  assert.ok(payload.length < 128, 'fixture uses short lengths');
  return Buffer.from([id, payload.length, ...payload]);
};
const custom = (name, bytes = [1]) => {
  const label = Buffer.from(name);
  return segment(0, [label.length, ...label, ...bytes]);
};
const code = (bytes = [0]) => segment(10, bytes);
const moduleBytes = (...sections) => Buffer.concat([magic, ...sections]);
const inputs = () => [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
].map(path => ({
  path, gitSha256: '1'.repeat(64), gitBytes: 16,
  checkoutSha256: '1'.repeat(64), checkoutBytes: 16,
  checkoutMatchesGit: true,
}));
function receipt(bytes, os = 'linux', arch = 'x64', withBodies = false) {
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'a'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0', rustc: 'rustc 1.99.1',
      rustcHost: 'synthetic-host', rustflags: '', cargoEncodedRustflags: '',
      target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: inputs(),
    ...inspectWasmSections(bytes, { codeBodyDigests: withBodies }),
  };
}

test('fingerprints raw section bytes and identifies custom section name', () => {
  const bytes = moduleBytes(custom('producers', [1, 2]), code([0]));
  const result = inspectWasmSections(bytes);
  assert.equal(result.artifact.bytes, bytes.length);
  assert.equal(result.artifact.sha256, hash(bytes));
  assert.deepEqual(result.sections.map(s => [s.id, s.name, s.bytes]),
    [[0, 'producers', 12], [10, null, 1]]);
  assert.equal(result.sections[1].sha256, hash(Buffer.from([0])));
});

test('separates differing metadata-only payloads from unchanged executable sections', () => {
  const a = receipt(moduleBytes(custom('producers', [1]), code([0])));
  const b = receipt(moduleBytes(custom('producers', [2]), code([0])), 'win32');
  const result = compareMarkdownGuests(a, b);
  assert.equal(result.sameArtifactBytes, false);
  assert.equal(result.sameNonCustomSections, true);
  assert.equal(result.sameCustomSections, false);
  assert.deepEqual(result.changedSections.map(s => s.section), ['custom:producers']);
});

test('reports changes to executable section bytes even if custom metadata is identical', () => {
  const a = receipt(moduleBytes(custom('name'), code([0])));
  const b = receipt(moduleBytes(custom('name'), code([1])), 'darwin', 'arm64');
  const result = compareMarkdownGuests(a, b);
  assert.equal(result.sameNonCustomSections, false);
  assert.equal(result.sameCustomSections, true);
  assert.deepEqual(result.changedSections.map(s => s.section), ['section:10']);
});

test('identical repeated sections and payloads yield an identical summary', () => {
  const bytes = moduleBytes(custom('same'), custom('same', [9]), code([0]));
  const compared = compareMarkdownGuests(receipt(bytes), receipt(bytes, 'win32'));
  assert.equal(compared.sameArtifactBytes, true);
  assert.equal(compared.sameNonCustomSections, true);
  assert.equal(compared.sameCustomSections, true);
  assert.deepEqual(compared.changedSections, []);
});

test('rejects malformed Wasm sections and unbounded or truncated encodings', () => {
  for (const bytes of [
    Buffer.from([]),
    Buffer.from([1, 2, 3, 4, 1, 0, 0, 0]),
    Buffer.concat([magic, Buffer.from([10, 255])]),
    Buffer.concat([magic, Buffer.from([10, 4, 1])]),
    Buffer.concat([magic, Buffer.from([0, 1, 4]), code()]),
    Buffer.concat([magic, Buffer.from([14, 0]), code()]),
    Buffer.alloc(16 * 1024 * 1024 + 1),
  ]) assert.throws(() => inspectWasmSections(bytes));
});

test('refuses comparison across different revisions or canonical tracked source', () => {
  const bytes = moduleBytes(code());
  const first = receipt(bytes), second = receipt(bytes);
  second.revision = 'b'.repeat(40);
  assert.throws(() => compareMarkdownGuests(first, second), /shared tested Git revision/);
  second.revision = first.revision;
  second.inputs[1].gitSha256 = '2'.repeat(64);
  second.inputs[1].checkoutSha256 = '2'.repeat(64);
  assert.throws(() => compareMarkdownGuests(first, second), /shared canonical Git inputs/);
});

test('reports checkout normalization while requiring original Git source identity', () => {
  const bytes = moduleBytes(code());
  const first = receipt(bytes), second = receipt(bytes, 'win32');
  second.inputs[2].checkoutSha256 = '3'.repeat(64);
  second.inputs[2].checkoutMatchesGit = false;
  const compared = compareMarkdownGuests(first, second);
  assert.deepEqual(compared.sourceCheckoutDifferences.map(x => x.paths),
    [[], ['crates/grunkalunka-markdown/examples/outline_guest.rs']]);
});

const bodyCode = (...bodies) => code([
  bodies.length, ...bodies.flatMap(body => [body.length, ...body]),
]);

test('reports bounded full SHA-256 digests of each size-prefixed code body', () => {
  const first = [0, 11], second = [0, 1, 11];
  const report = inspectWasmSections(
    moduleBytes(bodyCode(first, second)), { codeBodyDigests: true });
  assert.deepEqual(report.codeBodies.lengths, [2, 3]);
  assert.equal(report.codeBodies.count, 2);
  const bytes = Buffer.from(report.codeBodies.bodyDigestsBase64, 'base64');
  assert.equal(bytes.length, 64);
  assert.equal(bytes.subarray(0, 32).toString('hex'), hash(Buffer.from(first)));
  assert.equal(bytes.subarray(32, 64).toString('hex'), hash(Buffer.from(second)));
});

test('counts changed defined-function bodies without claiming semantic equivalence', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [0, 11])), 'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([0, 11], [0, 1, 11])), 'win32', 'x64', true);
  const comparison = compareMarkdownGuests(a, b);
  assert.equal(comparison.codeBodyComparison.indicesComparable, true);
  assert.deepEqual(comparison.codeBodyComparison.firstChangedDefinedFunctionIndices, [1]);
  assert.equal(comparison.codeBodyComparison.changedCount, 1);
  assert.equal(comparison.codeBodyComparison.producerCount, 2);
  assert.equal(comparison.codeBodyComparison.receiverCount, 2);
  assert.equal(comparison.codeBodyComparison.changedBodyBytes, 3);
  assert.equal(comparison.sameNonCustomSections, false);
});

test('different function counts avoid misleading positional changed-body metrics', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11])), 'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([0, 11], [0, 11])),
    'darwin', 'arm64', true);
  const comparison = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(comparison.indicesComparable, false);
  assert.equal(comparison.changedCount, null);
  assert.deepEqual(comparison.firstChangedDefinedFunctionIndices, []);
});

test('refuses malformed code body lengths and fingerprints, or asymmetric receipts', () => {
  const badCode = [
    code([1, 5, 0, 11]),
    code([1, 2, 0]),
    code([1, 2, 0, 11, 7]),
    code([0]),
    code([255, 255, 255, 255, 127]),
  ];
  for (const section of badCode) {
    assert.throws(() => inspectWasmSections(
      moduleBytes(section), { codeBodyDigests: true }));
  }
  const a = receipt(moduleBytes(bodyCode([0, 11])), 'linux', 'x64', true);
  const b = structuredClone(a);
  b.codeBodies.bodyDigestsBase64 = 'not canonical';
  assert.throws(() => compareMarkdownGuests(a, b), /reported code body fingerprints/);
  b.codeBodies.bodyDigestsBase64 = Buffer.from([1]).toString('base64');
  assert.throws(() => compareMarkdownGuests(a, b), /canonical complete SHA-256/);
  delete b.codeBodies;
  assert.throws(() => compareMarkdownGuests(a, b), /both hosts must report/);
});

test('existing section-only receipts remain readable and comparable', () => {
  const bytes = moduleBytes(code([0]));
  const a = receipt(bytes), b = receipt(bytes, 'win32');
  assert.equal(compareMarkdownGuests(a, b).codeBodyComparison, null);
});

test('exact body multiset overlap finds identical bodies in a different order', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [1, 11], [2, 11])),
    'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([2, 11], [0, 11], [1, 11])),
    'win32', 'x64', true);
  const result = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(result.producerCount, 3);
  assert.equal(result.receiverCount, 3);
  assert.equal(result.changedCount, 3);
  assert.equal(result.matchingAtSameOrdinal, 0);
  assert.equal(result.matchingBodyOccurrencesIgnoringOrder, 3);
  assert.equal(result.additionalMatchesIgnoringOrdinal, 3);
  assert.equal(result.producerUnmatchedBodyOccurrences, 0);
  assert.equal(result.receiverUnmatchedBodyOccurrences, 0);
});

test('multiset comparison counts duplicate exact bodies with multiplicity', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [0, 11], [1, 11])),
    'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([0, 11], [1, 11], [1, 11])),
    'win32', 'x64', true);
  const result = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(result.changedCount, 1);
  assert.equal(result.matchingAtSameOrdinal, 2);
  assert.equal(result.matchingBodyOccurrencesIgnoringOrder, 2);
  assert.equal(result.additionalMatchesIgnoringOrdinal, 0);
  assert.equal(result.producerUnmatchedBodyOccurrences, 1);
  assert.equal(result.receiverUnmatchedBodyOccurrences, 1);
});

test('count differences retain exact multiset overlap without ordinal claims', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [1, 11], [2, 11])),
    'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([2, 11], [0, 11])),
    'darwin', 'arm64', true);
  const result = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(result.indicesComparable, false);
  assert.equal(result.changedCount, null);
  assert.equal(result.matchingAtSameOrdinal, null);
  assert.equal(result.additionalMatchesIgnoringOrdinal, null);
  assert.equal(result.matchingBodyOccurrencesIgnoringOrder, 2);
  assert.equal(result.producerUnmatchedBodyOccurrences, 1);
  assert.equal(result.receiverUnmatchedBodyOccurrences, 0);
});

test('same position and additional out-of-order matches are independently counted', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [1, 11], [2, 11])),
    'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([0, 11], [2, 11], [1, 11])),
    'darwin', 'arm64', true);
  const result = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(result.changedCount, 2);
  assert.equal(result.matchingAtSameOrdinal, 1);
  assert.equal(result.matchingBodyOccurrencesIgnoringOrder, 3);
  assert.equal(result.additionalMatchesIgnoringOrdinal, 2);
});

test('different bytes at every index and any position yield no exact matches', () => {
  const a = receipt(moduleBytes(bodyCode([0, 11], [1, 11])),
    'linux', 'x64', true);
  const b = receipt(moduleBytes(bodyCode([2, 11], [3, 11])),
    'win32', 'x64', true);
  const result = compareMarkdownGuests(a, b).codeBodyComparison;
  assert.equal(result.changedCount, 2);
  assert.equal(result.matchingAtSameOrdinal, 0);
  assert.equal(result.matchingBodyOccurrencesIgnoringOrder, 0);
  assert.equal(result.additionalMatchesIgnoringOrdinal, 0);
  assert.equal(result.producerUnmatchedBodyOccurrences, 2);
  assert.equal(result.receiverUnmatchedBodyOccurrences, 2);
});
