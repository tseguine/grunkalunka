import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import { extractNamedBodyHashes } from '../wasm-named-bodies.mjs';
import { compareNativeMetadataControls } from '../markdown-native-metadata.mjs';

const SOURCE_PATHS = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const THREE_INPUTS = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const SHA = 'a'.repeat(64);
const source = path => ({
  path, gitSha256: SHA, gitBytes: 16,
  checkoutSha256: SHA, checkoutBytes: 16, checkoutMatchesGit: true,
});
const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const section = (id, bytes) => {
  assert.ok(bytes.length < 128);
  return Buffer.from([id, bytes.length, ...bytes]);
};
const wasm = (bodies, names) => {
  const nameEntries = names.flatMap((name, i) => {
    const text = Buffer.from(name);
    return [i, text.length, ...text];
  });
  const nameMap = [names.length, ...nameEntries];
  return Buffer.concat([
    magic,
    section(1, [1, 0x60, 0, 0]),
    section(3, [bodies.length, ...bodies.map(() => 0)]),
    section(10, [bodies.length, ...bodies.flatMap(body => [body.length, ...body])]),
    section(0, [4, ...Buffer.from('name'), 1, nameMap.length, ...nameMap]),
  ]);
};
function variant(name, os, arch, body, symbol) {
  const bytes = wasm([body], [symbol]);
  const sections = inspectWasmSections(bytes, { codeBodyDigests: true });
  const namedBodies = extractNamedBodyHashes(bytes, sections.codeBodies.count);
  delete namedBodies.normalization;
  delete namedBodies.mangledHashNormalization;
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'd'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0',
      rustc: 'rustc 1.99.1 (synthetic)', rustcHost:
        os === 'linux' ? 'x86_64-unknown-linux-gnu' :
          os === 'win32' ? 'x86_64-pc-windows-msvc' : 'aarch64-apple-darwin',
      rustflags: name === 'default' ? '' : '-C metadata=p1crosshostfixed',
      cargoEncodedRustflags: '', target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: THREE_INPUTS.map(source),
    ...sections, namedBodies,
  };
}
function receipt(os='linux', arch='x64', defaultBody=[0, 11],
  fixedBody=[0, 11], defaultName='alpha', fixedName='alpha') {
  const native = os === 'linux' ? 'x86_64-unknown-linux-gnu' :
    os === 'win32' ? 'x86_64-pc-windows-msvc' : 'aarch64-apple-darwin';
  return {
    schemaVersion: 1, workload: 'p1-native-markdown-metadata-control',
    revision: 'd'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0',
      rustc: 'rustc 1.99.1 (synthetic)', rustcHost: native,
      rustcCommit: 'e'.repeat(40), llvmVersion: '23.1.1',
      target: 'wasm32-unknown-unknown', profile: 'dev', cargoIncremental: '0',
      targetDirectoryPolicy:
        'same private absolute cargo target directory; clean wasm target between variants',
    },
    sourceInputs: SOURCE_PATHS.map(source),
    variants: {
      default: variant('default', os, arch, defaultBody, defaultName),
      fixed: variant('fixed', os, arch, fixedBody, fixedName),
    },
  };
}

test('identical default and fixed hosts are counted separately and remain equal', () => {
  const linux = receipt();
  const windows = receipt('win32', 'x64');
  const result = compareNativeMetadataControls(linux, windows);
  assert.equal(result.workload, 'p1-native-markdown-metadata-comparison');
  assert.equal(result.acrossHosts.default.sameArtifactBytes, true);
  assert.equal(result.acrossHosts.fixed.sameArtifactBytes, true);
  assert.equal(result.acrossHosts.fixed.sameRawNameOccurrences, 1);
  assert.equal(result.acrossHosts.fixed.sameRawNameAndBodyOccurrences, 1);
  assert.equal(result.acrossHosts.fixed.sameBodyOccurrencesIgnoringOrder, 1);
  assert.equal(result.withinHost.producer.sameArtifactBytes, true);
  assert.equal(result.withinHost.receiver.sameArtifactBytes, true);
  assert.equal(result.artifacts.producer.default.sha256,
    result.artifacts.receiver.default.sha256);
});

test('fixed metadata can improve body identity while default output differs', () => {
  const linux = receipt();
  const mac = receipt('darwin','arm64', [0, 1, 11], [0, 11], 'beta', 'alpha');
  const result = compareNativeMetadataControls(linux, mac);
  assert.equal(result.acrossHosts.default.sameArtifactBytes, false);
  assert.equal(result.acrossHosts.default.sameRawNameOccurrences, 0);
  assert.equal(result.acrossHosts.default.sameBodyOccurrencesIgnoringOrder, 0);
  assert.equal(result.acrossHosts.fixed.sameArtifactBytes, true);
  assert.equal(result.acrossHosts.fixed.sameRawNameOccurrences, 1);
  assert.equal(result.acrossHosts.fixed.sameBodyOccurrencesIgnoringOrder, 1);
  assert.equal(result.withinHost.receiver.sameArtifactBytes, false);
});

test('fixed metadata can change only code bytes or only diagnostic names', () => {
  const linux = receipt();
  const windows = receipt('win32', 'x64', [0, 11], [0, 1, 11],
    'alpha', 'alpha');
  const changedCode = compareNativeMetadataControls(linux, windows);
  assert.equal(changedCode.acrossHosts.default.sameArtifactBytes, true);
  assert.equal(changedCode.acrossHosts.fixed.sameRawNameOccurrences, 1);
  assert.equal(changedCode.acrossHosts.fixed.sameRawNameAndBodyOccurrences, 0);
  assert.ok(changedCode.acrossHosts.fixed.changedSections.includes('section:10'));
  windows.variants.fixed = variant('fixed', 'win32', 'x64', [0, 11], 'beta');
  const changedName = compareNativeMetadataControls(linux, windows);
  assert.deepEqual(changedName.acrossHosts.fixed.changedSections, ['custom:name']);
  assert.equal(changedName.acrossHosts.fixed.sameBodyOccurrencesIgnoringOrder, 1);
  assert.equal(changedName.acrossHosts.fixed.sameRawNameOccurrences, 0);
});

test('rejects forged requested metadata flags and missing report variants', () => {
  const linux = receipt(), windows = receipt('win32', 'x64');
  windows.variants.fixed.environment.rustflags = '-C metadata=notfixed';
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /native fixed source, flags and guest receipt/);
  windows.variants.fixed.environment.rustflags = '-C metadata=p1crosshostfixed';
  delete windows.variants.default;
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /both exact metadata variants/);
});

test('rejects different canonical source identities, Git revisions and Rust releases', () => {
  const linux = receipt(), windows = receipt('win32', 'x64');
  // Change a canonical manifest outside the inspector's narrower three-file
  // subset; both receipts remain individually valid, but hosts disagree.
  windows.sourceInputs[0].gitSha256 = 'b'.repeat(64);
  windows.sourceInputs[0].checkoutSha256 = 'b'.repeat(64);
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /identical canonical Git source inputs/);
  windows.sourceInputs[0] = source(SOURCE_PATHS[0]);
  windows.revision = 'f'.repeat(40);
  windows.variants.fixed.revision = windows.revision;
  windows.variants.default.revision = windows.revision;
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /identical tested Git revision/);
  windows.revision = linux.revision;
  windows.variants.fixed.revision = linux.revision;
  windows.variants.default.revision = linux.revision;
  windows.environment.rustcCommit = 'f'.repeat(40);
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /same observed Rust release/);
});

test('rejects corrupt body/name vectors and hidden normalization data', () => {
  const linux = receipt(), windows = receipt('win32', 'x64');
  windows.variants.fixed.codeBodies.bodyDigestsBase64 = 'bogus!';
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /reported code body fingerprints/);
  windows.variants.fixed = variant('fixed', 'win32', 'x64', [0, 11], 'alpha');
  windows.variants.fixed.namedBodies.nameDigestsBase64 = 'wrong!';
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /bounded named function vector/);
  windows.variants.fixed = variant('fixed', 'win32', 'x64', [0, 11], 'alpha');
  windows.variants.fixed.namedBodies.normalization = {};
  assert.throws(() => compareNativeMetadataControls(linux, windows),
    /native fixed source, flags and guest receipt/);
});

test('refuses incorrect native-host role, source checkout integrity and target', () => {
  const linux = receipt(), other = receipt('win32','x64');
  const wrongRoles = compareNativeMetadataControls(linux, other);
  assert.equal(wrongRoles.producer.os, 'linux');
  assert.throws(() => compareNativeMetadataControls(other, linux),
    /Linux x64 producer/);
  other.sourceInputs[3].checkoutMatchesGit = false;
  assert.throws(() => compareNativeMetadataControls(linux, other),
    /tracked Git source identity/);
  other.sourceInputs[3] = source(SOURCE_PATHS[3]);
  other.environment.target = 'wasm32-wasip1';
  assert.throws(() => compareNativeMetadataControls(linux, other),
    /native host build conditions/);
});
