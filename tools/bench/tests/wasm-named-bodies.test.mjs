import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import {
  extractNamedBodyHashes, compareNamedBodyReceipts, stripTerminalRustHash, stripMangledRustHash,
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


test('strict Rust terminal hash normalization strips only the supported spelling', () => {
  assert.equal(stripTerminalRustHash('crate::item::h0123456789abcdef'), 'crate::item');
  assert.equal(stripTerminalRustHash('crate::item::hfedcba9876543210'), 'crate::item');
  for (const unchanged of [
    'crate::item',
    'crate::item::h0123456789abcdeg',
    'crate::item::h0123456789abcde',
    'crate::item::h0123456789abcdef::closure',
    'crate::item::H0123456789abcdef',
    '::h0123456789abcdef',
    '_RNvC0_4test4item',
  ]) assert.equal(stripTerminalRustHash(unchanged), unchanged);
  assert.throws(() => stripTerminalRustHash(null), /Invalid/);
});

test('matching names after removing Rust terminal hashes are counted separately', () => {
  const bodies = [[0, 11], [0, 1, 11]];
  const a = receipt(moduleBytes(bodies,
    [[0, 'crate::item::h0123456789abcdef'], [1, 'shared']]));
  const b = receipt(moduleBytes(bodies,
    [[0, 'crate::item::hfedcba9876543210'], [1, 'shared']]),
    'win32');
  const report = compareNamedBodyReceipts(a, b);
  assert.equal(report.sharedNameOccurrences, 1);
  assert.equal(report.sharedNameAndExactBodyOccurrences, 1);
  assert.deepEqual({
    method: report.terminalRustHashControl.method,
    producer: report.terminalRustHashControl.producerSuffixesStripped,
    receiver: report.terminalRustHashControl.receiverSuffixesStripped,
    shared: report.terminalRustHashControl.normalizedSharedNameOccurrences,
    sharedExact: report.terminalRustHashControl.normalizedSharedNameAndExactBodyOccurrences,
    additional: report.terminalRustHashControl.additionalNameMatchesAfterNormalization,
    additionalExact: report.terminalRustHashControl.additionalNameAndExactBodyMatchesAfterNormalization,
  }, {
    method: 'rust-legacy-terminal-hex-hash-v1',
    producer: 1, receiver: 1, shared: 2, sharedExact: 2,
    additional: 1, additionalExact: 1,
  });
});

test('equal normalized names with changed code do not count as matching bodies', () => {
  const a = receipt(moduleBytes([[0, 11]],
    [[0, 'crate::item::h0123456789abcdef']]));
  const b = receipt(moduleBytes([[0, 1, 11]],
    [[0, 'crate::item::hfedcba9876543210']]), 'darwin', 'arm64');
  const report = compareNamedBodyReceipts(a, b).terminalRustHashControl;
  assert.equal(report.normalizedSharedNameOccurrences, 1);
  assert.equal(report.normalizedSharedNameAndExactBodyOccurrences, 0);
  assert.equal(report.additionalNameMatchesAfterNormalization, 1);
  assert.equal(report.additionalNameAndExactBodyMatchesAfterNormalization, 0);
});

test('suffix normalization preserves multiplicity when names collapse', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 11], [0, 11]],
    [[0, 'item::h0123456789abcdef'], [1, 'item::hfedcba9876543210'],
      [2, 'other']]));
  const b = receipt(moduleBytes([[0, 11], [0, 11], [0, 11]],
    [[0, 'item::h1111111111111111'], [1, 'other'],
      [2, 'item::h2222222222222222']]), 'win32');
  const report = compareNamedBodyReceipts(a, b);
  assert.equal(report.sharedNameOccurrences, 1);
  assert.equal(report.terminalRustHashControl.normalizedSharedNameOccurrences, 3);
  assert.equal(report.terminalRustHashControl.normalizedSharedNameAndExactBodyOccurrences, 3);
  assert.equal(report.terminalRustHashControl.additionalNameMatchesAfterNormalization, 2);
});

test('old raw-only reports remain supported while one-sided normalization is rejected', () => {
  const bytes = moduleBytes([[0, 11]], [[0, 'legacy']]);
  const a = receipt(bytes), b = receipt(bytes, 'win32');
  delete a.namedBodies.normalization;
  delete b.namedBodies.normalization;
  assert.equal(compareNamedBodyReceipts(a, b).terminalRustHashControl, null);
  b.namedBodies.normalization = extractNamedBodyHashes(bytes, 1).normalization;
  assert.throws(() => compareNamedBodyReceipts(a, b),
    /both hosts must provide normalized name observations/);
});

test('rejects forged suffix counts, missing slot changes and incomplete normalized digests', () => {
  const bytes = moduleBytes([[0, 11], [0, 11]],
    [[0, 'item::h0123456789abcdef']]);
  const a = receipt(bytes), b = receipt(bytes, 'win32');
  b.namedBodies.normalization.matchingTerminalSuffixCount = 0;
  assert.throws(() => compareNamedBodyReceipts(a, b),
    /consistent number of normalized name hashes/);
  b.namedBodies.normalization.matchingTerminalSuffixCount = 1;
  b.namedBodies.normalization.normalizedNameDigestsBase64 = 'incorrect base64!';
  assert.throws(() => compareNamedBodyReceipts(a, b),
    /bounded terminal Rust hash normalization receipt/);
  b.namedBodies.normalization.normalizedNameDigestsBase64 =
    Buffer.from([1]).toString('base64');
  assert.throws(() => compareNamedBodyReceipts(a, b),
    /complete canonical normalized name hash vector/);
  const digest = Buffer.from(a.namedBodies.normalization.normalizedNameDigestsBase64, 'base64');
  digest.fill(2, 32, 64);
  b.namedBodies.normalization.normalizedNameDigestsBase64 = digest.toString('base64');
  assert.throws(() => compareNamedBodyReceipts(a, b),
    /unchanged presence of optional function names/);
});


test('only terminal legacy Rust mangled 17h + sixteen hex digits are stripped', () => {
  assert.equal(stripMangledRustHash('_ZN4core4item17h0123456789abcdefE'),
    '_ZN4core4itemE');
  assert.equal(stripMangledRustHash('__ZN4core4item17hfedcba9876543210E'),
    '__ZN4core4itemE');
  for (const unchanged of [
    '_ZN4core4itemE',
    '_ZN4core4item17h0123456789abcdeE',
    '_ZN4core4item17h0123456789abcdegE',
    '_ZN4core4item17H0123456789abcdefE',
    '_ZN4core4item17h0123456789abcdefE.llvm.123',
    'prefix_ZN4core4item17h0123456789abcdefE',
    'crate::item::h0123456789abcdef',
    '_RNvC0_4core4item',
    '_ZN17h0123456789abcdefE',
  ]) assert.equal(stripMangledRustHash(unchanged), unchanged);
  assert.throws(() => stripMangledRustHash(null), /Invalid/);
});

test('legacy mangled-name suffix control increases exact matching name and body occurrences', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 1, 11]],
    [[0, '_ZN4core4item17h0123456789abcdefE'], [1, 'shared']]));
  const b = receipt(moduleBytes([[0, 11], [0, 1, 11]],
    [[0, '_ZN4core4item17hfedcba9876543210E'], [1, 'shared']]), 'win32');
  const result = compareNamedBodyReceipts(a, b);
  assert.equal(result.sharedNameOccurrences, 1);
  assert.equal(result.sharedNameAndExactBodyOccurrences, 1);
  const normalized = result.mangledRustHashControl;
  assert.equal(normalized.method, 'rust-legacy-mangled-itanium-hash-v1');
  assert.equal(normalized.producerSuffixesStripped, 1);
  assert.equal(normalized.receiverSuffixesStripped, 1);
  assert.equal(normalized.normalizedSharedNameOccurrences, 2);
  assert.equal(normalized.normalizedSharedNameAndExactBodyOccurrences, 2);
  assert.equal(normalized.additionalNameMatchesAfterNormalization, 1);
  assert.equal(normalized.additionalNameAndExactBodyMatchesAfterNormalization, 1);
  assert.equal(result.terminalRustHashControl.additionalNameMatchesAfterNormalization, 0);
  assert.equal(normalized.producerSyntaxCounts.legacyMangled, 1);
  assert.equal(normalized.producerSyntaxCounts.other, 1);
});

test('normalized Rust mangled names do not turn differing code into identical bytes', () => {
  const a = receipt(moduleBytes([[0, 11]],
    [[0, '_ZN4core4item17h0123456789abcdefE']]));
  const b = receipt(moduleBytes([[0, 1, 11]],
    [[0, '_ZN4core4item17hfedcba9876543210E']]), 'darwin', 'arm64');
  const report = compareNamedBodyReceipts(a, b).mangledRustHashControl;
  assert.equal(report.normalizedSharedNameOccurrences, 1);
  assert.equal(report.normalizedSharedNameAndExactBodyOccurrences, 0);
});

test('legacy normalization preserves multiset multiplicity for collapsed names', () => {
  const a = receipt(moduleBytes([[0, 11], [0, 11]],
    [[0, '_ZN4core4item17h0123456789abcdefE'],
      [1, '_ZN4core4item17hfedcba9876543210E']]));
  const b = receipt(moduleBytes([[0, 11], [0, 11]],
    [[0, '_ZN4core4item17haaaaaaaaaaaaaaaaE'],
      [1, '_ZN4core4item17hbbbbbbbbbbbbbbbbE']]), 'win32');
  const control = compareNamedBodyReceipts(a, b).mangledRustHashControl;
  assert.equal(control.normalizedSharedNameOccurrences, 2);
  assert.equal(control.normalizedSharedNameAndExactBodyOccurrences, 2);
  assert.equal(control.additionalNameMatchesAfterNormalization, 2);
});

test('name syntax inventory categorizes forms without emitting symbol strings', () => {
  const names = [
    [0, '_ZN4core4item17h0123456789abcdefE'],
    [1, '_RNvC0_4core4item'],
    [2, 'crate::item'],
    [3, 'import_name'],
  ];
  const bytes = moduleBytes([[0, 11], [0, 11], [0, 11], [0, 11]], names);
  const info = extractNamedBodyHashes(bytes, 4);
  assert.deepEqual(info.mangledHashNormalization.nameSyntaxCounts, {
    legacyMangled: 1, v0MangledLike: 1, pathLike: 1, other: 1,
  });
  assert.equal(info.mangledHashNormalization.matchingTerminalSuffixCount, 1);
  assert.equal(info.mappedCount, 4);
});

test('missing new mangled control is backward compatible if absent on both sides', () => {
  const bytes = moduleBytes([[0, 11]], [[0, 'legacy']]);
  const first = receipt(bytes), second = receipt(bytes, 'win32');
  delete first.namedBodies.mangledHashNormalization;
  delete second.namedBodies.mangledHashNormalization;
  assert.equal(compareNamedBodyReceipts(first, second).mangledRustHashControl, null);
  second.namedBodies.mangledHashNormalization = extractNamedBodyHashes(bytes, 1).mangledHashNormalization;
  assert.throws(() => compareNamedBodyReceipts(first, second),
    /both hosts must provide mangled Rust name observations/);
});

test('rejects falsified mangled Rust hash counts and syntax histogram', () => {
  const bytes = moduleBytes([[0, 11]],
    [[0, '_ZN4core4item17h0123456789abcdefE']]);
  const first = receipt(bytes), other = receipt(bytes, 'win32');
  const entry = other.namedBodies.mangledHashNormalization;
  entry.matchingTerminalSuffixCount = 0;
  assert.throws(() => compareNamedBodyReceipts(first, other),
    /consistent number of normalized name hashes/);
  entry.matchingTerminalSuffixCount = 1;
  entry.nameSyntaxCounts.legacyMangled = 0;
  assert.throws(() => compareNamedBodyReceipts(first, other),
    /complete mangled Rust name syntax categories/);
  entry.nameSyntaxCounts.legacyMangled = 1;
  entry.normalizedNameDigestsBase64 = 'bogus!';
  assert.throws(() => compareNamedBodyReceipts(first, other),
    /bounded terminal Rust hash normalization receipt/);
  entry.normalizedNameDigestsBase64 = Buffer.from([1]).toString('base64');
  assert.throws(() => compareNamedBodyReceipts(first, other),
    /complete canonical normalized name hash vector/);
});
