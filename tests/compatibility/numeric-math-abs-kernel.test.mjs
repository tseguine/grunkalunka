import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, readFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { numericMathAbsRejected, promotedMathAbsReference } from './numeric-math-abs.mjs';

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
const out = mkdtempSync(join(target, 'math-abs-kernel-'));
const view = new DataView(new ArrayBuffer(8));
const bits = value => {
  view.setFloat64(0, value);
  return view.getBigUint64(0).toString(16).padStart(16, '0');
};
const number = hex => {
  view.setBigUint64(0, BigInt('0x' + hex));
  return view.getFloat64(0);
};

test('compiled Math.abs agrees with pristine Node and both engines on raw binary64 inputs', async () => {
  const source = join(out, 'kernel.js');
  const wasm = join(out, 'kernel.wasm');
  writeFileSync(source, promotedMathAbsReference.source);
  const compiled = run(cli, ['compile', source, wasm]);
  assert.equal(compiled.status, 0, compiled.stderr);
  const kernel = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(wasm))).exports.run;
  // Keep the promoted reference's original oracle arguments/result executable.
  const reference = await import('data:text/javascript;base64,' +
    Buffer.from(promotedMathAbsReference.oracleSource).toString('base64'));
  const { args, expected, entry } = promotedMathAbsReference;
  assert.equal(reference[entry](...args), expected);
  assert.equal(kernel(...args), expected);
  const original = run(cli, ['eval', wasm], { input: args.map(bits).join(' ') + '\n' });
  assert.equal(original.status, 0, original.stderr);
  assert.equal(number(original.stdout.trim()), expected);
  const inputs = [3, -3, 0, -0, NaN, Infinity, -Infinity, Number.MIN_VALUE,
    -Number.MIN_VALUE, Number.MAX_VALUE, -Number.MAX_VALUE, 1.5, -1.5];
  let state = 0x41425330n;
  const mask = (1n << 64n) - 1n;
  for (let i = 0; i < 4096; i++) {
    state ^= state << 13n; state &= mask;
    state ^= state >> 7n;
    state ^= state << 17n; state &= mask;
    inputs.push(number(state.toString(16).padStart(16, '0')));
  }
  const actual = [];
  // Keep each invocation within the existing 4096-pair CLI resource limit.
  for (let start = 0; start < inputs.length; start += 2048) {
    const batch = inputs.slice(start, start + 2048);
    const result = run(cli, ['eval', wasm], {
      input: batch.map(value => `${bits(value)} 0000000000000000`).join('\n') + '\n',
    });
    assert.equal(result.status, 0, result.stderr);
    actual.push(...result.stdout.trim().split(/\s+/).map(number));
  }
  assert.equal(actual.length, inputs.length);
  for (let i = 0; i < inputs.length; i++) {
    const expected = Math.abs(inputs[i]);
    for (const result of [kernel(inputs[i], 0), actual[i]]) {
      assert.ok(Number.isNaN(expected) ? Number.isNaN(result) : Object.is(expected, result),
        `${bits(inputs[i])}: ${bits(result)} != ${bits(expected)}`);
    }
  }
  writeFileSync(join(out, 'receipt.json'), JSON.stringify({
    inputs: inputs.length, comparisons: inputs.length * 2, seed: '41425330', source, wasm,
    nanComparison: 'class', nonNaNComparison: 'exact-binary64',
    promotedReference: { args, expected, engines: ['wasmtime', 'node-wasm'], oracle: entry },
  }, null, 2));
  console.log(`Math.abs kernel: ${inputs.length} inputs, ${inputs.length * 2} comparisons; ${out}`);
});

test('resolved Math shadows and attempted intrinsic patches remain rejected', () => {
  const guards = numericMathAbsRejected.filter(([id]) =>
    /shadow|tdz|patch|property-write|global-write|global-update/.test(id));
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
