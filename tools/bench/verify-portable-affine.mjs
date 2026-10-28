// P1: replay one already compiled affine core-Wasm artifact in two engines.
// Fixed IEEE-754 goldens are literal data, not computed from source at test time.
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { readFile, stat } from 'node:fs/promises';
import { arch, platform } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const MAX_WASM = 64 * 1024;
const HEX = /^[a-f0-9]{16}$/;
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const requireValid = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const floatView = new DataView(new ArrayBuffer(8));
const fromHex = hex => {
  floatView.setBigUint64(0, BigInt('0x' + hex), false);
  return floatView.getFloat64(0, false);
};
const toHex = value => {
  floatView.setFloat64(0, value, false);
  return floatView.getBigUint64(0, false).toString(16).padStart(16, '0');
};

// These input/output bit patterns are precomputed independently. For NaN,
// verify the IEEE-754 classification rather than the engine-specific payload.
export const cases = Object.freeze([
  { name: 'integer', a: '4010000000000000', b: '3ff0000000000000', result: '4008000000000000' },
  { name: 'negative', a: 'c010000000000000', b: 'bff0000000000000', result: 'c008000000000000' },
  { name: 'one-third', a: '3ff0000000000000', b: 'bff0000000000000', result: '3fd5555555555555' },
  { name: 'negative-third', a: 'bff0000000000000', b: '3ff0000000000000', result: 'bfd5555555555555' },
  { name: 'positive-zero', a: '0000000000000000', b: '8000000000000000', result: '0000000000000000' },
  { name: 'negative-zero', a: '8000000000000000', b: '8000000000000000', result: '8000000000000000' },
  { name: 'subnormal', a: '0000000000000001', b: '0000000000000001', result: '0000000000000001' },
  { name: 'overflow', a: '7fe1ccf385ebc8a0', b: '0000000000000000', result: '7ff0000000000000' },
  { name: 'nan-operand', a: '7ff8000000000000', b: '4000000000000000', result: 'nan' },
  { name: 'infinite-cancellation', a: '7ff0000000000000', b: 'fff0000000000000', result: 'nan' },
  { name: 'infinite-result', a: '7ff0000000000000', b: '4008000000000000', result: '7ff0000000000000' },
  { name: 'fractional-rounding', a: '3fb999999999999a', b: '3fc999999999999a', result: '3fc1111111111111' },
]);

export function verifyResults(actual, engine) {
  requireValid(Array.isArray(actual) && actual.length === cases.length, engine + ' result count');
  for (let i = 0; i < cases.length; i++) {
    const value = actual[i];
    const expected = cases[i];
    requireValid(typeof value === 'string' && HEX.test(value), engine + ' binary64 ' + expected.name);
    if (expected.result === 'nan') {
      requireValid(Number.isNaN(fromHex(value)), engine + ' NaN ' + expected.name);
    } else {
      requireValid(value === expected.result, engine + ' golden ' + expected.name);
    }
  }
  return actual.length;
}

export async function verifyArtifact(wasmFile, nativeRunner) {
  requireValid(typeof wasmFile === 'string' && wasmFile.length > 0 &&
    typeof nativeRunner === 'string' && nativeRunner.length > 0, 'artifact and runner');
  const info = await stat(wasmFile);
  requireValid(info.isFile() && info.size >= 8 && info.size <= MAX_WASM, 'Wasm size');
  const wasm = await readFile(wasmFile);
  requireValid(wasm.length === info.size && wasm.length <= MAX_WASM, 'Wasm bytes');
  const wasmHash = sha256(wasm);
  const module = new WebAssembly.Module(wasm);
  requireValid(WebAssembly.Module.imports(module).length === 0, 'no guest imports');
  const instance = new WebAssembly.Instance(module);
  requireValid(typeof instance.exports.run === 'function', 'numeric run export');
  const nodeResults = cases.map(({ a, b }) => toHex(instance.exports.run(fromHex(a), fromHex(b))));
  const nodeCases = verifyResults(nodeResults, 'Node');

  // The only native subprocess operation is eval of the *same* Wasm file.
  // Never compile from source on the receiving host.
  const input = cases.map(c => c.a + ' ' + c.b).join('\n') + '\n';
  const child = spawnSync(resolve(nativeRunner), ['eval', resolve(wasmFile)], {
    input, cwd: root, encoding: 'utf8', timeout: 60_000, maxBuffer: 1024 * 1024,
    shell: false, windowsHide: true, killSignal: 'SIGKILL',
  });
  if (child.error || child.signal || child.status !== 0 || child.stderr) {
    throw new Error('Native Wasmtime eval failed: ' +
      (child.error?.message ?? child.stderr ?? child.signal ?? child.status));
  }
  const nativeResults = child.stdout.trim().split(/\r?\n/);
  const nativeCases = verifyResults(nativeResults, 'Wasmtime');
  requireValid(sha256(await readFile(wasmFile)) === wasmHash,
    'unchanged artifact after evaluation');

  return {
    schemaVersion: 1, workload: 'portable-core-affine',
    artifact: { bytes: wasm.length, sha256: wasmHash, imports: 0 },
    node: { engine: 'node-v8', goldenCases: nodeCases, node: process.version },
    wasmtime: { engine: 'native-wasmtime-cli', goldenCases: nativeCases },
    receiver: { os: platform(), arch: arch() },
    interpretation: 'one identical core-Wasm module; independent binary64 goldens; no timings',
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const args = process.argv.slice(2);
    requireValid(args.length === 3 && args[1] === '--runner',
      'usage: verify-portable-affine.mjs WASM --runner NATIVE_CLI');
    console.log(JSON.stringify(await verifyArtifact(args[0], args[2]), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
