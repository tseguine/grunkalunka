// P1: compare independently compiled portable GCD Wasm bytes across native hosts.
// This is a correctness and reproducibility observation, not a performance benchmark.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { prepare, replay } from './portable-gcd.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const fixturePath = 'examples/numeric-workflow/gcd.js';
const sha = /^[a-f0-9]{64}$/;
const base64 = /^[A-Za-z0-9+/]+={0,2}$/;
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

function child(command, args, label, binary = false) {
  const result = spawnSync(command, args, {
    cwd: root, shell: false, windowsHide: true, timeout: 60_000,
    maxBuffer: 64 * 1024, encoding: binary ? null : 'utf8',
  });
  if (result.error || result.signal || result.status !== 0 ||
      (result.stderr && result.stderr.length !== 0)) {
    throw new Error(label + ' failed: ' +
      (result.error?.message ?? result.stderr?.toString('utf8') ?? result.signal ?? result.status));
  }
  return result.stdout;
}

export function verifyTransferredIdentity(sourceBytes, wasmBytes, expected) {
  assert.ok(Buffer.isBuffer(sourceBytes) && sourceBytes.length >= 1 &&
    sourceBytes.length <= 16384, 'bounded canonical source bytes');
  assert.ok(Buffer.isBuffer(wasmBytes) && wasmBytes.length >= 8 &&
    wasmBytes.length <= 65536, 'bounded compiled Wasm bytes');
  assert.match(expected.sourceSha256 ?? '', sha, 'expected source hash');
  assert.match(expected.wasmSha256 ?? '', sha, 'expected Wasm hash');
  assert.match(expected.wasm64 ?? '', base64, 'expected canonical base64');
  const original = Buffer.from(expected.wasm64, 'base64');
  assert.ok(original.length >= 8 && original.length <= 65536, 'bounded transferred Wasm');
  assert.equal(original.toString('base64'), expected.wasm64, 'canonical transferred base64');
  assert.equal(sha256(sourceBytes), expected.sourceSha256, 'canonical source identity');
  assert.equal(sha256(original), expected.wasmSha256, 'transferred Wasm digest');
  assert.ok(wasmBytes.equals(original), 'independently compiled Wasm must match original bytes');
  return true;
}

export async function runNativeBuild(mode, nativeCompiler, expected) {
  assert.ok(['produce', 'verify'].includes(mode), 'expected produce or verify mode');
  assert.equal(typeof nativeCompiler, 'string');
  const compiler = resolve(nativeCompiler);
  // Git tree object bytes are stable despite Windows checkout autocrlf.
  const canonicalSource = child('git', ['show', 'HEAD:' + fixturePath],
    'read tracked GCD fixture', true);
  assert.ok(canonicalSource.length >= 1 && canonicalSource.length <= 16384,
    'bounded canonical source');
  const sourceHash = sha256(canonicalSource);
  const folder = await mkdtemp(join(tmpdir(), 'grunkalunka-native-gcd-'));
  try {
    const sourceFile = join(folder, 'canonical.js');
    const wasmFile = join(folder, 'compiled.wasm');
    const manifestFile = join(folder, 'manifest.json');
    await writeFile(sourceFile, canonicalSource, { flag: 'wx' });
    const compilation = child(compiler, ['compile', sourceFile, wasmFile],
      'compile canonical GCD source');
    assert.match(compilation, /^COMPILED \d+ bytes:/);
    const wasm = await readFile(wasmFile);
    assert.ok(wasm.length >= 8 && wasm.length <= 65536, 'bounded compiled Wasm');
    const wasmHash = sha256(wasm);
    if (mode === 'verify') verifyTransferredIdentity(canonicalSource, wasm, expected);

    // Reuse independently checked Node/Wasmtime binary64 GCD goldens from #76.
    const manifest = await prepare(wasmFile, 'p1-native-build');
    assert.equal(manifest.artifact.sha256, wasmHash);
    await writeFile(manifestFile, JSON.stringify(manifest), { flag: 'wx' });
    const result = await replay(wasmFile, manifestFile,
      { samples: 2, iterations: 3, warmup: 0 }, compiler);
    assert.equal(result.artifact.sha256, wasmHash);
    assert.equal(result.node.goldenCases, 8);
    assert.equal(result.node.checksum, 33);
    assert.equal(result.wasmtime.status, 'passed');
    assert.equal(result.wasmtime.goldenCases, 8);
    assert.equal(result.receiver.os, platform());
    assert.equal(result.receiver.arch, arch());
    const rustc = child('rustc', ['-vV'], 'inspect native Rust host');
    const host = /^host: ([^\r\n]+)$/m.exec(rustc)?.[1];
    assert.ok(host, 'native Rust host triple');
    return {
      schemaVersion: 1, workload: 'p1-native-gcd-output-identity', mode,
      source: { path: fixturePath, sha256: sourceHash, bytes: canonicalSource.length },
      artifact: { sha256: wasmHash, bytes: wasm.length },
      host: { os: platform(), arch: arch(), rustcHost: host,
        node: process.version, nativeCompilerSha256: sha256(await readFile(compiler)) },
      node: { goldenCases: result.node.goldenCases, checksum: result.node.checksum },
      wasmtime: { status: result.wasmtime.status,
        goldenCases: result.wasmtime.goldenCases,
        nativeRunnerSha256: result.wasmtime.nativeRunnerSha256 },
      ...(mode === 'produce' ? { wasm64: wasm.toString('base64') } : {}),
      interpretation: 'independent native builds of canonical Git source; equal portable Wasm bytes, not equal performance',
    };
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const [mode, compiler] = process.argv.slice(2);
    assert.equal(process.argv.length, 4,
      'usage: native-build-identity.mjs produce|verify PATH_TO_NATIVE_COMPILER');
    const expected = mode === 'verify' ? {
      sourceSha256: process.env.EXPECTED_SOURCE_SHA256,
      wasmSha256: process.env.EXPECTED_WASM_SHA256,
      wasm64: process.env.EXPECTED_WASM64,
    } : undefined;
    console.log(JSON.stringify(await runNativeBuild(mode, compiler, expected), null, 2));
  } catch (error) {
    console.error(error.stack ?? error.message);
    process.exitCode = 1;
  }
}
