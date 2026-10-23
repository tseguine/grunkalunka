import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const runner = fileURLToPath(new URL('./gap-oracle.mjs', import.meta.url));
function observe(source, args = [], entry = 'run', timeout = 5000) {
  return spawnSync(process.execPath, [runner, Buffer.from(source).toString('base64'), JSON.stringify(args), entry],
    { encoding: 'utf8', timeout, maxBuffer: 64 * 1024, shell: false });
}

test('a zero-valued reference emits its binary64 result', () => {
  const result = observe('export function run() { return 0; }');
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '0000000000000000');
});

test('a reference receives numeric arguments and returns their computed result', () => {
  const result = observe('export function run(a, b) { return a + b; }', [4, 2]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '4018000000000000');
});

test('a string result must not be silently coerced to a number', () => {
  const result = observe('export function run() { return "4"; }');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /reference result must be a primitive number/);
});

test('an async reference is observed after its result resolves', () => {
  const result = observe('export async function run(a, b) { return a + await Promise.resolve(b); }', [4, 2]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '4018000000000000');
});

test('a reference can specify the exported function to observe', () => {
  const result = observe('export function sum(a, b) { return a + b; }', [4, 2], 'sum');
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, '4018000000000000');
});

test('signed zero, infinities and NaN survive the transport', () => {
  for (const [expression, expected] of [['-0', '8000000000000000'],
    ['Infinity', '7ff0000000000000'], ['-Infinity', 'fff0000000000000']]) {
    const result = observe(`export function run() { return ${expression}; }`);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout, expected);
  }
  const result = observe('export function run() { return NaN; }');
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^[a-f0-9]{16}$/);
  const view = new DataView(new ArrayBuffer(8));
  view.setBigUint64(0, BigInt(`0x${result.stdout}`), false);
  assert.ok(Number.isNaN(view.getFloat64(0, false)));
});

test('throwing and nonterminating references cannot count as successful observations', () => {
  const thrown = observe('export function run() { throw new Error("reference sentinel"); }');
  assert.notEqual(thrown.status, 0);
  assert.match(thrown.stderr, /reference sentinel/);
  const timedOut = observe('export function run() { while (true) {} }', [], 'run', 1000);
  assert.equal(timedOut.error?.code, 'ETIMEDOUT');
});
