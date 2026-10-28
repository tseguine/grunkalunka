import assert from 'node:assert/strict';
import test from 'node:test';
import { cases, verifyResults } from '../../tools/bench/verify-portable-affine.mjs';
import { run } from '../../examples/numeric-workflow/affine.js';

const bits = value => {
  const data = new DataView(new ArrayBuffer(8));
  data.setFloat64(0, value, false);
  return data.getBigUint64(0, false).toString(16).padStart(16, '0');
};
const value = hex => {
  const data = new DataView(new ArrayBuffer(8));
  data.setBigUint64(0, BigInt('0x' + hex), false);
  return data.getFloat64(0, false);
};
test('fixed bit-pattern goldens independently describe the checked-in affine source', () => {
  const results = cases.map(c => bits(run(value(c.a), value(c.b))));
  assert.equal(verifyResults(results, 'source JS'), 12);
});
test('rejects modified sign of zero and one-bit rounding errors', () => {
  const original = cases.map(c => c.result === 'nan' ? '7ff8000000000000' : c.result);
  assert.equal(verifyResults(original, 'synthetic'), 12);
  const negativeZero = [...original];
  const index = cases.findIndex(c => c.name === 'negative-zero');
  negativeZero[index] = '0000000000000000';
  assert.throws(() => verifyResults(negativeZero, 'synthetic'), /negative-zero/);
  const rounded = [...original];
  rounded[cases.findIndex(c => c.name === 'one-third')] = '3fd5555555555554';
  assert.throws(() => verifyResults(rounded, 'synthetic'), /one-third/);
});
test('NaN classification accepts different canonical payloads, not infinity', () => {
  const original = cases.map(c => c.result === 'nan' ? '7ff8000000000000' : c.result);
  const nanIndex = cases.findIndex(c => c.name === 'nan-operand');
  original[nanIndex] = 'fff8000000000001';
  assert.equal(verifyResults(original, 'synthetic'), 12);
  original[nanIndex] = '7ff0000000000000';
  assert.throws(() => verifyResults(original, 'synthetic'), /NaN/);
});
test('rejects missing, malformed or excessive native output', () => {
  const original = cases.map(c => c.result === 'nan' ? '7ff8000000000000' : c.result);
  assert.throws(() => verifyResults(original.slice(0, -1), 'synthetic'), /result count/);
  assert.throws(() => verifyResults([...original, '0000000000000000'], 'synthetic'), /result count/);
  original[0] = 'not-hex';
  assert.throws(() => verifyResults(original, 'synthetic'), /binary64/);
});
