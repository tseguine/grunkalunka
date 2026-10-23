// Differential experiment: original JS vs emitted Wasm in Wasmtime and Node.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, writeFile, access } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = fileURLToPath(new URL('./', import.meta.url));
const binary = join(directory, 'target', 'debug', `grunkalunka-numeric-spike${process.platform === 'win32' ? '.exe' : ''}`);
const outputRoot = join(directory, 'target', 'poc-runs');
await mkdir(outputRoot, { recursive: true });
const output = await mkdtemp(join(outputRoot, 'run-'));
const seed = 0x4752554en;
const started = performance.now();

function invoke(args, input = '', success = true) {
  if (performance.now() - started > 180_000) throw new Error('experiment exceeded three-minute harness limit');
  const result = spawnSync(binary, args, {
    cwd: directory, input, encoding: 'utf8', shell: false, timeout: 30_000, maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  if (success) assert.equal(result.status, 0, `${args.join(' ')}\n${result.stderr}`);
  else {
    assert.notEqual(result.status, 0, `unexpected admission: ${args.join(' ')}`);
    assert.match(result.stderr, /ERROR:/, 'rejection must be an explicit error, not a crash');
  }
  return result;
}

const bytes = new ArrayBuffer(8);
const view = new DataView(bytes);
function bits(value) {
  view.setFloat64(0, value, false);
  return view.getBigUint64(0, false).toString(16).padStart(16, '0');
}
function number(hex) {
  assert.match(hex, /^[0-9a-f]{16}$/);
  view.setBigUint64(0, BigInt(`0x${hex}`), false);
  return view.getFloat64(0, false);
}
function same(expected, actual, details) {
  assert.ok(Number.isNaN(expected) ? Number.isNaN(actual) : Object.is(expected, actual),
    `${details}: expected=${bits(expected)} actual=${bits(actual)}`);
}

const boundary = [0, -0, 1, -1, 2, -2, 1 / 3, Number.MIN_VALUE, -Number.MIN_VALUE,
  2 ** -1022, Number.MAX_VALUE, -Number.MAX_VALUE, Infinity, -Infinity, NaN,
  2 ** 53, -(2 ** 53), 1 + Number.EPSILON, 1 - Number.EPSILON];
const vectors = boundary.flatMap(a => boundary.map(b => [a, b]));
let state = seed;
function randomBits() {
  state = BigInt.asUintN(64, state * 6364136223846793005n + 1442695040888963407n);
  return number(state.toString(16).padStart(16, '0'));
}
for (let i = 0; i < 256; i++) vectors.push([randomBits(), randomBits()]);
const input = vectors.map(([a, b]) => `${bits(a)} ${bits(b)}`).join('\n') + '\n';
const expressions = [
  ['affine', '(a * 2 + b) / 3'], ['add', 'a + b'], ['subtract', 'a - b'],
  ['multiply', 'a * b'], ['divide', 'a / b'], ['negate', '-a'], ['unary-plus', '+a'],
  ['negative-zero', '-0'], ['subnormal-literal', '5e-324 + a'],
  ['overflow-literal', '1e999 + a'], ['rounded-literal', '9007199254740993 - a'],
  ['group-left', '(a + b) + 1'], ['group-right', 'a + (b + 1)'],
  ['separate-rounding', '(a * 1.0000000000000002) - a'],
  ['literal-bases', '0x10 + 0b10 + 0o7 + 1_000 + a - b'],
  ['nested-unary', '-(+(-(a / -(b + 1))))'],
];

const observations = [];
let comparisons = 0;
for (const [name, expression] of expressions) {
  const sourceJS = name === 'affine'
    ? await readFile(new URL('fixtures/affine.js', import.meta.url), 'utf8')
    : `export function run(a, b) { return ${expression}; }\n`;
  const sourceTS = name === 'affine'
    ? await readFile(new URL('fixtures/affine.ts', import.meta.url), 'utf8')
    : `export function run(a: number, b: number): number { return ${expression}; }\n`;
  const reference = await import(`data:text/javascript;base64,${Buffer.from(sourceJS).toString('base64')}`);
  const modules = [];
  for (const [extension, source] of [['js', sourceJS], ['ts', sourceTS]]) {
    const sourcePath = join(output, `${name}.${extension}`);
    const wasmPath = join(output, `${name}-${extension}.wasm`);
    await writeFile(sourcePath, source, { flag: 'wx' });
    invoke(['compile', sourcePath, wasmPath]);
    await writeFile(join(output, `${name}-${extension}.wat`), invoke(['wat', sourcePath]).stdout, { flag: 'wx' });
    const wasm = await readFile(wasmPath);
    assert.ok(WebAssembly.validate(wasm));
    const module = new WebAssembly.Module(wasm);
    assert.deepEqual(WebAssembly.Module.imports(module), []);
    assert.deepEqual(WebAssembly.Module.exports(module), [{ name: 'run', kind: 'function' }]);
    const native = new WebAssembly.Instance(module).exports.run;
    const wasmtime = invoke(['eval', wasmPath], input).stdout.trim().split(/\r?\n/);
    assert.equal(wasmtime.length, vectors.length, 'missing/extra Wasmtime results');
    for (let i = 0; i < vectors.length; i++) {
      const [a, b] = vectors[i];
      const expected = reference.run(a, b);
      const details = `${name}.${extension} seed=${seed.toString(16)} vector=${i} a=${bits(a)} b=${bits(b)}`;
      same(expected, number(wasmtime[i]), `${details} Wasmtime`);
      same(expected, native(a, b), `${details} Node-Wasm`);
      comparisons += 2;
    }
    observations.push({ fixture: `${name}.${extension}`, wasmBytes: wasm.length, vectors: vectors.length });
    modules.push(wasm);
  }
  assert.deepEqual(modules[0], modules[1], `${name}: annotation-only variants emitted different bytes`);
  console.log(`PASS ${name}: JS + TS, ${vectors.length} vectors, Wasmtime + Node-Wasm`);
}

// Every negative gets its own nonexistent artifact path. Never execute these in Node.
const rejected = [
  ['malformed', 'export function ('],
  ['duplicate-parameter', 'export function run(a, a) { return a; }'],
  ['extra-top-level', 'sideEffect(); export function run(a,b) { return a+b; }'],
  ['extra-export', 'export function run(a,b) { return a; } export const x = 2;'],
  ['default-export', 'export default function run(a,b) { return a; }'],
  ['named-export', 'function run(a,b) { return a; } export {run};'],
  ['wrong-name', 'export function other(a,b) { return a; }'],
  ['async', 'export async function run(a,b) { return a; }'],
  ['generator', 'export function* run(a,b) { return a; }'],
  ['default-param', 'export function run(a=sideEffect(),b) { return a; }'],
  ['rest-param', 'export function run(a,...b) { return a; }'],
  ['destructure', 'export function run({a},b) { return a; }'],
  ['arity', 'export function run(a) { return a; }'],
  ['global', 'export function run(a,b) { return clock; }'],
  ['function-reference', 'export function run(a,b) { return run; }'],
  ['arguments', 'export function run(a,b) { return arguments[0]; }'],
  ['call', 'export function run(a,b) { return Math.abs(a); }'],
  ['string', 'export function run(a,b) { return a + "1"; }'],
  ['bigint', 'export function run(a,b) { return 1n; }'],
  ['boolean', 'export function run(a,b) { return true; }'],
  ['null', 'export function run(a,b) { return null; }'],
  ['remainder', 'export function run(a,b) { return a % b; }'],
  ['power', 'export function run(a,b) { return a ** b; }'],
  ['bitwise', 'export function run(a,b) { return a | b; }'],
  ['logical', 'export function run(a,b) { return a || b; }'],
  ['comparison', 'export function run(a,b) { return a < b; }'],
  ['conditional', 'export function run(a,b) { return a ? a : b; }'],
  ['sequence', 'export function run(a,b) { return (a,b); }'],
  ['assignment', 'export function run(a,b) { return a=b; }'],
  ['update', 'export function run(a,b) { return a++; }'],
  ['object', 'export function run(a,b) { return {}; }'],
  ['array', 'export function run(a,b) { return []; }'],
  ['this', 'export function run(a,b) { return this; }'],
  ['unary-not', 'export function run(a,b) { return !a; }'],
  ['extra-body', 'export function run(a,b) { a++; return a; }'],
  ['empty-return', 'export function run(a,b) { return; }'],
  ['body-directive', 'export function run(a,b) { "use strict"; return a; }'],
  ['module-directive', '"use strict"; export function run(a,b) { return a; }'],
  ['hashbang', '#!/usr/bin/env node\nexport function run(a,b) { return a; }'],
  ['type-param', 'export function run<T>(a:number,b:number):number { return a; }', 'ts'],
  ['this-param', 'export function run(this:void,a:number,b:number):number { return a; }', 'ts'],
  ['optional-param', 'export function run(a?:number,b?:number):number { return a; }', 'ts'],
  ['string-annotation', 'export function run(a:string,b:number):number { return a; }', 'ts'],
  ['return-annotation', 'export function run(a:number,b:number):string { return a; }', 'ts'],
  ['type-assertion', 'export function run(a:number,b:number):number { return a as number; }', 'ts'],
  ['declare', 'export declare function run(a:number,b:number):number;', 'ts'],
  ['over-limit', 'export function run(a,b) { return a; }' + ' '.repeat(16384)],
  ['depth-limit', `export function run(a,b) { return ${'('.repeat(140)}a${')'.repeat(140)}; }`],
];
for (const [name, source, extension = 'js'] of rejected) {
  const sourcePath = join(output, `reject-${name}.${extension}`);
  const wasmPath = join(output, `reject-${name}.wasm`);
  await writeFile(sourcePath, source, { flag: 'wx' });
  invoke(['compile', sourcePath, wasmPath], '', false);
  await assert.rejects(access(wasmPath), { code: 'ENOENT' });
}

// Existing output is preserved even for an otherwise valid compilation.
const sentinel = join(output, 'sentinel.wasm');
await writeFile(sentinel, 'preserve me', { flag: 'wx' });
invoke(['compile', join(output, 'affine.js'), sentinel], '', false);
assert.equal(await readFile(sentinel, 'utf8'), 'preserve me');

const affine = new WebAssembly.Instance(new WebAssembly.Module(await readFile(join(output, 'affine-ts.wasm'))));
assert.equal(affine.exports.run(4, 1), 3);
assert.equal(number(invoke(['eval', join(output, 'affine-ts.wasm')], `${bits(4)} ${bits(1)}\n`).stdout.trim()), 3);
const report = {
  seed: seed.toString(16), node: process.version, vectorsPerModule: vectors.length,
  modules: observations, comparisons, rejected: rejected.length, overwritePreserved: true,
  demo: { input: [4, 1], output: 3 },
};
await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(`PASS ${comparisons} numeric comparisons; ${rejected.length} rejected inputs; overwrite preserved`);
console.log('DEMO: affine.ts -> Wasm; run(4, 1) = 3');
console.log(`Artifacts and report: ${output}`);
