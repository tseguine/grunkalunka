import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { runNativeBuild } from '../../tools/bench/native-build-identity.mjs';
import { verifyArtifact } from '../../tools/bench/verify-portable-affine.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const native = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target', 'debug',
  process.platform === 'win32' ? 'grunkalunka.exe' : 'grunkalunka');

test('native compiler GCD Wasm executes against independent Node and Wasmtime goldens', async () => {
  // Uses the canonical Git source, not an untracked or shell-transformed copy.
  const report = await runNativeBuild('produce', native);
  assert.equal(report.workload, 'p1-native-gcd-output-identity');
  assert.equal(report.node.goldenCases, 8);
  assert.equal(report.node.checksum, 33);
  assert.equal(report.wasmtime.status, 'passed');
  assert.equal(report.wasmtime.goldenCases, 8);
  assert.match(report.artifact.sha256, /^[a-f0-9]{64}$/);
});

test('compiled affine Wasm meets all twelve binary64 goldens on two native engines', async t => {
  const temp = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-affine-'));
  t.after(() => rm(temp, { recursive: true, force: true }));
  const wasm = join(temp, 'affine.wasm');
  const source = fileURLToPath(new URL('../../examples/numeric-workflow/affine.js', import.meta.url));
  const built = spawnSync(native, ['compile', source, wasm], {
    cwd: root, shell: false, windowsHide: true, timeout: 60_000,
    encoding: 'utf8', maxBuffer: 1024 * 1024,
  });
  assert.equal(built.status, 0, built.error?.message ?? built.stderr ?? 'affine compile failed');
  assert.equal(built.signal, null);
  const report = await verifyArtifact(wasm, native);
  assert.equal(report.workload, 'portable-core-affine');
  assert.equal(report.node.goldenCases, 12);
  assert.equal(report.wasmtime.goldenCases, 12);
  assert.equal(report.artifact.imports, 0);
  assert.match(report.artifact.sha256, /^[a-f0-9]{64}$/);
});
