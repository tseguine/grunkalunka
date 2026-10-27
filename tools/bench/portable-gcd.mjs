// Replay a verified, compiler-produced GCD artifact on another host without compiling it.
// Portability receipts are observations, not an attestation or engine speed ranking.
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readFile, stat } from 'node:fs/promises';
import { arch, cpus, platform, release } from 'node:os';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sourceURL = new URL('../../examples/numeric-workflow/gcd.js', import.meta.url);
const replayScript = fileURLToPath(new URL('./core-engine-replay.mjs', import.meta.url));
const MAX_WASM = 1024 * 1024;
const MAX_MANIFEST = 64 * 1024;
const SHA256 = /^[a-f0-9]{64}$/;
const SHA1 = /^[a-f0-9]{40}$/;
const cases = Object.freeze([[1071, 462, 21], [84, 30, 6], [-84, 30, 6], [0, 0, 0],
  [0, -9, 9], [13, 17, 1], [81, 27, 27], [-55, -34, 1]]);

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const bits = value => {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, value, false);
  return view.getBigUint64(0, false).toString(16).padStart(16, '0');
};
const goldens = cases.map(([a, b, expected]) => ({ aHex: bits(a), bHex: bits(b), resultHex: bits(expected) }));
const expectedChecksum = iterations => {
  const full = Math.floor(iterations / cases.length);
  return full * cases.reduce((sum, [, , output]) => sum + output, 0) +
    cases.slice(0, iterations % cases.length).reduce((sum, [, , output]) => sum + output, 0);
};
const requireValid = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const pathValue = (path, label) => {
  requireValid(typeof path === 'string' && path.length > 0 && !path.includes('\0'), label);
  return resolve(path);
};
const configDefaults = { samples: 5, iterations: 100, warmup: 20 };

export function parseArgs(argv) {
  const [command, ...args] = argv;
  const prepare = command === 'prepare';
  if (!prepare && command !== 'replay') {
    throw new TypeError('usage: portable-gcd.mjs prepare WASM [--build-label LABEL] | replay WASM MANIFEST (--runner CLI | --node-only) [--samples N --iterations N --warmup N]');
  }
  const positional = prepare ? 1 : 2;
  if (args.length < positional) throw new TypeError('Missing portable GCD input');
  const output = { command, artifact: pathValue(args[0], 'artifact') };
  if (!prepare) output.manifest = pathValue(args[1], 'manifest');
  const seen = new Set();
  const config = { ...configDefaults };
  let buildLabel = 'unverified';
  let runner;
  let nodeOnly = false;
  for (let i = positional; i < args.length; i++) {
    const option = args[i];
    requireValid(typeof option === 'string' && option.startsWith('--') && !seen.has(option), 'unique option');
    seen.add(option);
    if (option === '--node-only' && !prepare) {
      nodeOnly = true;
      continue;
    }
    const value = args[++i];
    requireValid(typeof value === 'string' && value.length > 0 && !value.includes('\0'), option);
    if (option === '--build-label' && prepare) {
      requireValid(/^[A-Za-z0-9_.+-]{1,96}$/.test(value), 'build label');
      buildLabel = value;
    } else if (option === '--runner' && !prepare) {
      runner = pathValue(value, 'runner');
    } else if (!prepare && ['--samples', '--iterations', '--warmup'].includes(option)) {
      requireValid(/^\d+$/.test(value), option);
      const key = option.slice(2);
      const n = Number(value);
      const bounds = { samples: [2, 20], iterations: [1, 10000], warmup: [0, 10000] };
      const [min, max] = bounds[key];
      requireValid(Number.isSafeInteger(n) && n >= min && n <= max, option);
      config[key] = n;
    } else {
      throw new TypeError('Unknown option: ' + option);
    }
  }
  if (prepare) return { ...output, buildLabel };
  requireValid(nodeOnly !== Boolean(runner), 'exactly one of --runner or --node-only');
  return { ...output, config, runner, nodeOnly };
}

async function boundedFile(path, maximum, label) {
  const info = await stat(path);
  requireValid(info.isFile() && info.size > 0 && info.size <= maximum, label + ' file size');
  const content = await readFile(path);
  requireValid(content.length > 0 && content.length <= maximum, label + ' loaded bytes');
  return content;
}

async function fileHash(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}

function run(command, args, input) {
  const result = spawnSync(command, args, {
    input, cwd: root, shell: false, timeout: 60_000, maxBuffer: 1024 * 1024,
    encoding: input instanceof Uint8Array ? null : 'utf8', windowsHide: true,
    killSignal: 'SIGKILL',
  });
  if (result.error || result.signal || result.status !== 0 || result.stderr.length) {
    const details = result.stderr?.toString('utf8') || result.error?.message || result.signal || result.status;
    throw new Error('Replay subprocess failed: ' + details);
  }
  return result.stdout.toString('utf8');
}

function nodeReplay(wasm, config) {
  const output = run(process.execPath, [replayScript, String(config.samples),
    String(config.iterations), String(config.warmup)], wasm);
  const report = JSON.parse(output);
  requireValid(report.schemaVersion === 1 && report.engine === 'node-v8', 'Node engine report');
  requireValid(report.artifact?.bytes === wasm.length && report.artifact?.sha256 === digest(wasm) &&
    report.artifact.imports === 0, 'same module identity and absence of imports');
  requireValid(isDeepStrictEqual(report.configuration, config) && report.goldenCases === cases.length &&
    report.checksum === expectedChecksum(config.iterations), 'Node golden report');
  requireValid(report.batches?.samplesMs?.length === config.samples &&
    report.memory?.scope === 'entire-node-process', 'Node batch and memory report');
  return report;
}

function provenance() {
  const revision = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root,
    encoding: 'utf8', windowsHide: true, shell: false, timeout: 5000, maxBuffer: 4096 });
  const commit = !revision.error && revision.status === 0 && !revision.signal &&
    SHA1.test(revision.stdout.trim()) ? revision.stdout.trim() : null;
  return { observedAt: new Date().toISOString(), gitCommit: commit,
    os: platform(), arch: arch(), osRelease: release(), cpuModel: cpus()[0]?.model ?? null,
    node: process.version, v8: process.versions.v8, nodeOptions: process.env.NODE_OPTIONS ?? '' };
}

export function validateManifest(manifest) {
  requireValid(manifest && typeof manifest === 'object' && !Array.isArray(manifest) &&
    manifest.schemaVersion === 1 && manifest.workload === 'portable-core-gcd', 'manifest envelope');
  const artifact = manifest.artifact;
  requireValid(artifact && typeof artifact === 'object' && SHA256.test(artifact.sha256) &&
    Number.isSafeInteger(artifact.bytes) && artifact.bytes >= 8 && artifact.bytes <= MAX_WASM,
    'manifest artifact identity');
  const source = manifest.source;
  requireValid(source && typeof source === 'object' &&
    source.path === 'examples/numeric-workflow/gcd.js' && SHA256.test(source.sha256) &&
    Number.isSafeInteger(source.bytes) && source.bytes >= 1 && source.bytes <= 16384,
    'manifest source identity');
  requireValid(isDeepStrictEqual(manifest.goldens, goldens), 'unaltered independent fixed goldens');
  requireValid(typeof manifest.buildLabel === 'string' &&
    /^[A-Za-z0-9_.+-]{1,96}$/.test(manifest.buildLabel), 'manifest build label');
  const origin = manifest.origin;
  requireValid(origin && typeof origin === 'object' &&
    typeof origin.observedAt === 'string' && Number.isFinite(Date.parse(origin.observedAt)) &&
    (origin.gitCommit === null || SHA1.test(origin.gitCommit)) &&
    ['linux', 'darwin', 'win32'].includes(origin.os) &&
    ['x64', 'arm64'].includes(origin.arch) &&
    typeof origin.osRelease === 'string' && origin.osRelease.length > 0 &&
    (origin.cpuModel === null || typeof origin.cpuModel === 'string') &&
    typeof origin.node === 'string' && /^v\d+\.\d+\.\d+/.test(origin.node) &&
    typeof origin.v8 === 'string' && origin.v8.length > 0 &&
    typeof origin.nodeOptions === 'string', 'manifest producer');
  return manifest;
}

export async function prepare(wasmFile, buildLabel = 'unverified') {
  const wasm = await boundedFile(wasmFile, MAX_WASM, 'Wasm');
  // The separate Node process has its own watchdog; no arbitrary guest executes
  // inside the manifest producer and no source compilation occurs here.
  nodeReplay(wasm, { samples: 2, iterations: 1, warmup: 0 });
  const source = await readFile(sourceURL);
  return validateManifest({
    schemaVersion: 1, workload: 'portable-core-gcd',
    artifact: { sha256: digest(wasm), bytes: wasm.length },
    source: { path: 'examples/numeric-workflow/gcd.js', sha256: digest(source), bytes: source.length },
    goldens, buildLabel, origin: provenance(),
  });
}

export async function replay(wasmFile, manifestFile, config, runner) {
  const manifestBytes = await boundedFile(manifestFile, MAX_MANIFEST, 'manifest');
  const manifest = validateManifest(JSON.parse(manifestBytes.toString('utf8')));
  const wasm = await boundedFile(wasmFile, MAX_WASM, 'Wasm');
  requireValid(manifest.artifact.sha256 === digest(wasm) &&
    manifest.artifact.bytes === wasm.length, 'original artifact bytes');
  const node = nodeReplay(wasm, config);
  let wasmtime;
  if (runner === undefined) {
    wasmtime = { status: 'not-run', reason: 'explicit --node-only' };
  } else {
    const input = goldens.map(c => c.aHex + ' ' + c.bHex).join('\n') + '\n';
    const output = run(runner, ['eval', wasmFile], input);
    const results = output.trim().split(/\r?\n/);
    requireValid(results.length === goldens.length && results.every((actual, i) =>
      actual === goldens[i].resultHex), 'Wasmtime CLI independent golden results');
    wasmtime = { status: 'passed', goldenCases: results.length, nativeRunnerSha256: await fileHash(runner) };
  }
  requireValid(await fileHash(wasmFile) === manifest.artifact.sha256,
    'artifact must remain unchanged during replay');
  return {
    schemaVersion: 1, workload: 'portable-core-gcd-replay',
    artifact: { ...manifest.artifact }, manifestSha256: digest(manifestBytes),
    original: { source: { ...manifest.source }, buildLabel: manifest.buildLabel,
      origin: { ...manifest.origin } },
    receiver: provenance(), node, wasmtime,
    interpretation: 'same artifact bytes; no compiler on receiver; no cross-host speed ratio',
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const options = parseArgs(process.argv.slice(2));
    const data = options.command === 'prepare' ?
      await prepare(options.artifact, options.buildLabel) :
      await replay(options.artifact, options.manifest, options.config, options.runner);
    console.log(JSON.stringify(data, null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
