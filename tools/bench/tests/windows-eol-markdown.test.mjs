import assert from 'node:assert/strict';
import test from 'node:test';
import { compareWindowsCheckoutModes } from '../windows-eol-markdown.mjs';

const tracked = [
  'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'crates/grunkalunka-markdown/Cargo.toml',
  'crates/grunkalunka-markdown/src/lib.rs',
  'crates/grunkalunka-markdown/examples/outline_guest.rs',
];
const hash = character => character.repeat(64);
function source(path, normalized) {
  return {
    path, gitSha256: hash('a'), gitBytes: 32,
    checkoutSha256: normalized ? hash('a') : hash('b'),
    checkoutBytes: normalized ? 32 : 35,
    checkoutMatchesGit: normalized,
  };
}
const sample = (phase, normalized, digest = hash('c')) => ({
  inputs: tracked.map(path => source(path, normalized)),
  artifact: { phase, bytes: 735367, sha256: digest },
});

test('identical Wasm after line normalization is explicitly observable', () => {
  const report = compareWindowsCheckoutModes(
    sample('CRLF', false), sample('LF', true));
  assert.equal(report.emittedWasmIdentical, true);
  assert.equal(report.crlfModifiedPaths.length, 6);
  assert.deepEqual(report.lfModifiedPaths, []);
  assert.equal(report.observation, 'same-wasm-with-both-checkout-modes');
});

test('byte difference after line normalization is an observation, not a failing assertion', () => {
  const result = compareWindowsCheckoutModes(
    sample('CRLF', false), sample('LF', true, hash('d')));
  assert.equal(result.emittedWasmIdentical, false);
  assert.equal(result.observation, 'different-wasm-after-checkout-normalization');
});

test('does not confuse a changed size with equal artifact identity', () => {
  const a = sample('CRLF', false), b = sample('LF', true);
  b.artifact.bytes--;
  assert.equal(compareWindowsCheckoutModes(a, b).emittedWasmIdentical, false);
});

test('refuses a test where Git text conversion was not exercised', () => {
  assert.throws(() => compareWindowsCheckoutModes(
    sample('CRLF', true), sample('LF', true)), /CRLF checkout actually altered/);
  assert.throws(() => compareWindowsCheckoutModes(
    sample('CRLF', false), sample('LF', false)), /LF checkout uses exact Git bytes/);
});

test('requires identical canonical Git sources and coherent checkouts', () => {
  const a = sample('CRLF', false), b = sample('LF', true);
  b.inputs[1].gitSha256 = hash('f');
  assert.throws(() => compareWindowsCheckoutModes(a, b), /tracked checkout record/);
  b.inputs[1].checkoutSha256 = hash('f');
  assert.throws(() => compareWindowsCheckoutModes(a, b), /unchanged canonical source/);
});

test('rejects invalid or missing artifact and input metadata', () => {
  const a = sample('CRLF', false), b = sample('LF', true);
  b.artifact.sha256 = 'bad';
  assert.throws(() => compareWindowsCheckoutModes(a, b), /Wasm fingerprint/);
  b.artifact.sha256 = hash('c');
  b.artifact.phase = 'different';
  assert.throws(() => compareWindowsCheckoutModes(a, b), /Wasm fingerprint/);
  b.artifact.phase = 'LF';
  b.inputs.pop();
  assert.throws(() => compareWindowsCheckoutModes(a, b), /complete source records/);
});
