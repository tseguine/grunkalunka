// Compare saved Markdown/WIT receipts only within a bound same-process capture pair.
import { createHash, randomUUID } from 'node:crypto';
import { readFile, stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import {
  capture, parseCaptureArgs, validateOptionalArtifactHashes,
  validateOptionalFreshLifecycles, validateOptionalProcessMemory,
} from './capture-markdown.mjs';

const workloads = new Set(['empty', 'unicode', 'dense', 'wide']);
const osMap = { linux: 'linux', darwin: 'macos', win32: 'windows' };
const archMap = { x64: 'x86_64', arm64: 'aarch64' };
// Preserve the five-file legacy receipt shape for historical captures.
const legacyHarnessPaths = [
  'tools/bench/capture-markdown.mjs',
  'crates/grunkalunka-runtime/examples/markdown_cost.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/embedding.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/inspector.wat.in',
  'Cargo.lock',
];
const harnessPaths = [...legacyHarnessPaths,
  'crates/grunkalunka-runtime/examples/core_engine_cost/process_memory.rs'];
const shaPattern = /^[a-f0-9]{64}$/;
const isoPattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
const requireValid = (condition, name) => {
  if (!condition) throw new TypeError('Invalid ' + name);
};
function field(object, path) {
  let value = object;
  for (const part of path.split('.')) {
    if (!value || typeof value !== 'object' || !Object.hasOwn(value, part)) {
      throw new TypeError('Missing ' + path);
    }
    value = value[part];
  }
  return value;
}
const hash = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const nonempty = value => typeof value === 'string' && value.length > 0;
const milliseconds = (value, name) => requireValid(Number.isFinite(value) && value >= 0, name);
function validateSeries(series, samples, name) {
  const raw = field(series, 'samplesMs');
  requireValid(Array.isArray(raw) && raw.length === samples, name + '.samplesMs');
  raw.forEach(value => milliseconds(value, name + '.samplesMs'));
  const sorted = [...raw].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 ? sorted[mid] : sorted[mid - 1] / 2 + sorted[mid] / 2;
  for (const [key, expected] of [
    ['minMs', sorted[0]], ['medianMs', median], ['maxMs', sorted.at(-1)],
  ]) {
    const observed = field(series, key);
    milliseconds(observed, name + '.' + key);
    requireValid(Math.abs(observed - expected) < 1e-8, name + '.' + key);
  }
}

// Validation relies on the saved environment, never the host running this reader.
function validateCapture(receipt) {
  requireValid(field(receipt, 'schemaVersion') === 1, 'capture.schemaVersion');
  requireValid(field(receipt, 'workload') === 'markdown-wit-capture', 'capture.workload');
  const date = field(receipt, 'observedAt');
  requireValid(typeof date === 'string' && isoPattern.test(date) &&
    new Date(date).toISOString() === date, 'observedAt');
  requireValid(field(receipt, 'cachePolicy') ===
    'OS and Wasmtime caches uncontrolled; one compilation, then warm samples', 'cachePolicy');
  const prov = field(receipt, 'provenance');
  requireValid(/^[a-f0-9]{40}$/.test(field(prov, 'gitCommit')), 'gitCommit');
  requireValid(field(prov, 'trackedChangesPresent') === false, 'clean tracked worktree');
  requireValid(nonempty(field(prov, 'buildLabel')) && prov.buildLabel !== 'unspecified' &&
    /^[A-Za-z0-9_.+-]{1,96}$/.test(prov.buildLabel), 'explicit buildLabel');
  requireValid(shaPattern.test(field(prov, 'guest.sha256')), 'guest.sha256');
  requireValid(shaPattern.test(field(prov, 'runner.sha256')), 'runner.sha256');
  const guestBytes = field(prov, 'guest.bytes');
  requireValid(Number.isSafeInteger(guestBytes) && guestBytes >= 8 && guestBytes <= 1024 * 1024,
    'guest.bytes');
  const modern = Object.hasOwn(prov, 'harnessVersion');
  requireValid(!modern || prov.harnessVersion === 2, 'harnessVersion');
  const paths = modern ? harnessPaths : legacyHarnessPaths;
  const harness = field(prov, 'harnessSha256');
  requireValid(harness && typeof harness === 'object' && !Array.isArray(harness) &&
    isDeepStrictEqual(Object.keys(harness).sort(), [...paths].sort()), 'harness keys');
  for (const name of paths) requireValid(shaPattern.test(harness[name]), 'harness.' + name);
  const host = field(prov, 'host');
  requireValid(Object.hasOwn(osMap, field(host, 'os')), 'host.os');
  requireValid(Object.hasOwn(archMap, field(host, 'arch')), 'host.arch');
  for (const name of ['osRelease', 'cpuModel', 'node', 'v8', 'observedRustc']) {
    requireValid(nonempty(field(host, name)), 'host.' + name);
  }
  requireValid(/^v\d+\.\d+\.\d+/.test(host.node), 'host.node');
  requireValid(/^rustc \d+\.\d+/.test(host.observedRustc), 'host.observedRustc');
  requireValid(Number.isSafeInteger(field(host, 'cpuCount')) && host.cpuCount > 0,
    'host.cpuCount');
  requireValid(typeof field(host, 'nodeOptions') === 'string', 'host.nodeOptions');
  const native = field(receipt, 'report');
  // New harnesses are known to include the memory collector; losing its
  // report field must not masquerade as a historic no-memory capture.
  if (modern) requireValid(Object.hasOwn(native, 'processMemory'),
    'v2 native process memory');
  requireValid(field(native, 'schemaVersion') === 1, 'native.schemaVersion');
  requireValid(field(native, 'workload') === 'markdown-wit-inspection', 'native.workload');
  const config = field(native, 'configuration');
  requireValid(workloads.has(field(config, 'case')), 'configuration.case');
  for (const [name, lower, upper] of [
    ['samples', 2, 12], ['iterations', 1, 100], ['warmup', 0, 100],
  ]) {
    const value = field(config, name);
    requireValid(Number.isSafeInteger(value) && value >= lower && value <= upper,
      'configuration.' + name);
  }
  requireValid(field(native, 'environment.os') === osMap[host.os], 'environment.os');
  requireValid(field(native, 'environment.arch') === archMap[host.arch], 'environment.arch');
  validateOptionalProcessMemory(native, field(native, 'environment.os'));
  validateOptionalFreshLifecycles(native, config.samples);
  requireValid(field(native, 'environment.fuelEnabled') === false, 'fuel policy');
  requireValid(/^=\d+\.\d+\.\d+$/.test(field(native, 'environment.wasmtimeDependency')),
    'Wasmtime dependency');
  requireValid(field(native, 'artifact.guestBytes') === guestBytes, 'artifact.guestBytes');
  // Saved legacy receipts predate embedded digests. When present, bind the
  // native report's guest identity to the separately saved capture provenance.
  validateOptionalArtifactHashes(native,
    Object.hasOwn(native.artifact, 'guestSha256') ? prov.guest.sha256 : null);
  requireValid(Number.isSafeInteger(field(native, 'artifact.componentBytes')) &&
    native.artifact.componentBytes > guestBytes, 'artifact.componentBytes');
  requireValid(field(native, 'timings.unit') === 'milliseconds', 'timings.unit');
  requireValid(field(native, 'timings.order') === 'alternating', 'timings.order');
  for (const name of ['embedMs', 'componentCompileMs']) {
    milliseconds(field(native, 'timings.' + name), 'timings.' + name);
  }
  for (const name of ['instantiation', 'granted', 'denied']) {
    validateSeries(field(native, 'timings.' + name), config.samples, 'timings.' + name);
  }
  const repetitions = config.warmup + config.samples * config.iterations;
  for (const [name, expected] of [
    ['grantedBackingReads', repetitions + 1], ['deniedBackingReads', 0],
    ['grantedCleanups', repetitions + 2], ['deniedCleanups', repetitions + 1],
  ]) requireValid(field(native, 'outcomes.' + name) === expected, 'outcomes.' + name);
  for (const [name, maxLength] of [['sourceHex', 8192], ['outlineHex', 65536]]) {
    const encoded = field(native, 'fixture.' + name);
    requireValid(typeof encoded === 'string' && encoded.length <= maxLength &&
      /^(?:[a-f0-9]{2})*$/.test(encoded), 'fixture.' + name);
  }
  new TextDecoder('utf-8', { fatal: true }).decode(Buffer.from(native.fixture.sourceHex, 'hex'));
}

const matching = [
  'cachePolicy', 'provenance.gitCommit', 'provenance.buildLabel',
  'provenance.guest', 'provenance.runner', 'provenance.harnessSha256',
  'provenance.host', 'report.configuration', 'report.environment',
  'report.artifact', 'report.fixture',
];

/** Build a self-consistency binding for two captures taken sequentially in this process. */
export function bindMarkdownPair(baseline, candidate) {
  const pair = {
    schemaVersion: 1, workload: 'markdown-wit-same-process-pair',
    binding: { method: 'same-node-process-sequential', sessionId: randomUUID(),
      baselineSha256: hash(baseline), candidateSha256: hash(candidate) },
    captures: [baseline, candidate],
  };
  compareMarkdownPair(pair);
  return pair;
}

const delta = (a, b) => Number.isFinite(b - a) ? b - a : null;
const one = (a, b) => ({ baselineMs: a, candidateMs: b, deltaMs: delta(a, b) });
const memoryPhases = [
  'beforeEmbed', 'afterCompile', 'afterInstantiation',
  'afterWarmup', 'afterBatches',
];
const byteDelta = (a, b) => ({
  baselineBytes: a, candidateBytes: b, deltaBytes: b - a,
});

// A bound pair runs two *separate* native processes sequentially. Preserve
// observed memory as raw process-wide evidence, never a guest-only metric,
// peak-regression classification, machine comparison or timing ratio.
function compareProcessMemory(baseline, candidate) {
  const left = baseline.report.processMemory;
  const right = candidate.report.processMemory;
  if (!left || !right) {
    return {
      availability: 'not-comparable',
      baselinePresent: left !== undefined,
      candidatePresent: right !== undefined,
      interpretation: 'one or both receipts lack process memory evidence',
    };
  }
  requireValid(left.source === right.source, 'paired process memory source');
  if (left.source === 'unsupported-on-this-host') {
    return {
      availability: 'unsupported-on-this-host',
      scope: left.scope, unit: left.unit, source: left.source,
      interpretation: 'native Rust process memory was unavailable on this host',
    };
  }
  const phases = Object.fromEntries(memoryPhases.map(phase => [
    phase, {
      rssBytes: byteDelta(left.snapshots[phase].rssBytes, right.snapshots[phase].rssBytes),
      peakRssBytes: byteDelta(
        left.snapshots[phase].peakRssBytes,
        right.snapshots[phase].peakRssBytes),
    },
  ]));
  return {
    availability: 'paired-process-observations',
    scope: left.scope, unit: left.unit, source: left.source, phases,
    interpretation: 'candidate minus baseline bytes from separate native processes; not guest-attributed memory or a comparable performance ranking',
  };
}

export function compareMarkdownPair(pair) {
  requireValid(field(pair, 'schemaVersion') === 1, 'pair.schemaVersion');
  requireValid(field(pair, 'workload') === 'markdown-wit-same-process-pair', 'pair.workload');
  const binding = field(pair, 'binding');
  requireValid(field(binding, 'method') === 'same-node-process-sequential', 'binding.method');
  requireValid(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
    field(binding, 'sessionId')), 'binding.sessionId');
  const captures = field(pair, 'captures');
  requireValid(Array.isArray(captures) && captures.length === 2, 'captures');
  for (const capture of captures) validateCapture(capture);
  const [baseline, candidate] = captures;
  requireValid(
    Object.hasOwn(baseline.report, 'freshLifecycles') ===
      Object.hasOwn(candidate.report, 'freshLifecycles'),
    'matching fresh WIT lifecycle evidence');
  requireValid(
    Object.hasOwn(baseline.provenance, 'harnessVersion') ===
      Object.hasOwn(candidate.provenance, 'harnessVersion'),
    'matching capture harness versions');
  requireValid(field(binding, 'baselineSha256') === hash(baseline), 'binding.baselineSha256');
  requireValid(field(binding, 'candidateSha256') === hash(candidate), 'binding.candidateSha256');
  for (const path of matching) {
    if (!isDeepStrictEqual(field(baseline, path), field(candidate, path))) {
      throw new Error('Incompatible captures: ' + path);
    }
  }
  requireValid(baseline.observedAt <= candidate.observedAt, 'capture order');
  const first = baseline.report.timings;
  const second = candidate.report.timings;
  const series = {};
  for (const name of ['instantiation', 'granted', 'denied']) {
    series[name] = {
      baselineSamplesMs: [...first[name].samplesMs],
      candidateSamplesMs: [...second[name].samplesMs],
      median: one(first[name].medianMs, second[name].medianMs),
    };
  }
  return {
    schemaVersion: 1, workload: 'markdown-wit-repeat-comparison',
    scope: 'same-process and identical binaries only; diagnostic observations, not speed rankings',
    sessionId: binding.sessionId,
    baselineObservedAt: baseline.observedAt, candidateObservedAt: candidate.observedAt,
    configuration: { ...baseline.report.configuration },
    guestSha256: baseline.provenance.guest.sha256,
    runnerSha256: baseline.provenance.runner.sha256,
    embed: one(first.embedMs, second.embedMs),
    componentCompile: one(first.componentCompileMs, second.componentCompileMs),
    series,
    processMemory: compareProcessMemory(baseline, candidate),
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const args = process.argv.slice(2);
    if (args[0] === '--capture') {
      const options = parseCaptureArgs(args.slice(1));
      const baseline = await capture(options);
      const candidate = await capture(options);
      console.log(JSON.stringify(bindMarkdownPair(baseline, candidate), null, 2));
    } else {
      if (args.length !== 1) throw new TypeError('usage: compare-markdown.mjs PAIR.json OR --capture RUNNER GUEST [options]');
      let saved;
      if (args[0] === '-') {
        const chunks = []; let size = 0;
        for await (const chunk of process.stdin) {
          size += chunk.byteLength;
          requireValid(size <= 1024 * 1024, 'input byte limit');
          chunks.push(chunk);
        }
        saved = Buffer.concat(chunks).toString('utf8');
      } else {
        requireValid((await stat(args[0])).size <= 1024 * 1024, 'input byte limit');
        saved = await readFile(args[0], 'utf8');
      }
      console.log(JSON.stringify(compareMarkdownPair(JSON.parse(saved)), null, 2));
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
