// Differential experiment: original JS vs emitted Wasm in Wasmtime and Node.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { corpusVersion, programs, rejected, unimplemented } from './corpus.mjs';
import { mkdir, mkdtemp, readFile, writeFile, access } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const directory = fileURLToPath(new URL('./', import.meta.url));
const root = fileURLToPath(new URL('../../', import.meta.url));
const binary = join(root, 'target', 'debug', `grunkalunka${process.platform === 'win32' ? '.exe' : ''}`);
const outputRoot = join(root, 'target', 'compatibility-runs');
await mkdir(outputRoot, { recursive: true });
const output = await mkdtemp(join(outputRoot, 'run-'));
let activeFixture = 'inventory';
process.on('uncaughtExceptionMonitor', () => {
  console.error(`FAIL ${activeFixture}; partial artifacts: ${output}`);
});
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

// Validate the whole selected inventory before executing any fixtures.
const ids = new Set();
function register(id, expectation) {
  assert.match(id, /^[a-z][a-z0-9-]*$/);
  const key = `${expectation}:${id}`;
  assert.ok(!ids.has(key), `duplicate fixture ID: ${key}`);
  ids.add(key);
  return key;
}
for (const [id, js, ts] of programs) {
  register(id, 'admitted');
  assert.equal(typeof js, 'string');
  assert.equal(typeof ts, 'string');
}
for (const [id, source, extension = 'js'] of rejected) {
  register(id, 'rejected');
  assert.equal(typeof source, 'string');
  assert.ok(['js', 'ts'].includes(extension));
}
for (const { id, milestone, reason } of unimplemented) {
  register(id, 'unimplemented');
  assert.match(milestone, /^[A-Z][0-9]+$/);
  assert.ok(typeof reason === 'string' && reason.length > 0);
}
assert.ok(Number.isSafeInteger(corpusVersion) && corpusVersion > 0);
const sha256 = data => createHash('sha256').update(data).digest('hex');
const lockfile = await readFile(join(root, 'Cargo.lock'));
const lockedWasmtime = lockfile.toString('utf8').match(/name = "wasmtime"\r?\nversion = "([^"]+)"/);
assert.ok(lockedWasmtime, 'locked Wasmtime version missing');
const runtime = {
  name: process.versions.bun ? 'bun' : 'node',
  version: process.versions.bun ?? process.versions.node,
  versions: process.versions, platform: process.platform, arch: process.arch,
};
const observations = [];
const fixtures = [];
let comparisons = 0;
for (const [name, bodyJS, bodyTS] of programs) {
  activeFixture = `admitted:${name}`;
  const sourceJS = name === 'affine'
    ? await readFile(new URL('fixtures/affine.js', import.meta.url), 'utf8')
    : `export function run(a, b) { ${bodyJS} }\n`;
  const sourceTS = name === 'affine'
    ? await readFile(new URL('fixtures/affine.ts', import.meta.url), 'utf8')
    : `export function run(a: number, b: number): number { ${bodyTS} }\n`;
  const reference = await import(`data:text/javascript;base64,${Buffer.from(sourceJS).toString('base64')}`);
  const modules = [];
  const variants = [];
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
      same(expected, native(a, b), `${details} ${runtime.name}-Wasm`);
      comparisons += 2;
    }
    variants.push({
      extension, source: `${name}.${extension}`, sourceSha256: sha256(source),
      wasm: `${name}-${extension}.wasm`, wasmSha256: sha256(wasm),
      wat: `${name}-${extension}.wat`, wasmBytes: wasm.length, vectors: vectors.length,
      engines: ['wasmtime', `${runtime.name}-wasm`], comparisons: vectors.length * 2,
      outcome: 'passed',
    });
    observations.push({ fixture: `${name}.${extension}`, wasmBytes: wasm.length, vectors: vectors.length });
    modules.push(wasm);
  }
  assert.deepEqual(modules[0], modules[1], `${name}: annotation-only variants emitted different bytes`);
  fixtures.push({ id: `admitted:${name}`, expectation: 'admitted', executed: true, outcome: 'passed', variants });
  console.log(`PASS ${name}: JS + TS, ${vectors.length} vectors, Wasmtime + ${runtime.name}-Wasm`);
}

// Every negative gets its own nonexistent artifact path. Never execute these in Node.
for (const [name, source, extension = 'js'] of rejected) {
  activeFixture = `rejected:${name}`;
  const sourcePath = join(output, `reject-${name}.${extension}`);
  const wasmPath = join(output, `reject-${name}.wasm`);
  await writeFile(sourcePath, source, { flag: 'wx' });
  const result = invoke(['compile', sourcePath, wasmPath], '', false);
  await assert.rejects(access(wasmPath), { code: 'ENOENT' });
  fixtures.push({
    id: `rejected:${name}`, expectation: 'rejected', executed: true, outcome: 'passed',
    extension, source: `reject-${name}.${extension}`, sourceSha256: sha256(source),
    diagnostic: result.stderr.trim(), exitStatus: result.status, artifactAbsent: true,
  });
}

// Existing output is preserved even for an otherwise valid compilation.
activeFixture = 'overwrite-sentinel';
const sentinel = join(output, 'sentinel.wasm');
await writeFile(sentinel, 'preserve me', { flag: 'wx' });
invoke(['compile', join(output, 'affine.js'), sentinel], '', false);
assert.equal(await readFile(sentinel, 'utf8'), 'preserve me');

activeFixture = 'affine-demo';
const affine = new WebAssembly.Instance(new WebAssembly.Module(await readFile(join(output, 'affine-ts.wasm'))));
assert.equal(affine.exports.run(4, 1), 3);
assert.equal(number(invoke(['eval', join(output, 'affine-ts.wasm')], `${bits(4)} ${bits(1)}\n`).stdout.trim()), 3);
activeFixture = 'report-inventory';
for (const gap of unimplemented) {
  fixtures.push({
    ...gap, id: `unimplemented:${gap.id}`, expectation: 'unimplemented', executed: false,
    outcome: 'not-executed', milestone: `docs/milestones.md`,
    milestoneId: gap.milestone,
  });
}
assert.equal(fixtures.length, ids.size, 'missing/extra fixture results');
assert.deepEqual(new Set(fixtures.map(fixture => fixture.id)), ids);
assert.equal(fixtures.filter(fixture => fixture.expectation === 'admitted').length, programs.length);
assert.ok(fixtures.filter(fixture => fixture.expectation === 'admitted').every(fixture =>
  fixture.variants.length === 2 && new Set(fixture.variants.map(variant => variant.extension)).size === 2));
assert.equal(observations.length, programs.length * 2);
assert.equal(comparisons, programs.length * 2 * vectors.length * 2);
const report = {
  schemaVersion: 1, corpusVersion, runtime,
  compiler: { binarySha256: sha256(await readFile(binary)), lockfileSha256: sha256(lockfile), lockedWasmtimeVersion: lockedWasmtime[1] },
  fixtures,
  counts: { admitted: programs.length, rejected: rejected.length, unimplemented: unimplemented.length },
  seed: seed.toString(16), vectorsPerModule: vectors.length,
  modules: observations, comparisons, rejected: rejected.length, overwritePreserved: true,
  demo: { input: [4, 1], output: 3 },
};
await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
console.log(`PASS ${comparisons} numeric comparisons; ${rejected.length} rejected inputs; overwrite preserved`);
console.log('DEMO: affine.ts -> Wasm; run(4, 1) = 3');
console.log(`Artifacts and report: ${output}`);
