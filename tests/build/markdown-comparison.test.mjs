import assert from 'node:assert/strict';
import { execFile, spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { arch, platform, tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';
import { bindMarkdownPair, compareMarkdownPair } from '../../tools/bench/compare-markdown.mjs';

const osMap = { linux: 'linux', darwin: 'macos', win32: 'windows' };
const archMap = { x64: 'x86_64', arm64: 'aarch64' };
const harnessPaths = [
  'tools/bench/capture-markdown.mjs',
  'crates/grunkalunka-runtime/examples/markdown_cost.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/embedding.rs',
  'crates/grunkalunka-runtime/tests/fixtures/document-outline/inspector.wat.in',
  'Cargo.lock',
];
const summary = values => {
  const sorted = [...values].sort((a, b) => a - b);
  return { samplesMs: [...values], minMs: sorted[0], medianMs: sorted[0] / 2 + sorted[1] / 2,
    maxMs: sorted[1] };
};
function receipt(at = '2026-10-27T17:00:00.000Z') {
  const samples = 2; const iterations = 2; const warmup = 1;
  const repeated = warmup + samples * iterations;
  return {
    schemaVersion: 1, workload: 'markdown-wit-capture', observedAt: at,
    cachePolicy: 'OS and Wasmtime caches uncontrolled; one compilation, then warm samples',
    provenance: {
      gitCommit: '1'.repeat(40), trackedChangesPresent: false, buildLabel: 'debug',
      guest: { sha256: '2'.repeat(64), bytes: 100 }, runner: { sha256: '3'.repeat(64) },
      harnessSha256: Object.fromEntries(harnessPaths.map((name, index) =>
        [name, String(index + 4).repeat(64)])),
      host: { os: platform(), arch: arch(), osRelease: 'test-release',
        cpuModel: 'specific machine model', cpuCount: 4, node: 'v24.0.0', v8: '13.0',
        nodeOptions: '', observedRustc: 'rustc 1.90.0 (aabbcc)' },
    },
    report: {
      schemaVersion: 1, workload: 'markdown-wit-inspection',
      configuration: { case: 'unicode', samples, iterations, warmup },
      environment: { os: osMap[platform()], arch: archMap[arch()], fuelEnabled: false,
        wasmtimeDependency: '=50.0.0' },
      artifact: { guestBytes: 100, componentBytes: 400 },
      timings: { unit: 'milliseconds', order: 'alternating', embedMs: 2,
        componentCompileMs: 3, instantiation: summary([1, 3]),
        granted: summary([4, 6]), denied: summary([0, 0]) },
      outcomes: { grantedBackingReads: repeated + 1, deniedBackingReads: 0,
        grantedCleanups: repeated + 2, deniedCleanups: repeated + 1 },
      fixture: { sourceHex: Buffer.from('# Root\n### 子😀\n').toString('hex'),
        outlineHex: Buffer.from('native projection').toString('hex') },
    },
  };
}
function change(object, path, value) {
  const parts = path.replace('harnessSha256.Cargo.lock', 'harnessSha256.Cargo__lock').split('.');
  const name = parts.pop()?.replace('Cargo__lock', 'Cargo.lock');
  let current = object;
  for (const part of parts) current = current[part];
  if (value === undefined) delete current[name]; else current[name] = value;
}
function validPair() {
  return bindMarkdownPair(receipt(), receipt('2026-10-27T17:00:01.000Z'));
}

test('only a same-process bound, same-binary pair exposes samples and unranked deltas', () => {
  const pair = validPair();
  pair.captures[1].report.timings.granted = summary([50, 60]);
  assert.throws(() => compareMarkdownPair(pair), /binding.candidateSha256/,
    'altering bound bytes must fail even if a sample remains finite');
  const before = receipt(); const after = receipt('2026-10-27T17:00:01.000Z');
  after.report.timings.granted = summary([5, 9]);
  const bound = bindMarkdownPair(before, after);
  const original = structuredClone(bound);
  const compared = compareMarkdownPair(bound);
  assert.deepEqual(compared.series.granted, {
    baselineSamplesMs: [4, 6], candidateSamplesMs: [5, 9],
    median: { baselineMs: 5, candidateMs: 7, deltaMs: 2 },
  });
  assert.equal(compared.series.denied.median.deltaMs, 0);
  assert.equal(compared.embed.deltaMs, 0);
  assert.equal(compared.scope.includes('not speed rankings'), true);
  assert.equal(Object.hasOwn(compared.embed, 'ratio'), false);
  compared.series.granted.baselineSamplesMs[0] = 100;
  assert.deepEqual(bound, original, 'comparison cannot mutate or alias saved receipts');
});

test('changed binary, harness, workload, fixtures or actual host conditions reject', () => {
  for (const [path, value] of [
    ['provenance.guest.sha256', 'a'.repeat(64)],
    ['provenance.runner.sha256', 'b'.repeat(64)],
    ['provenance.harnessSha256.Cargo.lock', 'c'.repeat(64)],
    ['provenance.gitCommit', 'd'.repeat(40)],
    ['provenance.buildLabel', 'release'],
    ['provenance.host.cpuModel', 'identical arch, different CPU'],
    ['provenance.host.osRelease', 'other release'],
    ['provenance.host.cpuCount', 8],
    ['provenance.host.nodeOptions', '--jitless'],
    ['provenance.host.observedRustc', 'rustc 1.91.0 (abcdef)'],
    ['report.configuration.case', 'wide'],
    ['report.configuration.iterations', 10],
    ['report.fixture.sourceHex', '23'],
    ['report.fixture.outlineHex', 'aa'],
    ['report.artifact.componentBytes', 500],
    ['report.environment.wasmtimeDependency', '=51.0.0'],
  ]) {
    const before = receipt(); const after = receipt('2026-10-27T17:00:01.000Z');
    change(after, path, value);
    assert.throws(() => bindMarkdownPair(before, after), undefined, path);
  }
});

test('a saved pair cannot omit or invent provenance on both sides', () => {
  for (const path of [
    'provenance.guest.sha256', 'provenance.runner.sha256',
    'provenance.harnessSha256', 'provenance.host.nodeOptions',
    'provenance.host.cpuModel', 'provenance.host.observedRustc',
    'report.environment.fuelEnabled', 'report.configuration.samples',
    'report.fixture.sourceHex', 'report.timings.granted.medianMs',
  ]) {
    const before = receipt(); const after = receipt('2026-10-27T17:00:01.000Z');
    change(before, path, undefined); change(after, path, undefined);
    assert.throws(() => bindMarkdownPair(before, after), undefined, path);
  }
  for (const [path, value] of [
    ['provenance.trackedChangesPresent', true],
    ['provenance.buildLabel', 'unspecified'],
    ['provenance.host.cpuModel', null],
    ['provenance.host.observedRustc', null],
    ['provenance.guest.sha256', 'not-a-sha'],
    ['provenance.harnessSha256.Cargo.lock', 'fake'],
    ['report.outcomes.deniedBackingReads', 1],
    ['report.outcomes.grantedCleanups', 99],
    ['report.timings.granted.medianMs', 300],
    ['report.timings.denied.samplesMs', [0, -1]],
    ['report.fixture.sourceHex', 'ff'],
    ['report.artifact.guestBytes', 99],
  ]) {
    const before = receipt(); const after = receipt('2026-10-27T17:00:01.000Z');
    change(before, path, value); change(after, path, value);
    assert.throws(() => bindMarkdownPair(before, after), undefined, path);
  }
});

test('forged or incomplete session binding and reversed timestamps fail closed', () => {
  for (const [path, value] of [
    ['schemaVersion', 2], ['workload', 'numeric-gcd'],
    ['binding.method', 'operator-supplied'], ['binding.sessionId', 'not-uuid'],
    ['binding.baselineSha256', '0'.repeat(64)], ['binding.candidateSha256', undefined],
    ['captures', [receipt()]],
  ]) {
    const pair = validPair(); change(pair, path, value);
    assert.throws(() => compareMarkdownPair(pair), undefined, path);
  }
  assert.throws(() => bindMarkdownPair(receipt('2026-10-27T17:00:01.000Z'), receipt()),
    /capture order/);
});

test('CLI accepts saved Unicode pathname/stdin and emits nothing on invalid input', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'markdown comparison é '));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const script = fileURLToPath(new URL('../../tools/bench/compare-markdown.mjs', import.meta.url));
  const pair = validPair(); const file = join(dir, 'pair 猫.json');
  await writeFile(file, JSON.stringify(pair));
  const exec = args => promisify(execFile)(process.execPath, [script, ...args],
    { cwd: dir, timeout: 5000, killSignal: 'SIGKILL', maxBuffer: 65536 });
  const stdout = (await exec([file])).stdout;
  assert.equal(JSON.parse(stdout).series.granted.median.deltaMs, 0);
  const stdin = spawnSync(process.execPath, [script, '-'], { cwd: dir,
    input: JSON.stringify(pair), encoding: 'utf8', timeout: 5000, shell: false });
  assert.equal(stdin.status, 0, stdin.stderr);
  assert.equal(JSON.parse(stdin.stdout).series.granted.median.deltaMs, 0);
  await writeFile(file, '{');
  for (const args of [[], [file], [file, file], ['missing.json'], ['--capture']]) {
    await assert.rejects(exec(args), error => {
      assert.equal(error.code, 1); assert.equal(error.stdout, '');
      assert.ok(error.stderr.length > 0); return true;
    });
  }
  assert.equal(await readFile(file, 'utf8'), '{');
});

function savedMemory(os) {
  if (os !== 'linux') {
    return { scope: 'entire-rust-process', unit: 'bytes',
      source: 'unsupported-on-this-host', snapshots: null };
  }
  const names = [
    'beforeEmbed', 'afterCompile', 'afterInstantiation', 'afterWarmup', 'afterBatches',
  ];
  return {
    scope: 'entire-rust-process', unit: 'bytes', source: 'linux-proc-status',
    snapshots: Object.fromEntries(names.map((name, index) => [
      name, { rssBytes: 1024 + index * 128, peakRssBytes: 2048 + index * 128 },
    ])),
  };
}

test('saved native memory is checked independently of the same-process digest binding', () => {
  // Missing memory is legitimate for historical receipts.
  const old = validPair();
  assert.doesNotThrow(() => compareMarkdownPair(old));
  const before = receipt();
  const after = receipt('2026-10-27T17:00:01.000Z');
  before.report.processMemory = savedMemory(before.report.environment.os);
  after.report.processMemory = savedMemory(after.report.environment.os);
  assert.doesNotThrow(() => bindMarkdownPair(before, after));
  // A forged but newly self-consistently rebound pair must not pass validation.
  after.report.processMemory.unit = 'pages';
  assert.throws(() => bindMarkdownPair(before, after), /processMemory scope\/units/);
  after.report.processMemory.unit = 'bytes';
  if (after.report.environment.os === 'linux') {
    after.report.processMemory.snapshots.afterWarmup.peakRssBytes = 0;
  } else {
    after.report.processMemory.snapshots = {};
  }
  assert.throws(() => bindMarkdownPair(before, after), /Invalid/);
});
