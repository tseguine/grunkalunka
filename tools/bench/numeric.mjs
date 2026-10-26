import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { arch, cpus, platform, release } from 'node:os';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';
import { Worker } from 'node:worker_threads';
import { configuration } from './measure.mjs';

const sourceUrl = new URL('../../examples/numeric-workflow/gcd.js', import.meta.url);
const defaultCompiler = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
async function fileHash(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}

export function parseOptions(args) {
  const result = { compiler: defaultCompiler, buildLabel: 'unspecified' };
  const seen = new Set();
  const keys = { '--compiler': 'compiler', '--build-label': 'buildLabel',
    '--samples': 'samples', '--iterations': 'iterations', '--warmup': 'warmup' };
  for (let i = 0; i < args.length; i += 2) {
    if (!Object.hasOwn(keys, args[i])) throw new TypeError(`Unknown option: ${args[i]}`);
    const key = keys[args[i]];
    const value = args[i + 1];
    if (!key || seen.has(key) || !value || value.includes('\0')) throw new TypeError('Expected unique --option value pairs');
    seen.add(key);
    if (['samples', 'iterations', 'warmup'].includes(key)) {
      if (!/^\d+$/.test(value)) throw new RangeError(`Invalid ${key}`);
      result[key] = Number(value);
    } else result[key] = value;
  }
  return { ...result, compiler: resolve(result.compiler), ...configuration(result) };
}

async function inWorker(wasm, source, config) {
  const worker = new Worker(new URL('./worker.mjs', import.meta.url), { workerData: { wasm, source, config }, execArgv: [] });
  let timer;
  try {
    return await new Promise((resolve, reject) => {
      worker.once('message', resolve);
      worker.once('error', reject);
      worker.once('exit', code => reject(new Error(`Probe exited without a report (${code})`)));
      timer = setTimeout(() => reject(new Error('Numeric probe exceeded 120 seconds')), 120_000);
    });
  } finally {
    clearTimeout(timer);
    await worker.terminate();
  }
}

export async function probe(options) {
  const { compiler, buildLabel, ...config } = options;
  configuration(config);
  const sourceBytes = await readFile(sourceUrl);
  const source = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(sourceBytes);
  const compilerHash = await fileHash(compiler);
  const { stdout: version } = await promisify(execFile)(compiler, ['--version'], {
    timeout: 5000, maxBuffer: 4096, killSignal: 'SIGKILL', shell: false, windowsHide: true,
  });
  const { compileSource } = await import('../build/compile.mjs');
  const started = performance.now();
  const { wasm, stderr } = await compileSource({ compiler, source, language: 'js' });
  const buildApiMs = performance.now() - started;
  if (stderr) process.stderr.write(stderr);
  const measurements = await inWorker(wasm, source, config);
  // A rebuilt compiler must not be reported using an earlier executable's identity.
  if (await fileHash(compiler) !== compilerHash) throw new Error('Compiler changed during the probe');
  return {
    schemaVersion: 1, workload: 'numeric-gcd', observedAt: new Date().toISOString(),
    provenance: { sourceSha256: digest(sourceBytes), wasmSha256: digest(wasm), wasmBytes: wasm.length,
      compiler: { path: compiler, sha256: compilerHash, version: version.trim(), buildLabel },
      node: process.version, v8: process.versions.v8, platform: platform(), arch: arch(),
      osRelease: release(), cpuModel: cpus()[0]?.model ?? null,
      nodeOptions: process.env.NODE_OPTIONS ?? '', workerExecArgv: [],
      harnessSha256: digest(Buffer.concat(await Promise.all([
        readFile(new URL(import.meta.url)), readFile(new URL('./measure.mjs', import.meta.url)),
        readFile(new URL('./worker.mjs', import.meta.url)),
      ]))) },
    cachePolicy: 'OS and engine caches uncontrolled; first observed startup, then explicit warmup',
    buildApiMs, ...measurements,
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try { console.log(JSON.stringify(await probe(parseOptions(process.argv.slice(2))), null, 2)); }
  catch (error) { console.error(error.stderr || error.message); process.exitCode = 1; }
}
