import assert from 'node:assert/strict';
import { test } from 'node:test';
import { comparePortableReplays } from '../../tools/bench/compare-portable-replays.mjs';

const hash = 'a'.repeat(64);
const host = { observedAt: '2026-10-28T08:00:00.000Z', gitCommit: null,
  os: 'linux', arch: 'x64', osRelease: 'test', cpuModel: 'test',
  node: 'v24.22.1', v8: 'test', nodeOptions: '' };
function receipt() {
  return {
    schemaVersion: 1, workload: 'portable-core-gcd-replay',
    artifact: { sha256: hash, bytes: 614 }, manifestSha256: 'b'.repeat(64),
    original: {
      source: { path: 'examples/numeric-workflow/gcd.js',
        sha256: 'c'.repeat(64), bytes: 228 },
      buildLabel: 'test', origin: structuredClone(host),
    },
    receiver: structuredClone(host),
    node: {
      schemaVersion: 1, engine: 'node-v8',
      artifact: { sha256: hash, bytes: 614, imports: 0 },
      environment: { arch: 'x64', os: 'linux', node: 'v24.22.1', v8: 'test' },
      configuration: { samples: 2, iterations: 3, warmup: 0 },
      goldenCases: 8, checksum: 33, batches: { samplesMs: [1, 2] },
    },
    wasmtime: { status: 'passed', goldenCases: 8, nativeRunnerSha256: 'd'.repeat(64) },
  };
}
test('compares same producer across independently reported architectures without ranking', () => {
  const first = receipt(), second = receipt();
  second.receiver.arch = 'arm64';
  second.node.environment.arch = 'arm64';
  second.wasmtime.nativeRunnerSha256 = 'e'.repeat(64);
  const report = comparePortableReplays(first, second);
  assert.equal(report.artifact.sha256, hash);
  assert.deepEqual(report.receivers.map(r => r.host.arch), ['x64', 'arm64']);
  assert.equal(report.receivers[1].wasmtime.status, 'passed');
  assert.ok(!JSON.stringify(report).includes('samplesMs'));
  assert.ok(!JSON.stringify(report).includes('speedup'));
});
test('refuses mismatched producers and modified artifact identities', () => {
  const a = receipt(), b = receipt();
  b.manifestSha256 = 'e'.repeat(64);
  assert.throws(() => comparePortableReplays(a, b), /shared producer/);
  b.manifestSha256 = a.manifestSha256;
  b.node.artifact.sha256 = 'f'.repeat(64);
  assert.throws(() => comparePortableReplays(a, b), /Node artifact/);
});
test('refuses false golden results and receiver identity', () => {
  const a = receipt(), b = receipt();
  b.node.checksum = 34;
  assert.throws(() => comparePortableReplays(a, b), /Node goldens/);
  b.node.checksum = 33;
  b.receiver.arch = 'arm64';
  assert.throws(() => comparePortableReplays(a, b), /receiver Node identity/);
});
test('preserves explicit Node-only limitations', () => {
  const a = receipt(), b = receipt();
  b.wasmtime = { status: 'not-run', reason: 'explicit --node-only' };
  const report = comparePortableReplays(a, b);
  assert.equal(report.receivers[1].wasmtime.status, 'not-run');
  b.wasmtime.reason = 'other';
  assert.throws(() => comparePortableReplays(a, b), /Wasmtime result/);
});
