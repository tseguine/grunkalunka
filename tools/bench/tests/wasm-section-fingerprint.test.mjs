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
function receipt(bytes, os = 'linux', arch = 'x64') {
  return {
    schemaVersion: 1, workload: 'p1-markdown-guest-wasm-sections',
    revision: 'a'.repeat(40),
    environment: {
      os, arch, node: 'v24.21.0', rustc: 'rustc 1.99.1',
      rustcHost: 'synthetic-host', rustflags: '', cargoEncodedRustflags: '',
      target: 'wasm32-unknown-unknown', profile: 'dev',
    },
    inputs: inputs(),
    ...inspectWasmSections(bytes),
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
