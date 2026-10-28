import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { verifyTransferredIdentity } from '../../tools/bench/native-build-identity.mjs';

const source = Buffer.from('export function run(a, b) { return a + b; }\n');
const wasm = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0]);
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const expected = () => ({
  sourceSha256: sha(source), wasmSha256: sha(wasm), wasm64: wasm.toString('base64'),
});

test('accepts exactly the same canonical source and independently produced byte sequence', () => {
  assert.equal(verifyTransferredIdentity(source, wasm, expected()), true);
});

test('rejects a different compiler artifact even if the source matches', () => {
  const changed = Buffer.from(wasm);
  changed[7] ^= 1;
  assert.throws(() => verifyTransferredIdentity(source, changed, expected()), /must match original bytes/);
});

test('rejects a source mismatch and a wrong artifact hash', () => {
  assert.throws(() => verifyTransferredIdentity(Buffer.from(source.toString() + '\r\n'),
    wasm, expected()), /canonical source identity/);
  const changed = expected();
  changed.wasmSha256 = 'f'.repeat(64);
  assert.throws(() => verifyTransferredIdentity(source, wasm, changed), /transferred Wasm digest/);
});

test('rejects missing, noncanonical and unbounded transfer encoding', () => {
  for (const wasm64 of ['', 'not base64!', expected().wasm64 + '=', 'QQ==']) {
    assert.throws(() => verifyTransferredIdentity(source, wasm, { ...expected(), wasm64 }),
      undefined, wasm64);
  }
  assert.throws(() => verifyTransferredIdentity(source, Buffer.alloc(65537), expected()),
    /bounded compiled Wasm/);
});
