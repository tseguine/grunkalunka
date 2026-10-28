import assert from 'node:assert/strict';
import test from 'node:test';
import { inspectWasmSections } from '../wasm-section-fingerprint.mjs';
import { compareCguControls } from '../markdown-cgu-control.mjs';

const paths = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const sourceSubset = [
  'Cargo.lock',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const source = path => ({
  path, gitSha256: 'a'.repeat(64), gitBytes: 16,
  checkoutSha256: 'a'.repeat(64), checkoutBytes: 16,
  checkoutMatchesGit: true,
});
const magic = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
function wasm(body) {
  const sectionPayload = [1, body.length, ...body];
  return Buffer.concat([magic, Buffer.from([10, sectionPayload.length, ...sectionPayload])]);
}
function variant(bytes, os, arch, rustflags) {
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'd'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0', rustc: 'rustc 1.99.1 (test)',
      rustcHost: 'test-host', rustflags, cargoEncodedRustflags: '',
      target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: sourceSubset.map(source),
    ...inspectWasmSections(wasm(bytes), { codeBodyDigests: true }),
  };
}
function receipt(os = 'linux', arch = 'x64', defaultBody = [0, 11],
  singleBody = [0, 11]) {
  return {
    schemaVersion: 1, workload: 'p1-markdown-cgu-control',
    revision: 'd'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0', rustc: 'rustc 1.99.1 (test)',
      rustcHost: 'test-host', target: 'wasm32-unknown-unknown',
      profile: 'dev', cargoIncremental: '0',
    },
    buildCommand: ['cargo', 'build', '--locked', '--offline', '-j', '4',
      '-p', 'grunkalunka-markdown', '--example', 'outline_guest',
      '--target', 'wasm32-unknown-unknown'],
    targetDirectoryPolicy:
      'same private absolute target directory; Wasm target cleaned between modes',
    sourceInputs: paths.map(source),
    variants: {
      default: variant(defaultBody, os, arch, ''),
      single: variant(singleBody, os, arch, '-C codegen-units=1'),
    },
  };
}

test('compares native hosts independently under default and single codegen units', () => {
  const linux = receipt();
  const windows = receipt('win32', 'x64', [0, 11], [0, 1, 11]);
  const report = compareCguControls(linux, windows);
  assert.equal(report.schemaVersion, 1);
  assert.equal(report.workload, 'p1-markdown-cgu-cross-host-comparison');
  assert.equal(report.acrossHosts.default.sameArtifactBytes, true);
  assert.equal(report.acrossHosts.single.sameArtifactBytes, false);
  assert.equal(report.acrossHosts.single.codeBodyComparison.changedCount, 1);
  assert.equal(report.withinHost.producer.sameArtifactBytes, true);
  assert.equal(report.withinHost.receiver.sameArtifactBytes, false);
  assert.equal(report.artifactIdentity.producer.default.bytes,
    report.artifactIdentity.receiver.default.bytes);
  assert.ok(!JSON.stringify(report).includes('bodyDigestsBase64'),
    'comparison summary must not export per-function hashes');
});

test('different Rust versions or tested Git revisions cannot be compared', () => {
  const linux = receipt(), mac = receipt('darwin', 'arm64');
  mac.environment.rustc = 'rustc 1.99.2';
  mac.variants.default.environment.rustc = 'rustc 1.99.2';
  mac.variants.single.environment.rustc = 'rustc 1.99.2';
  assert.throws(() => compareCguControls(linux, mac), /same observed Rust release/);
  mac.environment.rustc = linux.environment.rustc;
  mac.variants.default.environment.rustc = linux.environment.rustc;
  mac.variants.single.environment.rustc = linux.environment.rustc;
  mac.revision = 'b'.repeat(40);
  mac.variants.default.revision = mac.revision;
  mac.variants.single.revision = mac.revision;
  assert.throws(() => compareCguControls(linux, mac), /same tested Git revision/);
});

test('rejects false or missing codegen-unit configuration metadata', () => {
  const first = receipt(), other = receipt('win32');
  other.variants.single.environment.rustflags = '';
  assert.throws(() => compareCguControls(first, other), /single exact native compiler configuration/);
  other.variants.single.environment.rustflags = '-C codegen-units=1';
  other.environment.cargoIncremental = '1';
  assert.throws(() => compareCguControls(first, other), /controlled build metadata/);
  other.environment.cargoIncremental = '0';
  delete other.variants.single;
  assert.throws(() => compareCguControls(first, other), /two required variants/);
});

test('rejects altered tracked source, source metadata and code fingerprints', () => {
  const first = receipt(), other = receipt('win32');
  other.sourceInputs[0].gitSha256 = 'f'.repeat(64);
  other.sourceInputs[0].checkoutSha256 = 'f'.repeat(64);
  assert.throws(() => compareCguControls(first, other), /same canonical Git source files/);
  other.sourceInputs[0] = source(paths[0]);
  other.variants.single.inputs[0].gitSha256 = 'f'.repeat(64);
  other.variants.single.inputs[0].checkoutSha256 = 'f'.repeat(64);
  assert.throws(() => compareCguControls(first, other), /single observed canonical source identity/);
  other.variants.single.inputs[0] = source(sourceSubset[0]);
  other.variants.default.codeBodies.bodyDigestsBase64 = 'not canonical!';
  assert.throws(() => compareCguControls(first, other), /reported code body fingerprints/);
});

test('allows real cross-host byte differences as observations, not a similarity gate', () => {
  const linux = receipt();
  const mac = receipt('darwin', 'arm64', [0, 1, 11], [0, 2, 11]);
  const report = compareCguControls(linux, mac);
  assert.equal(report.acrossHosts.default.sameArtifactBytes, false);
  assert.equal(report.acrossHosts.single.sameArtifactBytes, false);
  assert.equal(report.acrossHosts.default.codeBodyComparison.changedCount, 1);
  assert.equal(report.acrossHosts.single.codeBodyComparison.changedCount, 1);
});
