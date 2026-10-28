// Validate two saved portable-GCD replay receipts without executing artifacts.
// Equality of recorded digests is not attestation of execution or machine identity.
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const sha = /^[a-f0-9]{64}$/;
const requireValid = (valid, field) => {
  if (!valid) throw new TypeError('Invalid ' + field);
};
const own = (value, key) => value && typeof value === 'object' &&
  !Array.isArray(value) && Object.hasOwn(value, key);
const field = (value, key) => {
  requireValid(own(value, key), key);
  return value[key];
};
function validate(receipt) {
  requireValid(field(receipt, 'schemaVersion') === 1 &&
    field(receipt, 'workload') === 'portable-core-gcd-replay', 'receipt schema');
  const artifact = field(receipt, 'artifact');
  requireValid(sha.test(field(artifact, 'sha256')) &&
    Number.isSafeInteger(field(artifact, 'bytes')) &&
    artifact.bytes >= 8 && artifact.bytes <= 1024 * 1024, 'artifact');
  requireValid(sha.test(field(receipt, 'manifestSha256')), 'manifest hash');
  const original = field(receipt, 'original');
  const source = field(original, 'source');
  requireValid(field(source, 'path') === 'examples/numeric-workflow/gcd.js' &&
    sha.test(field(source, 'sha256')) &&
    Number.isSafeInteger(field(source, 'bytes')) && source.bytes > 0,
  'source identity');
  requireValid(typeof field(original, 'buildLabel') === 'string', 'build label');
  const origin = field(original, 'origin');
  const receiver = field(receipt, 'receiver');
  for (const [name, host] of [['origin', origin], ['receiver', receiver]]) {
    requireValid(['x64', 'arm64'].includes(field(host, 'arch')) &&
      ['linux', 'darwin', 'win32'].includes(field(host, 'os')) &&
      typeof field(host, 'node') === 'string' &&
      /^v\d+\.\d+\.\d+/.test(host.node) &&
      typeof field(host, 'v8') === 'string' &&
      typeof field(host, 'observedAt') === 'string' &&
      Number.isFinite(Date.parse(host.observedAt)), name);
  }
  const node = field(receipt, 'node');
  requireValid(field(node, 'schemaVersion') === 1 &&
    field(node, 'engine') === 'node-v8' &&
    field(node, 'artifact').sha256 === artifact.sha256 &&
    node.artifact.bytes === artifact.bytes &&
    node.artifact.imports === 0, 'Node artifact');
  const env = field(node, 'environment');
  requireValid(env.arch === receiver.arch && env.os === receiver.os &&
    env.node === receiver.node && env.v8 === receiver.v8, 'receiver Node identity');
  const config = field(node, 'configuration');
  requireValid(Number.isSafeInteger(field(config, 'samples')) &&
    config.samples >= 2 && config.samples <= 20 &&
    Number.isSafeInteger(field(config, 'iterations')) &&
    config.iterations >= 1 && config.iterations <= 10000 &&
    Number.isSafeInteger(field(config, 'warmup')) &&
    config.warmup >= 0 && config.warmup <= 10000, 'configuration');
  const cases = [21, 6, 6, 0, 9, 1, 27, 1];
  const full = Math.floor(config.iterations / cases.length);
  const expected = full * cases.reduce((sum, v) => sum + v, 0) +
    cases.slice(0, config.iterations % cases.length).reduce((sum, v) => sum + v, 0);
  requireValid(field(node, 'goldenCases') === cases.length &&
    field(node, 'checksum') === expected, 'Node goldens');
  const samples = field(field(node, 'batches'), 'samplesMs');
  requireValid(Array.isArray(samples) && samples.length === config.samples &&
    samples.every(v => typeof v === 'number' && Number.isFinite(v) && v >= 0),
  'Node timing samples');
  const wasmtime = field(receipt, 'wasmtime');
  requireValid(wasmtime.status === 'passed' ?
    wasmtime.goldenCases === cases.length && sha.test(wasmtime.nativeRunnerSha256) :
    wasmtime.status === 'not-run' && wasmtime.reason === 'explicit --node-only',
  'Wasmtime result');
  return receipt;
}

export function comparePortableReplays(left, right) {
  validate(left);
  validate(right);
  requireValid(isDeepStrictEqual(left.artifact, right.artifact) &&
    isDeepStrictEqual(left.original.source, right.original.source) &&
    left.manifestSha256 === right.manifestSha256 &&
    left.original.buildLabel === right.original.buildLabel &&
    isDeepStrictEqual(left.original.origin, right.original.origin),
  'shared producer artifact and manifest');
  return {
    schemaVersion: 1, workload: 'portable-core-gcd-replay-comparison',
    artifact: { ...left.artifact }, manifestSha256: left.manifestSha256,
    producer: { ...left.original.origin },
    receivers: [left, right].map(receipt => ({
      host: { ...receipt.receiver },
      node: { goldenCases: receipt.node.goldenCases, checksum: receipt.node.checksum,
        configuration: { ...receipt.node.configuration } },
      wasmtime: { status: receipt.wasmtime.status,
        goldenCases: receipt.wasmtime.goldenCases ?? null },
    })),
    interpretation: 'matching recorded artifact and producer only; no attestation or cross-host performance ranking',
  };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    requireValid(process.argv.length === 4, 'usage: compare-portable-replays.mjs LEFT.json RIGHT.json');
    const files = await Promise.all(process.argv.slice(2).map(async path => {
      const { stat } = await import('node:fs/promises');
      requireValid((await stat(path)).size <= 1024 * 1024, 'receipt file size');
      return JSON.parse(await readFile(path, 'utf8'));
    }));
    process.stdout.write(JSON.stringify(comparePortableReplays(...files), null, 2) + '\n');
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
