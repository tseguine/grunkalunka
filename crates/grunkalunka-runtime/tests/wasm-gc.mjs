import assert from 'node:assert/strict';

assert.ok(WebAssembly.validate(bytes),
  `Node ${process.version} must support standard WasmGC (verified with Node 24).`);
const { exports } = new WebAssembly.Instance(new WebAssembly.Module(bytes));
function native(a, b) {
  const left = { value: a, next: null }, alias = left;
  const right = { value: b, next: null };
  left.next = left;
  const items = [left, right];
  items[0].value = b;
  assert.equal(left, alias);
  assert.notEqual(left, right);
  assert.equal(left.next, left);
  return alias.value;
}
const values = [0, -0, 1, -7, Number.MIN_VALUE, Infinity, -Infinity, NaN];
for (const a of values) for (const b of values) {
  assert.ok(Object.is(exports.run(a, b), native(a, b)));
  assert.ok(Object.is(exports.read(), b));
}
console.log(`PASS native JS / WasmGC: 64 input pairs, alias mutation and identity (${process.version})`);
