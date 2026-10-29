import assert from 'node:assert/strict';
import { test } from 'node:test';
import { arch, platform } from 'node:os';
import { resolve } from 'node:path';
import {
  parseCaptureArgs, validateNativeReport, validateOptionalProcessMemory,
} from '../../tools/bench/capture-markdown.mjs';

const rustOS = { linux: 'linux', darwin: 'macos', win32: 'windows' };
const rustArch = { x64: 'x86_64', arm64: 'aarch64' };

function series(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return {
    samplesMs: samples,
    minMs: sorted[0],
    medianMs: sorted.length % 2 ? sorted[middle] : sorted[middle - 1] / 2 + sorted[middle] / 2,
    maxMs: sorted.at(-1),
  };
}

const config = { case: 'unicode', samples: 3, iterations: 4, warmup: 2 };
function report() {
  const repeated = config.warmup + config.samples * config.iterations;
  return {
    schemaVersion: 1, workload: 'markdown-wit-inspection',
    configuration: { ...config },
    environment: { os: rustOS[platform()], arch: rustArch[arch()],
      fuelEnabled: false, wasmtimeDependency: '=50.0.0' },
    artifact: { guestBytes: 100, componentBytes: 256 },
    timings: { unit: 'milliseconds', order: 'alternating', embedMs: 1, componentCompileMs: 2,
      instantiation: series([1, 3, 2]), granted: series([4, 6, 5]),
      denied: series([0, 0, 0]) },
    outcomes: { grantedBackingReads: repeated + 1, deniedBackingReads: 0,
      grantedCleanups: repeated + 2, deniedCleanups: repeated + 1 },
    fixture: { sourceHex: Buffer.from('# Root\r\n### 子😀\n').toString('hex'),
      outlineHex: Buffer.from('1 0 8 8 -1 4 526f6f74\n').toString('hex') },
  };
}

function changed(path, value) {
  const candidate = report();
  const parts = path.split('.');
  const key = parts.pop();
  let parent = candidate;
  for (const segment of parts) parent = parent[segment];
  if (value === undefined) delete parent[key];
  else parent[key] = value;
  return candidate;
}

test('capture CLI constrains work and preserves explicitly named build provenance', () => {
  const parsed = parseCaptureArgs(['my native runner', 'guest 猫.wasm',
    '--workload', 'wide', '--samples', '2', '--iterations', '3', '--warmup', '0',
    '--build-label', 'release-lto']);
  assert.equal(parsed.runner, resolve('my native runner'));
  assert.equal(parsed.guest, resolve('guest 猫.wasm'));
  assert.equal(parsed.buildLabel, 'release-lto');
  assert.deepEqual(parsed.config, { case: 'wide', samples: 2, iterations: 3, warmup: 0 });
  assert.deepEqual(parseCaptureArgs(['runner', 'guest']).config,
    { case: 'unicode', samples: 5, iterations: 20, warmup: 5 });
});

test('invalid CLI options fail before launching a native program', () => {
  for (const args of [
    [], ['runner'], ['runner', 'guest', '--samples'],
    ['runner', 'guest', '--samples', '1'],
    ['runner', 'guest', '--samples', '13'],
    ['runner', 'guest', '--iterations', '0'],
    ['runner', 'guest', '--iterations', '101'],
    ['runner', 'guest', '--warmup', '101'],
    ['runner', 'guest', '--workload', 'invalid'],
    ['runner', 'guest', '--build-label', 'bad label'],
    ['runner', 'guest', '--samples', '2', '--samples', '3'],
    ['runner', 'guest', '--other', '1'],
    ['runner', 'guest', '--iterations', 'NaN'],
    ['runner', 'guest', '--iterations', '2.5'],
    ['runner', 'guest', '--iterations', '2\0bad'],
  ]) {
    assert.throws(() => parseCaptureArgs(args), undefined, args.join(' '));
  }
});

test('valid native WIT report retains raw samples and rejects no legitimate zero', () => {
  const input = report();
  const before = structuredClone(input);
  assert.equal(validateNativeReport(input, config, 100), input);
  assert.deepEqual(input, before);
  assert.equal(input.timings.denied.minMs, 0);
  const rounded = report();
  rounded.timings.granted.samplesMs = [1.000000001, 2.000000001, 3.000000001];
  rounded.timings.granted.minMs = 1;
  rounded.timings.granted.medianMs = 2;
  rounded.timings.granted.maxMs = 3;
  assert.doesNotThrow(() => validateNativeReport(rounded, config, 100));
});

test('different workload, backing-read/cleanup behavior or host policy cannot be passed off', () => {
  for (const [path, value] of [
    ['schemaVersion', 2], ['workload', 'wrong'],
    ['configuration.case', 'wide'], ['configuration.iterations', 5],
    ['environment.os', 'unknown'], ['environment.arch', 'unknown'],
    ['environment.fuelEnabled', true], ['environment.wasmtimeDependency', 'unknown'],
    ['artifact.guestBytes', 101], ['artifact.componentBytes', 99],
    ['outcomes.grantedBackingReads', 3], ['outcomes.deniedBackingReads', 1],
    ['outcomes.grantedCleanups', 15], ['outcomes.deniedCleanups', 0],
    ['timings.unit', 'seconds'], ['timings.order', 'uncontrolled'],
  ]) {
    assert.throws(() => validateNativeReport(changed(path, value), config, 100),
      undefined, path);
  }
});

test('invalid, missing or fabricated timing series and fixture bytes are rejected', () => {
  for (const [path, value] of [
    ['timings.embedMs', -1], ['timings.componentCompileMs', Infinity],
    ['timings.granted.samplesMs', [1, 2]],
    ['timings.granted.samplesMs', [1, NaN, 3]],
    ['timings.granted.samplesMs', [1, '2', 3]],
    ['timings.granted.samplesMs', new Array(3)],
    ['timings.granted.medianMs', 900],
    ['timings.denied.maxMs', 1],
    ['timings.instantiation', undefined],
    ['fixture.sourceHex', 'zz'],
    ['fixture.sourceHex', 'ff'],
    ['fixture.outlineHex', 'bad'],
    ['fixture.outlineHex', 'aa'.repeat(32769)],
  ]) {
    assert.throws(() => validateNativeReport(changed(path, value), config, 100),
      undefined, path);
  }
});

function memoryEnvelope(os) {
  if (os === 'macos') {
    return {
      scope: 'entire-rust-process', unit: 'bytes',
      source: 'unsupported-on-this-host', snapshots: null,
    };
  }
  const names = [
    'beforeEmbed', 'afterCompile', 'afterInstantiation',
    'afterWarmup', 'afterBatches',
  ];
  return {
    scope: 'entire-rust-process', unit: 'bytes',
    source: os === 'linux' ? 'linux-proc-status' : 'windows-psapi-working-set',
    snapshots: Object.fromEntries(names.map((name, index) => [
      name, { rssBytes: 1024 + index * 256, peakRssBytes: 2048 + index * 256 },
    ])),
  };
}

test('optional real Markdown memory is validated without invalidating historical receipts', () => {
  const legacy = report();
  assert.doesNotThrow(() => validateNativeReport(legacy, config, 100));
  const modern = report();
  modern.processMemory = memoryEnvelope(modern.environment.os);
  assert.doesNotThrow(() => validateNativeReport(modern, config, 100));
  assert.doesNotThrow(() =>
    validateOptionalProcessMemory(modern, modern.environment.os));
  assert.deepEqual(report(), legacy, 'historical data remains unmodified');
});

test('claimed Markdown memory must be complete and platform-specific', () => {
  const os = rustOS[platform()];
  const values = [
    null,
    { ...memoryEnvelope(os), unit: 'KiB' },
    { ...memoryEnvelope(os), source: 'invented' },
    { ...memoryEnvelope(os), snapshots: {} },
  ];
  if (os !== 'macos') {
    for (const mutate of [
      x => { delete x.snapshots.beforeEmbed; },
      x => { x.snapshots.afterCompile.rssBytes = -1; },
      x => { x.snapshots.afterWarmup.peakRssBytes = 1; },
      x => { x.snapshots.afterBatches.rssBytes = 2 ** 54; },
      x => { x.snapshots.beforeEmbed.unverified = true; },
    ]) {
      const value = memoryEnvelope(os);
      mutate(value);
      values.push(value);
    }
  } else {
    values.push({
      ...memoryEnvelope(os), snapshots: { beforeEmbed: { rssBytes: 0, peakRssBytes: 0 } },
    });
  }
  for (const value of values) {
    const candidate = report();
    candidate.processMemory = value;
    assert.throws(() => validateNativeReport(candidate, config, 100),
      /Invalid/, 'forged or incomplete process memory must fail');
  }
});

test('historical Windows unsupported data and new measured Windows data both validate', () => {
  const old = report();
  old.environment.os = 'windows';
  old.processMemory = {
    scope: 'entire-rust-process', unit: 'bytes',
    source: 'unsupported-on-this-host', snapshots: null,
  };
  assert.doesNotThrow(() => validateOptionalProcessMemory(old, 'windows'));
  const measured = report();
  measured.environment.os = 'windows';
  measured.processMemory = memoryEnvelope('windows');
  assert.doesNotThrow(() => validateOptionalProcessMemory(measured, 'windows'));
  measured.processMemory.source = 'linux-proc-status';
  assert.throws(() => validateOptionalProcessMemory(measured, 'windows'), /Invalid/);
  const fakeMac = report();
  fakeMac.processMemory = memoryEnvelope('windows');
  assert.throws(() => validateOptionalProcessMemory(fakeMac, 'macos'), /Invalid/);
});

test('macOS Mach memory receipts accept complete native readings and historic absence', () => {
  const legacy = report();
  legacy.environment.os = 'macos';
  legacy.processMemory = memoryEnvelope('macos');
  assert.doesNotThrow(() => validateOptionalProcessMemory(legacy, 'macos'));
  const measured = report();
  measured.environment.os = 'macos';
  measured.processMemory = memoryEnvelope('linux');
  measured.processMemory.source = 'macos-mach-task-basic-info';
  assert.doesNotThrow(() => validateOptionalProcessMemory(measured, 'macos'));
  // The full runner validator intentionally binds to the executing host.
  // Cross-host archived receipt checks must use the OS-aware saved parser.
  if (platform() === 'darwin') {
    assert.doesNotThrow(() => validateNativeReport(measured, config, 100));
  }

  for (const corrupt of [
    x => { x.source = 'linux-proc-status'; },
    x => { x.source = 'windows-psapi-working-set'; },
    x => { x.source = 'unsupported-on-this-host'; },
    x => { x.snapshots.beforeEmbed.rssBytes = -1; },
    x => { x.snapshots.afterWarmup.peakRssBytes = 1; },
    x => { delete x.snapshots.afterCompile; },
    x => { x.snapshots.afterBatches.peakRssBytes = 2 ** 54; },
    x => { x.snapshots.beforeEmbed.unverified = true; },
  ]) {
    const candidate = report();
    candidate.environment.os = 'macos';
    candidate.processMemory = structuredClone(measured.processMemory);
    corrupt(candidate.processMemory);
    assert.throws(() => validateOptionalProcessMemory(candidate, 'macos'), /Invalid/);
  }
  // Neither Linux nor Windows can relabel a captured Mach task observation.
  assert.throws(() => validateOptionalProcessMemory(measured, 'linux'), /Invalid/);
  assert.throws(() => validateOptionalProcessMemory(measured, 'windows'), /Invalid/);
});

test('fresh WIT lifecycle observations preserve legacy receipts and reject forged series', () => {
  const native = report();
  assert.doesNotThrow(() => validateNativeReport(native, config, 100));
  const lifecycle = {
    count: config.samples, unit: 'milliseconds per fresh component lifecycle',
    granted: series([1, 2, 3]), denied: series([4, 5, 6]),
  };
  native.freshLifecycles = lifecycle;
  assert.doesNotThrow(() => validateNativeReport(native, config, 100));
  for (const bad of [
    { ...lifecycle, count: 4 },
    { ...lifecycle, unit: 'microseconds' },
    { ...lifecycle, granted: series([1, 2]) },
    { ...lifecycle, denied: { ...series([4, 5, 6]), medianMs: 99 } },
    { ...lifecycle, invented: 42 },
    { ...lifecycle, granted: { ...series([1, 2, 3]), samplesMs: [-1, 2, 3] } },
  ]) {
    native.freshLifecycles = bad;
    assert.throws(() => validateNativeReport(native, config, 100), /Invalid/);
  }
});

test('native artifact digests must be complete and match the independently hashed guest', () => {
  const hash = 'a'.repeat(64);
  const withDigests = report();
  withDigests.artifact.guestSha256 = hash;
  withDigests.artifact.componentSha256 = 'b'.repeat(64);
  assert.equal(validateNativeReport(withDigests, config, 100, hash), withDigests);
  assert.throws(() => validateNativeReport(withDigests, config, 100, 'c'.repeat(64)),
    /Invalid artifact guest digest identity/);
  assert.throws(() => validateNativeReport(report(), config, 100, hash),
    /Invalid required independent guest digest/);
  for (const artifact of [
    { ...withDigests.artifact, guestSha256: 'short' },
    { ...withDigests.artifact, componentSha256: 'B'.repeat(64) },
    { ...withDigests.artifact, componentSha256: undefined },
  ]) {
    assert.throws(() => validateNativeReport({ ...withDigests, artifact }, config, 100, hash),
      /Invalid artifact digest syntax/);
  }
  const missing = { ...withDigests.artifact };
  delete missing.componentSha256;
  assert.throws(() => validateNativeReport({ ...withDigests, artifact: missing }, config, 100),
    /Invalid complete artifact digest pair/);
  // Pre-hash archived reports still pass when no external guest identity is supplied.
  assert.doesNotThrow(() => validateNativeReport(report(), config, 100));
});

test('subprocess wall-cost validity is distinct from the native internal timer', () => {
  const input = report();
  assert.equal(validateNativeReport(input, config, 100), input,
    'runner report remains a separate internal timing contract');
  // The outer timing belongs to the capture wrapper, not inside a forged
  // Wasmtime timing array. Legacy runner reports remain unchanged.
  assert.equal(Object.hasOwn(input, 'nativeProcessElapsedMs'), false);
});
