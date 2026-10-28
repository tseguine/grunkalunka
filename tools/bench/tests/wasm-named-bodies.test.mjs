import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import {
  extractNamedBodyHashes, compareNamedBodyReceipts,
} from '../wasm-named-bodies.mjs';

const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const encode = (id, payload) => {
  assert.ok(payload.length < 128, 'short test section');
  return Buffer.from([id, payload.length, ...payload]);
};
const custom = names => {
  const pairs = names.flatMap(([index, label]) => {
    const text = Buffer.from(label, 'utf8');
    return [index, text.length, ...text];
  });
  const subsection = [names.length, ...pairs];
  return encode(0, [4, ...Buffer.from('name'), 1, subsection.length, ...subsection]);
};
const moduleBytes = (bodies, names) => {
  const types = encode(1, [1, 0x60, 0, 0]);
  const functions = encode(3, [bodies.length, ...bodies.map(() => 0)]);
  const code = encode(10, [bodies.length,
    ...bodies.flatMap(body => [body.length, ...body])]);
  return Buffer.concat([magic, types, functions, code, custom(names)]);
};
const inputs = () => [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
].map(path => ({
  path, gitSha256: 'a'.repeat(64), gitBytes: 16,
  checkoutSha256: 'a'.repeat(64), checkoutBytes: 16,
  checkoutMatchesGit: true,
}));
function receipt(bytes, os = 'linux', arch = 'x64') {
  const body = inspectWasmSections(bytes, { codeBodyDigests: true });
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'b'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0', rustc: 'rustc 1.99.1',
      rustcHost: 'test-host', rustflags: '', cargoEncodedRustflags: '',
      target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: inputs(), ...body,
    namedBodies: extractNamedBodyHashes(bytes, body.codeBodies.count),
  };
}

test('extracts UTF-8 function name hashes by absolute function index', () => {
  const bytes = moduleBytes([[0, 11], [0, 1, 11]], [[0, 'alpha'], [1, 'βeta']]);
  assert.equal(WebAssembly.validate(bytes), true);
  const names = extractNamedBodyHashes(bytes, 2);
  assert.equal(names.count, 2);
  assert.equal(names.mappedCount, 2);
  assert.equal(names.missingCount, 0);
  const digests = Buffer.from(names.nameDigestsBase64, 'base64');
  assert.equal(digests.subarray(0, 32).toString('hex'),
    sha(Buffer.from('alpha')));
  assert.equal(digests.subarray(32, 64).toString('hex'),
    sha(Buffer.from('βeta')));
});

test('correlates unchanged source-named function bodies despite reordering', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 1, 11]],
    [[0, 'alpha'], [1, 'beta']]));
  const b = receipt(moduleBytes([[0, 1, 11], [0, 11]],
    [[0, 'beta'], [1, 'alpha']]), 'darwin', 'arm64');
  const result = compareNamedBodyReceipts(a, b);
  assert.equal(result.sharedNameOccurrences, 2);
  assert.equal(result.sharedNameAndExactBodyOccurrences, 2);
  assert.equal(result.sharedNameOccurrencesWithoutExactBodyMatch, 0);
  assert.equal(result.namesMatchingAtSameOrdinal, 0);
  assert.equal(result.rawCodeBodyComparison.changedCount, 2);
});

test('same name with altered raw code is reported without semantic inference', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 1, 11]],
    [[0, 'alpha'], [1, 'beta']]));
  const b = receipt(moduleBytes([[0, 11], [0, 1, 1, 11]],
    [[0, 'alpha'], [1, 'beta']]), 'win32');
  const result = compareNamedBodyReceipts(a, b);
  assert.equal(result.sharedNameOccurrences, 2);
  assert.equal(result.sharedNameAndExactBodyOccurrences, 1);
  assert.equal(result.sharedNameOccurrencesWithoutExactBodyMatch, 1);
  assert.equal(result.namesMatchingAtSameOrdinal, 2);
});

test('duplicate names are counted with multiplicity, not Set uniqueness', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 11], [0, 1, 11]],
    [[0, 'dup'], [1, 'dup'], [2, 'other']]));
  const b = receipt(moduleBytes([[0, 11], [0, 1, 11], [0, 1, 11]],
    [[0, 'dup'], [1, 'other'], [2, 'other']]), 'win32');
  const result = compareNamedBodyReceipts(a, b);
  assert.equal(result.sharedNameOccurrences, 2);
  assert.equal(result.sharedNameAndExactBodyOccurrences, 2);
  assert.equal(result.producerNamedUnmatchedOccurrences, 1);
  assert.equal(result.receiverNamedUnmatchedOccurrences, 1);
});

test('incomplete function name metadata is explicit rather than fabricated', () => {
  const bytes = moduleBytes([[0, 11], [0, 11], [0, 11]],
    [[0, 'alpha'], [2, 'gamma']]);
  const names = extractNamedBodyHashes(bytes, 3);
  assert.equal(names.mappedCount, 2);
  assert.equal(names.missingCount, 1);
  const digests = Buffer.from(names.nameDigestsBase64, 'base64');
  assert.ok(digests.subarray(32, 64).every(x => x === 0));
  const r = receipt(bytes);
  const result = compareNamedBodyReceipts(r, r);
  assert.equal(result.sharedNameOccurrences, 2);
});

test('refuses duplicate or out-of-range indices, name-map omissions and imports', () => {
  const bodies = [[0, 11], [0, 11]];
  for (const names of [
    [[0, 'first'], [0, 'again']],
    [[0, 'first'], [2, 'too high']],
    [],
  ]) {
    assert.throws(() => extractNamedBodyHashes(moduleBytes(bodies, names), 2));
  }
  const imported = Buffer.concat([
    magic, encode(1, [1, 0x60, 0, 0]), encode(2, [0]),
    encode(3, [1, 0]), encode(10, [1, 2, 0, 11]), custom([[0, 'name']]),
  ]);
  assert.throws(() => extractNamedBodyHashes(imported, 1), /imported functions/);
});

test('rejects truncated name sections and corrupt transferred hashes', () => {
  const good = moduleBytes([[0, 11]], [[0, 'alpha']]);
  assert.throws(() => extractNamedBodyHashes(good.subarray(0, -1), 1),
    /bounded Wasm section payload/);
  const a = receipt(good), b = structuredClone(a);
  b.namedBodies.nameDigestsBase64 = 'not valid base64!';
  assert.throws(() => compareNamedBodyReceipts(a, b), /bounded named function vector/);
  b.namedBodies.nameDigestsBase64 = Buffer.from([1]).toString('base64');
  assert.throws(() => compareNamedBodyReceipts(a, b), /complete canonical/);
  b.namedBodies.nameDigestsBase64 = a.namedBodies.nameDigestsBase64;
  b.namedBodies.mappedCount = 0;
  assert.throws(() => compareNamedBodyReceipts(a, b), /bounded named function vector/);
});
