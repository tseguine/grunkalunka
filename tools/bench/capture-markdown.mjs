// Capture Markdown/WIT measurements with verifiable inputs; never publish artifacts.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { arch, cpus, platform, release } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const nativeOS = { linux: 'linux', darwin: 'macos', win32: 'windows' };
const nativeArch = { x64: 'x86_64', arm64: 'aarch64' };
const workloads = new Set(['empty', 'unicode', 'dense', 'wide']);
// Capture-v1 checked these five files; v2 also identifies the transitive
// Rust source included by markdown_cost.rs through #[path].
const harnessPaths = [
  'tools/bench/capture-markdown.mjs',
  'crates/grunkalunka-runtime/examples/markdown_cost.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/embedding.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/inspector.wat.in',
  'Cargo.lock',
  'crates/grunkalunka-runtime/examples/core_engine_cost/process_memory.rs',
];
const requireValue = (condition, label) => {
  if (!condition) throw new TypeError('Invalid ' + label);
};
const valueAt = (object, path) => {
  let current = object;
  for (const segment of path.split('.')) {
    if (!current || typeof current !== 'object' || !Object.hasOwn(current, segment)) {
      throw new TypeError('Missing ' + path);
    }
    current = current[segment];
  }
  return current;
};
const finiteMs = (value, label) => requireValue(Number.isFinite(value) && value >= 0, label);

export function parseCaptureArgs(argv) {
  if (argv.length < 2 || argv.length % 2 !== 0) {
    throw new TypeError('usage: capture-markdown.mjs RUNNER GUEST [--name value ...]');
  }
  const [runner, guest, ...pairs] = argv;
  for (const [name, value] of [['runner', runner], ['guest', guest]]) {
    requireValue(typeof value === 'string' && value.length > 0 && !value.includes('\0'), name);
  }
  const config = { case: 'unicode', samples: 5, iterations: 20, warmup: 5 };
  let buildLabel = 'unspecified';
  const seen = new Set();
  for (let i = 0; i < pairs.length; i += 2) {
    const name = pairs[i];
    const value = pairs[i + 1];
    requireValue(!seen.has(name) && typeof value === 'string' && value.length > 0 &&
      !value.includes('\0'), name);
    seen.add(name);
    if (name === '--build-label') {
      requireValue(/^[A-Za-z0-9_.+-]{1,96}$/.test(value), name);
      buildLabel = value;
    } else if (name === '--workload') {
      requireValue(workloads.has(value), name);
      config.case = value;
    } else {
      const rules = { '--samples': ['samples', 2, 12], '--iterations': ['iterations', 1, 100],
        '--warmup': ['warmup', 0, 100] };
      requireValue(Object.hasOwn(rules, name) && /^\d+$/.test(value), name);
      const [key, min, max] = rules[name];
      const number = Number(value);
      requireValue(Number.isSafeInteger(number) && number >= min && number <= max, name);
      config[key] = number;
    }
  }
  return { runner: resolve(runner), guest: resolve(guest), config, buildLabel };
}

function checkSeries(series, count, label) {
  const numbers = valueAt(series, 'samplesMs');
  requireValue(Array.isArray(numbers) && numbers.length === count &&
    numbers.every(number => Number.isFinite(number) && number >= 0), label + ' samples');
  const sorted = [...numbers].sort((a, b) => a - b);
  const midpoint = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 ? sorted[midpoint] :
    sorted[midpoint - 1] / 2 + sorted[midpoint] / 2;
  for (const [key, expected] of [
    ['minMs', sorted[0]], ['medianMs', median], ['maxMs', sorted.at(-1)],
  ]) {
    const value = valueAt(series, key);
    finiteMs(value, label + '.' + key);
    // The native runner formats each floating-point sample to nine decimals.
    requireValue(Math.abs(value - expected) < 0.00000001, label + '.' + key);
  }
}

// Historical native-v1 receipts did not carry Rust process memory. If a
// later report claims memory evidence, validate its complete platform-specific
// envelope rather than treating arbitrary saved values as verified readings.
export function validateOptionalProcessMemory(report, nativeOs) {
  if (!Object.hasOwn(report, 'processMemory')) return;
  const memory = report.processMemory;
  requireValue(memory && typeof memory === 'object' && !Array.isArray(memory) &&
    Object.keys(memory).sort().join(',') === 'scope,snapshots,source,unit',
  'processMemory envelope');
  requireValue(memory.scope === 'entire-rust-process' && memory.unit === 'bytes',
    'processMemory scope/units');
  // Archived Windows captures genuinely reported unavailable; retain their
  // original evidence rather than inventing readings for those old reports.
  if (nativeOs === 'macos' ||
      (nativeOs === 'windows' && memory.source === 'unsupported-on-this-host')) {
    requireValue(memory.source === 'unsupported-on-this-host' &&
      memory.snapshots === null, 'unsupported native process memory');
    return;
  }
  const expectedSource = nativeOs === 'linux' ? 'linux-proc-status' :
    nativeOs === 'windows' ? 'windows-psapi-working-set' : null;
  requireValue(expectedSource !== null && memory.source === expectedSource &&
    memory.snapshots && typeof memory.snapshots === 'object' &&
    !Array.isArray(memory.snapshots), 'native process memory source');
  const phases = [
    'beforeEmbed', 'afterCompile', 'afterInstantiation', 'afterWarmup', 'afterBatches',
  ];
  requireValue(
    Object.keys(memory.snapshots).sort().join(',') === [...phases].sort().join(','),
    'complete native process memory phases');
  let previousPeak = 0;
  for (const phase of phases) {
    const sample = memory.snapshots[phase];
    requireValue(sample && typeof sample === 'object' &&
      !Array.isArray(sample) &&
      Object.keys(sample).sort().join(',') === 'peakRssBytes,rssBytes',
    'process memory sample ' + phase);
    requireValue(Number.isSafeInteger(sample.rssBytes) && sample.rssBytes >= 0 &&
      Number.isSafeInteger(sample.peakRssBytes) &&
      sample.peakRssBytes >= sample.rssBytes &&
      sample.peakRssBytes >= previousPeak,
    'process memory RSS/peak ' + phase);
    previousPeak = sample.peakRssBytes;
  }
}

// Earlier native-v1 receipts lack this post-measurement observation. If a
// report claims new fresh WIT lifecycle samples, validate all real timings.
export function validateOptionalFreshLifecycles(report, samples) {
  if (!Object.hasOwn(report, 'freshLifecycles')) return;
  const lifecycle = valueAt(report, 'freshLifecycles');
  requireValue(lifecycle && typeof lifecycle === 'object' &&
    !Array.isArray(lifecycle) &&
    Object.keys(lifecycle).sort().join(',') === 'count,denied,granted,unit',
  'freshLifecycles envelope');
  requireValue(lifecycle.count === samples &&
    lifecycle.unit === 'milliseconds per fresh component lifecycle',
  'freshLifecycles count/units');
  for (const name of ['granted', 'denied']) {
    checkSeries(lifecycle[name], samples, 'freshLifecycles.' + name);
  }
}

// Report integrity is not a speed threshold or a second CommonMark oracle.
export function validateNativeReport(report, config, guestBytes) {
  requireValue(valueAt(report, 'schemaVersion') === 1, 'schemaVersion');
  requireValue(valueAt(report, 'workload') === 'markdown-wit-inspection', 'workload');
  for (const key of ['case', 'samples', 'iterations', 'warmup']) {
    requireValue(valueAt(report, 'configuration.' + key) === config[key], 'configuration.' + key);
  }
  requireValue(valueAt(report, 'environment.os') === nativeOS[platform()], 'host OS');
  requireValue(valueAt(report, 'environment.arch') === nativeArch[arch()], 'host architecture');
  validateOptionalProcessMemory(report, valueAt(report, 'environment.os'));
  validateOptionalFreshLifecycles(report, config.samples);
  requireValue(valueAt(report, 'environment.fuelEnabled') === false, 'fuel policy');
  requireValue(/^=\d+\.\d+\.\d+$/.test(valueAt(report, 'environment.wasmtimeDependency')),
    'Wasmtime dependency declaration');
  requireValue(valueAt(report, 'artifact.guestBytes') === guestBytes, 'guest byte length');
  requireValue(Number.isSafeInteger(valueAt(report, 'artifact.componentBytes')) &&
    report.artifact.componentBytes > guestBytes, 'component byte length');
  requireValue(valueAt(report, 'timings.unit') === 'milliseconds', 'timing unit');
  requireValue(valueAt(report, 'timings.order') === 'alternating', 'sample order');
  for (const name of ['embedMs', 'componentCompileMs']) {
    finiteMs(valueAt(report, 'timings.' + name), name);
  }
  for (const name of ['instantiation', 'granted', 'denied']) {
    checkSeries(valueAt(report, 'timings.' + name), config.samples, name);
  }
  const repeated = config.warmup + config.samples * config.iterations;
  for (const [key, expected] of [
    ['grantedBackingReads', repeated + 1], ['deniedBackingReads', 0],
    ['grantedCleanups', repeated + 2], ['deniedCleanups', repeated + 1],
  ]) {
    requireValue(valueAt(report, 'outcomes.' + key) === expected, key);
  }
  for (const name of ['sourceHex', 'outlineHex']) {
    const hex = valueAt(report, 'fixture.' + name);
    requireValue(typeof hex === 'string' && /^(?:[a-f0-9]{2})*$/.test(hex), name);
    requireValue(hex.length <= (name === 'sourceHex' ? 8192 : 65536), name + ' bound');
  }
  new TextDecoder('utf-8', { fatal: true }).decode(Buffer.from(report.fixture.sourceHex, 'hex'));
  return report;
}

async function sha256(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}

function execute(command, args, options = {}) {
  const child = spawnSync(command, args, {
    cwd: root, encoding: 'utf8', timeout: 180_000, maxBuffer: 2 * 1024 * 1024,
    killSignal: 'SIGKILL', windowsHide: true, shell: false, ...options,
  });
  if (child.error) throw child.error;
  if (child.status !== 0 || child.signal !== null || child.stderr !== '') {
    throw new Error(command + ' exited unsuccessfully: ' + (child.stderr || child.signal || child.status));
  }
  return child.stdout.trim();
}

function rustcVersion() {
  const child = spawnSync('rustc', ['--version'], {
    cwd: root, encoding: 'utf8', timeout: 5_000, maxBuffer: 4096, shell: false,
    windowsHide: true,
  });
  return !child.error && child.status === 0 && !child.signal ? child.stdout.trim() : null;
}

export async function capture(options) {
  const guestInfo = await stat(options.guest);
  requireValue(guestInfo.isFile() && guestInfo.size >= 8 && guestInfo.size <= 1024 * 1024,
    'guest binary');
  const runnerInfo = await stat(options.runner);
  requireValue(runnerInfo.isFile(), 'native runner');
  const [guestSha, runnerSha, sources] = await Promise.all([
    sha256(options.guest), sha256(options.runner),
    Promise.all(harnessPaths.map(async path => [path, await sha256(resolve(root, path))])),
  ]);
  const gitCommit = execute('git', ['rev-parse', 'HEAD']);
  requireValue(/^[a-f0-9]{40}$/.test(gitCommit), 'repository commit');
  const trackedChanges = execute('git', ['status', '--porcelain', '--untracked-files=no']);
  const nativeArgs = [options.guest, '--workload', options.config.case,
    '--samples', String(options.config.samples),
    '--iterations', String(options.config.iterations), '--warmup', String(options.config.warmup)];
  const native = JSON.parse(execute(options.runner, nativeArgs));
  validateNativeReport(native, options.config, guestInfo.size);
  if (await sha256(options.guest) !== guestSha || await sha256(options.runner) !== runnerSha ||
      execute('git', ['rev-parse', 'HEAD']) !== gitCommit ||
      execute('git', ['status', '--porcelain', '--untracked-files=no']) !== trackedChanges) {
    throw new Error('Measurement inputs or tracked repository state changed during capture');
  }
  const cpu = cpus();
  return {
    schemaVersion: 1, workload: 'markdown-wit-capture', observedAt: new Date().toISOString(),
    cachePolicy: 'OS and Wasmtime caches uncontrolled; one compilation, then warm samples',
    provenance: {
      gitCommit, trackedChangesPresent: trackedChanges.length > 0,
      buildLabel: options.buildLabel,
      guest: { sha256: guestSha, bytes: guestInfo.size },
      runner: { sha256: runnerSha },
      // Separate from the unchanged capture-v1 envelope. Older captures
      // omit this field and used a five-file harness identity.
      harnessVersion: 2,
      harnessSha256: Object.fromEntries(sources),
      host: { os: platform(), arch: arch(), osRelease: release(),
        cpuModel: cpu[0]?.model ?? null, cpuCount: cpu.length,
        node: process.version, v8: process.versions.v8, nodeOptions: process.env.NODE_OPTIONS ?? '',
        observedRustc: rustcVersion() },
    },
    report: native,
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    console.log(JSON.stringify(await capture(parseCaptureArgs(process.argv.slice(2))), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
