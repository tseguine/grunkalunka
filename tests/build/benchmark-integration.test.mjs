import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';

const entry = fileURLToPath(new URL('../../tools/bench/numeric.mjs', import.meta.url));
const binary = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));

test('numeric timing probe compiles real source and emits a complete report outside the repo', async t => {
  await access(binary); // Missing build prerequisites are failures, never skips.
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka probe é '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { stdout, stderr } = await promisify(execFile)(process.execPath, [entry,
    '--compiler', binary, '--build-label', 'CI debug smoke', '--samples', '2', '--iterations', '16', '--warmup', '8'],
  { cwd, timeout: 60_000, maxBuffer: 1024 * 1024, killSignal: 'SIGKILL', shell: false });
  const report = JSON.parse(stdout);
  assert.equal(stderr, '');
  assert.equal(report.schemaVersion, 1);
  assert.equal(report.workload, 'numeric-gcd');
  assert.deepEqual(report.config, { samples: 2, iterations: 16, warmup: 8 });
  assert.equal(report.checksum, 142);
  assert.ok(Number.isFinite(report.buildApiMs) && report.buildApiMs >= 0);
  assert.ok(report.provenance.wasmBytes > 8);
  for (const key of ['sourceSha256', 'wasmSha256', 'harnessSha256']) {
    assert.match(report.provenance[key], /^[a-f0-9]{64}$/);
  }
  assert.equal(report.provenance.sourceSha256, createHash('sha256').update(
    await readFile(new URL('../../examples/numeric-workflow/gcd.js', import.meta.url))).digest('hex'));
  assert.match(report.provenance.compiler.version, /grunkalunka/i);
  assert.equal(report.provenance.compiler.buildLabel, 'CI debug smoke');
  assert.match(report.provenance.compiler.sha256, /^[a-f0-9]{64}$/);
  assert.equal(report.provenance.node, process.version);
  assert.equal(report.provenance.platform, process.platform);
  for (const engine of ['js', 'wasm']) assert.equal(report.batches[engine].samplesMs.length, 2);
  assert.deepEqual(await readdir(cwd), []);
});
