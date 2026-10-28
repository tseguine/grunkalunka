import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import { extractNamedBodyHashes } from '../wasm-named-bodies.mjs';
import { summarizeMetadataControl } from '../markdown-crate-metadata-control.mjs';

const MODES = {
  default: '',
  alpha: '-C metadata=p1controlalpha',
  beta: '-C metadata=p1controlbeta',
};
const SOURCE_PATHS = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const section = (id, data) => {
  assert.ok(data.length < 128, 'small synthetic fixture');
  return Buffer.from([id, data.length, ...data]);
};
function makeModule(bodies, names) {
  const entry = names.flatMap(([index, value]) => {
    const bytes = Buffer.from(value, 'utf8');
    return [index, bytes.length, ...bytes];
  });
  const nameSubsection = [names.length, ...entry];
  return Buffer.concat([
    magic,
    section(1, [1, 0x60, 0, 0]),
    section(3, [bodies.length, ...bodies.map(() => 0)]),
    section(10, [bodies.length, ...bodies.flatMap(b => [b.length, ...b])]),
    section(0, [4, ...Buffer.from('name'), 1, nameSubsection.length, ...nameSubsection]),
  ]);
}
const source = path => ({
  path, gitSha256: 'a'.repeat(64), gitBytes: 64,
  checkoutSha256: 'a'.repeat(64), checkoutBytes: 64,
  checkoutMatchesGit: true,
});
function receipt(mode, names = ['first', 'second'], bodies = [[0, 11], [0, 11]]) {
  const wasm = makeModule(bodies, names.map((name, index) => [index, name]));
  const sections = inspectWasmSections(wasm, { codeBodyDigests: true });
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'd'.repeat(40),
    environment: {
      os: 'linux', arch: 'x64', node: 'v24.21.0',
      rustc: 'rustc 1.99.1 (synthetic)', rustcHost: 'x86_64-unknown-linux-gnu',
      rustflags: MODES[mode], cargoEncodedRustflags: '',
      target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: SOURCE_PATHS.map(source),
    ...sections,
    namedBodies: extractNamedBodyHashes(wasm, sections.codeBodies.count),
  };
}
function receipts() {
  return {
    default: receipt('default'),
    alpha: receipt('alpha'),
    beta: receipt('beta'),
  };
}

test('identical artifact bodies and diagnostic names remain identical across config variants', () => {
  const report = summarizeMetadataControl(receipts());
  assert.equal(report.workload, 'p1-markdown-crate-metadata-control');
  assert.equal(report.environment.rustflagsByVariant.default, '');
  for (const comparison of Object.values(report.comparisons)) {
    assert.equal(comparison.sameWasmBytes, true);
    assert.deepEqual(comparison.changedSections, []);
    assert.equal(comparison.rawNameSharedOccurrences, 2);
    assert.equal(comparison.rawNameAndExactBodySharedOccurrences, 2);
    assert.equal(comparison.matchingRawBodiesAtSameOrdinal, 2);
    assert.equal(comparison.matchingRawBodiesIgnoringOrder, 2);
    assert.deepEqual(comparison.rawBodiesUnmatched, [0, 0]);
  }
});

test('changed name metadata alone does not misreport a code-body change', () => {
  const variants = receipts();
  variants.alpha = receipt('alpha', ['new-first', 'second']);
  const report = summarizeMetadataControl(variants);
  assert.deepEqual(report.comparisons.defaultToAlpha.changedSections, ['custom:name']);
  assert.equal(report.comparisons.defaultToAlpha.sameWasmBytes, false);
  assert.equal(report.comparisons.defaultToAlpha.rawNameSharedOccurrences, 1);
  assert.equal(report.comparisons.defaultToAlpha.matchingRawBodiesIgnoringOrder, 2);
  assert.equal(report.comparisons.defaultToAlpha.rawNameAndExactBodySharedOccurrences, 1);
});

test('different body bytes are observed independently of changed Rust name strings', () => {
  const variants = receipts();
  variants.alpha = receipt('alpha', ['first', 'second'], [[0, 1, 11], [0, 11]]);
  variants.beta = receipt('beta', ['other', 'second'], [[0, 1, 11], [0, 11]]);
  const report = summarizeMetadataControl(variants);
  assert.equal(report.comparisons.defaultToAlpha.rawNameSharedOccurrences, 2);
  assert.equal(report.comparisons.defaultToAlpha.rawNameAndExactBodySharedOccurrences, 1);
  assert.equal(report.comparisons.defaultToAlpha.matchingRawBodiesIgnoringOrder, 1);
  assert.equal(report.comparisons.alphaToBeta.rawNameSharedOccurrences, 1);
  assert.equal(report.comparisons.alphaToBeta.matchingRawBodiesIgnoringOrder, 2);
});

test('rejects missing modes, forged rustc metadata flags and encoded hidden flags', () => {
  const variants = receipts();
  delete variants.beta;
  assert.throws(() => summarizeMetadataControl(variants),
    /complete default\/alpha\/beta metadata variants/);
  variants.beta = receipt('beta');
  variants.alpha.environment.rustflags = '';
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant alpha/);
  variants.alpha.environment.rustflags = MODES.alpha;
  variants.beta.environment.cargoEncodedRustflags = '-Cmetadata=hidden';
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant beta/);
});

test('rejects different Git revisions, toolchains and source snapshots', () => {
  const variants = receipts();
  variants.beta.revision = 'e'.repeat(40);
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant beta/);
  variants.beta.revision = variants.default.revision;
  variants.alpha.environment.rustc = 'rustc 1.99.2';
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant alpha/);
  variants.alpha.environment.rustc = variants.default.environment.rustc;
  variants.beta.inputs[1].gitSha256 = 'b'.repeat(64);
  variants.beta.inputs[1].checkoutSha256 = 'b'.repeat(64);
  assert.throws(() => summarizeMetadataControl(variants),
    /shared canonical Git inputs/);
});

test('rejects malformed body or name hash vectors instead of silently ignoring them', () => {
  const variants = receipts();
  variants.alpha.codeBodies.bodyDigestsBase64 = 'bogus!';
  assert.throws(() => summarizeMetadataControl(variants), /reported code body fingerprints/);
  variants.alpha = receipt('alpha');
  variants.beta.namedBodies.nameDigestsBase64 = 'not base64!';
  assert.throws(() => summarizeMetadataControl(variants), /bounded named function vector/);
});

test('rejects an unexpected native OS or unrelated compilation target', () => {
  const variants = receipts();
  variants.alpha.environment.os = 'win32';
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant alpha/);
  variants.alpha.environment.os = 'linux';
  variants.beta.environment.target = 'wasm32-wasip1';
  assert.throws(() => summarizeMetadataControl(variants),
    /controlled native Rust metadata variant beta/);
});
