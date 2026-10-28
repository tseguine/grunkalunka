import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';
import { compareReports } from '../../tools/bench/compare.mjs';

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
  const comparison = compareReports(report, structuredClone(report));
  assert.equal(comparison.buildApiMs.delta, 0);
  for (const engine of ['js', 'wasm']) {
    assert.equal(comparison.batches[engine].medianMs.delta, 0);
    assert.deepEqual(comparison.batches[engine].baseline.samplesMs, report.batches[engine].samplesMs);
  }
  assert.deepEqual(await readdir(cwd), []);
});

// Cross-machine artifact transport: actual Wasm bytes, no compilation on the receiver.
const portableEntry = fileURLToPath(new URL('../../tools/bench/portable-gcd.mjs', import.meta.url));
const gcdSource = fileURLToPath(new URL('../../examples/numeric-workflow/gcd.js', import.meta.url));

test('portable GCD options require an explicit replay engine and bounded work', async () => {
  const { parseArgs, validateManifest } = await import('../../tools/bench/portable-gcd.mjs');
  assert.equal(parseArgs(['prepare', 'artifact.wasm', '--build-label', 'ci-debug']).buildLabel, 'ci-debug');
  assert.equal(parseArgs(['replay', 'a.wasm', 'manifest.json', '--node-only']).nodeOnly, true);
  assert.deepEqual(parseArgs(['replay', 'a.wasm', 'manifest.json', '--runner', 'cli',
    '--samples', '2', '--iterations', '3', '--warmup', '0']).config,
  { samples: 2, iterations: 3, warmup: 0 });
  for (const args of [[], ['prepare'], ['replay', 'a.wasm'],
    ['replay', 'a.wasm', 'manifest.json'],
    ['replay', 'a.wasm', 'manifest.json', '--node-only', '--runner', 'cli'],
    ['replay', 'a.wasm', 'manifest.json', '--node-only', '--samples', '1'],
    ['replay', 'a.wasm', 'manifest.json', '--node-only', '--iterations', '10001'],
    ['replay', 'a.wasm', 'manifest.json', '--node-only', '--warmup', '-1'],
    ['replay', 'a.wasm', 'manifest.json', '--node-only', '--samples', '2', '--samples', '3'],
    ['prepare', 'a.wasm', '--build-label', 'with spaces'],
    ['prepare', 'a.wasm', '--samples', '2']]) {
    assert.throws(() => parseArgs(args), undefined, args.join(' '));
  }
  assert.throws(() => validateManifest({}), /manifest envelope/);
});

test('the identical saved GCD Wasm artifact replays through Node and Wasmtime without recompiling', async t => {
  await access(binary);
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka portability é '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const wasmPath = join(cwd, 'gcd shared.wasm');
  const manifestPath = join(cwd, 'manifest 猫.json');
  const invoke = args => promisify(execFile)(process.execPath, [portableEntry, ...args],
    { cwd, timeout: 60_000, maxBuffer: 1024 * 1024, killSignal: 'SIGKILL', shell: false });
  const compilation = await promisify(execFile)(binary, ['compile', gcdSource, wasmPath],
    { cwd, timeout: 60_000, maxBuffer: 1024 * 1024, killSignal: 'SIGKILL', shell: false });
  assert.match(compilation.stdout, /^COMPILED /);
  assert.equal(compilation.stderr, '');
  const prepared = await invoke(['prepare', wasmPath, '--build-label', 'ci-debug']);
  assert.equal(prepared.stderr, '');
  const manifest = JSON.parse(prepared.stdout);
  const wasm = await readFile(wasmPath);
  assert.equal(manifest.artifact.sha256, createHash('sha256').update(wasm).digest('hex'));
  assert.equal(manifest.artifact.bytes, wasm.length);
  assert.equal(manifest.source.sha256,
    createHash('sha256').update(await readFile(gcdSource)).digest('hex'));
  assert.equal(manifest.goldens.length, 8);
  assert.equal(manifest.goldens[0].resultHex, '4035000000000000');
  await writeFile(manifestPath, prepared.stdout);

  const full = await invoke(['replay', wasmPath, manifestPath, '--runner', binary,
    '--samples', '2', '--iterations', '3', '--warmup', '1']);
  assert.equal(full.stderr, '');
  const receipt = JSON.parse(full.stdout);
  assert.equal(receipt.artifact.sha256, manifest.artifact.sha256);
  assert.equal(receipt.workload, 'portable-core-gcd-replay');
  assert.equal(receipt.wasmtime.status, 'passed');
  assert.equal(receipt.wasmtime.goldenCases, 8);
  assert.match(receipt.wasmtime.nativeRunnerSha256, /^[a-f0-9]{64}$/);
  assert.equal(receipt.node.checksum, 33);
  assert.equal(receipt.node.engine, 'node-v8');
  assert.equal(receipt.node.memory.scope, 'entire-node-process');
  assert.equal(receipt.node.batches.samplesMs.length, 2);
  assert.equal(receipt.original.buildLabel, 'ci-debug');
  assert.equal(receipt.receiver.arch, process.arch);

  const nodeOnly = JSON.parse((await invoke(['replay', wasmPath, manifestPath,
    '--node-only', '--samples', '2', '--iterations', '1', '--warmup', '0'])).stdout);
  assert.deepEqual(nodeOnly.wasmtime, { status: 'not-run', reason: 'explicit --node-only' });

  const changed = structuredClone(manifest);
  changed.goldens[0].resultHex = '0000000000000000';
  await writeFile(manifestPath, JSON.stringify(changed));
  await assert.rejects(invoke(['replay', wasmPath, manifestPath, '--runner', binary]), e => {
    assert.equal(e.code, 1);
    assert.equal(e.stdout, '');
    assert.match(e.stderr, /unaltered independent fixed goldens/);
    return true;
  });
  await writeFile(manifestPath, prepared.stdout);
  const corrupted = join(cwd, 'corrupted.wasm');
  const altered = Buffer.from(wasm);
  altered[altered.length - 1] ^= 1;
  await writeFile(corrupted, altered);
  await assert.rejects(invoke(['replay', corrupted, manifestPath, '--runner', binary]), e => {
    assert.equal(e.code, 1);
    assert.equal(e.stdout, '');
    assert.match(e.stderr, /original artifact bytes/);
    return true;
  });
});
