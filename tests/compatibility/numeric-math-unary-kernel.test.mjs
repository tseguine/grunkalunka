import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, readFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mathUnaryMethods, numericMathUnaryRejected, promotedMathUnaryReference } from './numeric-math-unary.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const cli = join(target, 'debug', process.platform === 'win32' ? 'grunkalunka.exe' : 'grunkalunka');
function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root, encoding: 'utf8', shell: false, timeout: 120_000,
    maxBuffer: 8 * 1024 * 1024, ...options,
  });
  assert.equal(result.error, undefined);
  return result;
}
const build = run('cargo', ['build', '-p', 'grunkalunka-cli', '--locked', '--offline', '-j', '2']);
assert.equal(build.status, 0, build.stderr);
mkdirSync(target, { recursive: true });
const out = mkdtempSync(join(target, 'math-unary-kernel-'));
const view = new DataView(new ArrayBuffer(8));
const bits = value => {
  view.setFloat64(0, value);
  return view.getBigUint64(0).toString(16).padStart(16, '0');
};
const number = hex => {
  view.setBigUint64(0, BigInt('0x' + hex));
  return view.getFloat64(0);
};
const named = [4, -1.5, 1.5, -0.5, 0.5, 0, -0, NaN, Infinity, -Infinity,
  Number.MIN_VALUE, -Number.MIN_VALUE, 2 ** -1022, -(2 ** -1022), Number.MAX_VALUE,
  -Number.MAX_VALUE, 2 ** 52 - 0.5, 2 ** 52, 2 ** 52 + 1, -(2 ** 52 - 0.5),
  -(2 ** 52 + 1), 1 - Number.EPSILON / 2, 1 + Number.EPSILON];
// Adjacent values around representative squares and exponent boundaries.
for (const x of [0, Number.MIN_VALUE, 2 ** -1022, 1, 4, 9, 16, 2 ** 52, 2 ** 104, Number.MAX_VALUE]) {
  view.setFloat64(0, x);
  const raw = view.getBigUint64(0);
  if (raw > 0n) named.push(number((raw - 1n).toString(16).padStart(16, '0')));
  named.push(x);
  if (raw < 0x7fefffffffffffffn) named.push(number((raw + 1n).toString(16).padStart(16, '0')));
}
const inputs = [...named];
let state = 0x554e4152n;
const mask = (1n << 64n) - 1n;
for (let i = 0; i < 4096; i++) {
  state ^= state << 13n; state &= mask;
  state ^= state >> 7n;
  state ^= state << 17n; state &= mask;
  inputs.push(number(state.toString(16).padStart(16, '0')));
}
for (const method of mathUnaryMethods) {
  test(`compiled Math.${method} matches pristine Node in both engines`, async () => {
    const source = join(out, method + '.js');
    const wasm = join(out, method + '.wasm');
    const text = `export function run(a,b) { return Math.${method}(a); }`;
    writeFileSync(source, text);
    const compiled = run(cli, ['compile', source, wasm]);
    assert.equal(compiled.status, 0, compiled.stderr);
    const native = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(wasm))).exports.run;
    const oracle = (await import('data:text/javascript;base64,' + Buffer.from(text).toString('base64'))).run;
    const actual = [];
    for (let start = 0; start < inputs.length; start += 2048) {
      const batch = inputs.slice(start, start + 2048);
      const result = run(cli, ['eval', wasm], {
        input: batch.map(x => `${bits(x)} 0000000000000000`).join('\n') + '\n',
      });
      assert.equal(result.status, 0, result.stderr);
      actual.push(...result.stdout.trim().split(/\s+/).map(number));
    }
    assert.equal(actual.length, inputs.length);
    for (let i = 0; i < inputs.length; i++) {
      const expected = oracle(inputs[i], 0);
      for (const result of [native(inputs[i], 0), actual[i]]) {
        assert.ok(Number.isNaN(expected) ? Number.isNaN(result) : Object.is(expected, result),
          `${method}(${bits(inputs[i])}): ${bits(result)} != ${bits(expected)}`);
      }
    }
    if (method === 'floor') assert.equal(text, promotedMathUnaryReference.source);
    writeFileSync(join(out, method + '-receipt.json'), JSON.stringify({
      method, inputs: inputs.length, namedInputs: named.length, comparisons: inputs.length * 2,
      seed: '554e4152', source, wasm, nonNaN: 'exact-binary64', NaN: 'class',
    }, null, 2));
    console.log(`Math.${method}: ${inputs.length} inputs, ${inputs.length * 2} comparisons; ${out}`);
  });
}
test('unary Math family preserves semantic shadows and rejects source patches', () => {
  const guards = numericMathUnaryRejected.filter(([id]) => /shadow|tdz|patch|global-write/.test(id));
  for (const [id, source, extension, diagnostic] of guards) {
    const path = join(out, id + '.' + extension);
    const wasm = join(out, id + '.wasm');
    writeFileSync(path, source);
    const result = run(cli, ['compile', path, wasm]);
    assert.equal(result.status, 1, id);
    assert.ok(result.stderr.includes(diagnostic), `${id}: ${result.stderr}`);
    assert.equal(existsSync(wasm), false, id);
  }
});
