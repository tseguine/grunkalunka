import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';
import { compareReports } from '../../tools/bench/compare.mjs';
import { statistics } from '../../tools/bench/measure.mjs';

function report() {
  return {
    schemaVersion: 1, workload: 'numeric-gcd', observedAt: '2026-10-27T00:00:00.000Z',
    provenance: { sourceSha256: '1'.repeat(64), wasmSha256: '2'.repeat(64), harnessSha256: '3'.repeat(64),
      wasmBytes: 100, compiler: { path: '/compiler', sha256: '4'.repeat(64), version: '0.0.1', buildLabel: 'debug' },
      node: 'v24.0.0', v8: 'test', platform: 'test', arch: 'x64', osRelease: 'test', cpuModel: 'test CPU',
      nodeOptions: '', workerExecArgv: [] },
    config: { samples: 3, iterations: 16, warmup: 8 }, cachePolicy: 'uncontrolled',
    cases: [[1071, 462, 21], [84, 30, 6], [-84, 30, 6], [0, 0, 0],
      [0, -9, 9], [13, 17, 1], [81, 27, 27], [-55, -34, 1]], checksum: 142,
    buildApiMs: 10,
    startup: { moduleConstructionMs: 2, instantiationMs: 1, firstJsCallMs: 0.5, firstWasmCallMs: 0.5 },
    batches: { unit: 'milliseconds per batch', order: 'alternating, JS first in sample 0',
      js: statistics([6, 2, 4]), wasm: statistics([9, 3, 6]) },
  };
}
function change(object, path, value) {
  const parts = path.split('.');
  const key = parts.pop();
  let parent = object;
  for (const part of parts) parent = parent[part];
  if (value === undefined) delete parent[key];
  else parent[key] = value;
}

test('comparison retains samples and identities without mutating either input', () => {
  const before = report();
  const after = report();
  after.provenance.compiler.sha256 = 'a'.repeat(64);
  after.provenance.compiler.buildLabel = 'release';
  after.provenance.compiler.path = '/different compiler';
  after.provenance.wasmSha256 = 'b'.repeat(64);
  after.provenance.wasmBytes = 80;
  after.buildApiMs = 8;
  after.batches.wasm = statistics([3, 1, 2]);
  const originals = structuredClone([before, after]);
  const result = compareReports(before, after);
  assert.deepEqual(result.buildApiMs, { baseline: 10, candidate: 8, delta: -2, ratio: 0.8 });
  assert.deepEqual(result.wasmBytes, { baseline: 100, candidate: 80, delta: -20, ratio: 0.8 });
  assert.deepEqual(result.batches.wasm.medianMs, { baseline: 6, candidate: 2, delta: -4, ratio: 1 / 3 });
  assert.deepEqual(result.batches.wasm.baseline.samplesMs, [9, 3, 6]);
  assert.equal(result.candidate.provenance.compiler.sha256, 'a'.repeat(64));
  assert.deepEqual([before, after], originals);
  result.baseline.provenance.compiler.path = 'changed';
  result.batches.wasm.baseline.samplesMs[0] = 0;
  assert.deepEqual([before, after], originals);
});

test('workload, harness and environment differences cannot masquerade as compiler changes', () => {
  for (const [path, value] of [
    ['config.warmup', 9], ['cachePolicy', 'cold'], ['provenance.sourceSha256', 'a'.repeat(64)],
    ['provenance.harnessSha256', 'b'.repeat(64)], ['provenance.node', 'different'],
    ['provenance.v8', 'different'], ['provenance.platform', 'different'], ['provenance.arch', 'arm64'],
    ['provenance.osRelease', 'different'], ['provenance.cpuModel', null],
    ['provenance.nodeOptions', '--jitless'], ['provenance.workerExecArgv', ['--jitless']],
  ]) {
    const candidate = report();
    change(candidate, path, value);
    assert.throws(() => compareReports(report(), candidate), /Incompatible reports:/, path);
  }
});

test('missing metadata on both sides is not evidence of matching conditions', () => {
  for (const path of ['config.samples', 'provenance.cpuModel', 'provenance.nodeOptions',
    'provenance.workerExecArgv', 'provenance.harnessSha256', 'observedAt', 'startup.firstJsCallMs']) {
    const before = report();
    change(before, path, undefined);
    assert.throws(() => compareReports(before, structuredClone(before)), /Missing/, path);
  }
});

test('invalid samples, inconsistent summaries and malformed report fields reject', () => {
  for (const [path, value] of [['schemaVersion', 2], ['workload', 'other'], ['config.samples', 1],
    ['checksum', 0], ['cases', []], ['buildApiMs', NaN], ['buildApiMs', -1],
    ['startup.firstWasmCallMs', Infinity], ['provenance.wasmBytes', 0],
    ['provenance.wasmSha256', 'not a hash'], ['observedAt', 'yesterday'],
    ['batches.wasm.samplesMs', [1, 2]], ['batches.wasm.samplesMs', [1, NaN, 3]],
    ['batches.wasm.samplesMs', [1, '2', 3]], ['batches.wasm.samplesMs', new Array(3)],
    ['batches.wasm.medianMs', 999], ['batches.unit', 'seconds'], ['batches.order', 'unknown']]) {
    const candidate = report();
    change(candidate, path, value);
    assert.throws(() => compareReports(report(), candidate), undefined, path);
  }
});

test('zero and unrepresentable ratios remain explicit null rather than JSON infinities', () => {
  const before = report();
  const after = report();
  before.buildApiMs = 0;
  assert.equal(compareReports(before, after).buildApiMs.ratio, null);
  before.buildApiMs = Number.MIN_VALUE;
  after.buildApiMs = Number.MAX_VALUE;
  assert.equal(compareReports(before, after).buildApiMs.ratio, null);
  assert.doesNotThrow(() => JSON.stringify(compareReports(before, after)));
});

test('CLI reads Unicode paths from another directory and fails without emitting a report', async t => {
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka comparison é '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const entry = fileURLToPath(new URL('../../tools/bench/compare.mjs', import.meta.url));
  const invoke = args => promisify(execFile)(process.execPath, [entry, ...args],
    { cwd, timeout: 5000, killSignal: 'SIGKILL', maxBuffer: 65536, shell: false });
  const names = ['old α.json', 'new 猫.json'];
  for (const name of names) await writeFile(join(cwd, name), JSON.stringify(report()));
  const result = await invoke(names);
  assert.equal(result.stderr, '');
  assert.equal(JSON.parse(result.stdout).batches.wasm.medianMs.ratio, 1);
  await writeFile(join(cwd, names[1]), '{');
  for (const args of [[], [names[0]], names, [names[0], 'missing.json']]) {
    await assert.rejects(invoke(args), error => {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, '');
      assert.ok(error.stderr.length > 0);
      return true;
    });
  }
  assert.deepEqual(JSON.parse(await readFile(join(cwd, names[0]), 'utf8')), report());
});
