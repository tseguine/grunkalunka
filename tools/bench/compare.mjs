import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { configuration, statistics } from './measure.mjs';

const startupKeys = ['moduleConstructionMs', 'instantiationMs', 'firstJsCallMs', 'firstWasmCallMs'];
const environmentKeys = ['node', 'v8', 'platform', 'arch', 'osRelease', 'cpuModel', 'nodeOptions', 'workerExecArgv'];
const goldens = [[1071, 462, 21], [84, 30, 6], [-84, 30, 6], [0, 0, 0],
  [0, -9, 9], [13, 17, 1], [81, 27, 27], [-55, -34, 1]];

function field(value, path) {
  for (const key of path.split('.')) {
    if (!value || typeof value !== 'object' || !Object.hasOwn(value, key)) throw new TypeError(`Missing ${path}`);
    value = value[key];
  }
  return value;
}
function requireValue(condition, label) {
  if (!condition) throw new TypeError(`Invalid ${label}`);
}
function timing(value, label) {
  requireValue(Number.isFinite(value) && value >= 0, label);
}

function validate(report) {
  requireValue(field(report, 'schemaVersion') === 1, 'schemaVersion');
  requireValue(field(report, 'workload') === 'numeric-gcd', 'workload');
  for (const name of ['samples', 'iterations', 'warmup']) field(report, `config.${name}`);
  const config = configuration(report.config);
  requireValue(isDeepStrictEqual(config, report.config), 'config');
  requireValue(isDeepStrictEqual(field(report, 'cases'), goldens), 'cases');
  const checksum = Math.floor(config.iterations / 8) * 71 +
    goldens.slice(0, config.iterations % 8).reduce((sum, entry) => sum + entry[2], 0);
  requireValue(field(report, 'checksum') === checksum, 'checksum');
  for (const path of ['observedAt', 'cachePolicy', 'provenance.compiler.path',
    'provenance.compiler.version', 'provenance.compiler.buildLabel',
    ...environmentKeys.slice(0, 5).map(key => `provenance.${key}`)]) {
    const value = field(report, path);
    requireValue(typeof value === 'string' && value.length > 0, path);
  }
  requireValue(Number.isFinite(Date.parse(report.observedAt)), 'observedAt');
  for (const name of ['sourceSha256', 'wasmSha256', 'harnessSha256', 'compiler.sha256']) {
    const value = field(report, `provenance.${name}`);
    requireValue(typeof value === 'string' && /^[a-f0-9]{64}$/.test(value), name);
  }
  const size = field(report, 'provenance.wasmBytes');
  requireValue(Number.isSafeInteger(size) && size >= 8, 'wasmBytes');
  const cpu = field(report, 'provenance.cpuModel');
  requireValue(cpu === null || (typeof cpu === 'string' && cpu.length > 0), 'cpuModel');
  requireValue(typeof field(report, 'provenance.nodeOptions') === 'string', 'nodeOptions');
  const flags = field(report, 'provenance.workerExecArgv');
  requireValue(Array.isArray(flags) && [...flags].every(flag => typeof flag === 'string'), 'workerExecArgv');
  requireValue(field(report, 'batches.unit') === 'milliseconds per batch', 'batches.unit');
  requireValue(field(report, 'batches.order') === 'alternating, JS first in sample 0', 'batches.order');
  timing(field(report, 'buildApiMs'), 'buildApiMs');
  for (const key of startupKeys) timing(field(report, `startup.${key}`), key);
  for (const engine of ['js', 'wasm']) {
    const samples = field(report, `batches.${engine}.samplesMs`);
    requireValue(Array.isArray(samples) && samples.length === config.samples, `${engine} sample count`);
    for (const sample of samples) timing(sample, `${engine} sample`);
    const actual = statistics(samples);
    for (const key of ['minMs', 'medianMs', 'maxMs']) {
      requireValue(field(report, `batches.${engine}.${key}`) === actual[key], `${engine}.${key}`);
    }
  }
}

function difference(baseline, candidate) {
  const ratio = baseline === 0 ? NaN : candidate / baseline;
  return { baseline, candidate, delta: candidate - baseline,
    ratio: Number.isFinite(ratio) ? ratio : null };
}

/** Compare saved schema-v1 GCD reports; never execute report source or binaries. */
export function compareReports(baseline, candidate) {
  validate(baseline);
  validate(candidate);
  for (const path of ['config', 'cases', 'checksum', 'cachePolicy',
    'provenance.sourceSha256', 'provenance.harnessSha256',
    ...environmentKeys.map(key => `provenance.${key}`)]) {
    if (!isDeepStrictEqual(field(baseline, path), field(candidate, path))) {
      throw new Error(`Incompatible reports: ${path}`);
    }
  }
  const identify = report => ({ observedAt: report.observedAt, provenance: structuredClone(report.provenance) });
  const batches = {};
  for (const engine of ['js', 'wasm']) {
    const before = statistics(baseline.batches[engine].samplesMs);
    const after = statistics(candidate.batches[engine].samplesMs);
    batches[engine] = { baseline: before, candidate: after, medianMs: difference(before.medianMs, after.medianMs) };
  }
  return {
    schemaVersion: 1, workload: 'numeric-gcd', config: { ...baseline.config },
    baseline: identify(baseline), candidate: identify(candidate), cachePolicy: baseline.cachePolicy,
    buildApiMs: difference(baseline.buildApiMs, candidate.buildApiMs),
    startup: Object.fromEntries(startupKeys.map(key => [key, difference(baseline.startup[key], candidate.startup[key])])),
    batches, wasmBytes: difference(baseline.provenance.wasmBytes, candidate.provenance.wasmBytes),
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const paths = process.argv.slice(2);
    if (paths.length !== 2) throw new TypeError('usage: node tools/bench/compare.mjs BASELINE.json CANDIDATE.json');
    const reports = await Promise.all(paths.map(async path => JSON.parse(await readFile(path, 'utf8'))));
    console.log(JSON.stringify(compareReports(...reports), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
