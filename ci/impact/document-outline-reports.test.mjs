// Fast checks of the actual report/orchestration decisions. Only subprocess
// responses are synthetic; artifact files, hashing, parsing and validation run.
// Native document behavior remains covered by the separate real feature check.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { syncBuiltinESMExports } from 'node:module';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const script = new URL('../../scripts/check-document-outline.mjs', import.meta.url);
const guestBytes = Buffer.from('synthetic guest identity; no Wasm execution in this unit');
const guestSha256 = createHash('sha256').update(guestBytes).digest('hex');
const componentSha256 = 'c'.repeat(64), runnerSha256 = 'a'.repeat(64);
const phases = ['beforeEmbed', 'afterCompile', 'afterInstantiation', 'afterWarmup', 'afterBatches'];
const sources = ['empty', 'unicode', 'dense', 'wide'];
const hex = text => Buffer.from(text).toString('hex');
const fixtures = Array.from({ length: 9 }, (_, index) => ({
  source: `# Fixture ${index} 子😀\n`, outline: `synthetic outline ${index}\n`,
}));
const costFixtures = Object.fromEntries(sources.map(name => [name, {
  source: `# Cost ${name} 子😀\n`, outline: `synthetic cost outline ${name}\n`,
}]));
const nativeOutlines = new Map([...fixtures, ...Object.values(costFixtures)]
  .map(({ source, outline }) => [source, outline]));
const memorySources = {
  linux: 'linux-proc-status', win32: 'windows-psapi-working-set', darwin: 'macos-mach-task-basic-info',
};
const series = () => ({ samplesMs: [1, 3], minMs: 1, medianMs: 2, maxMs: 3 });
const configuration = () => ({ case: 'unicode', samples: 2, iterations: 2, warmup: 1 });

function report(selected, platform) {
  const fixture = costFixtures[selected];
  assert.ok(fixture, 'the fake only implements declared cost fixtures');
  return {
    schemaVersion: 1, workload: 'markdown-wit-inspection',
    configuration: { ...configuration(), case: selected },
    environment: { fixture: 'independent-report-contract' },
    artifact: { guestBytes: guestBytes.length, guestSha256,
      componentBytes: guestBytes.length + 32, componentSha256 },
    processMemory: { scope: 'entire-rust-process', unit: 'bytes',
      source: memorySources[platform] ?? 'unsupported-on-this-host',
      snapshots: memorySources[platform] ? Object.fromEntries(phases.map((phase, index) => [phase,
        { rssBytes: 10 + index, peakRssBytes: 20 + index }])) : null },
    outcomes: { grantedBackingReads: 6, deniedBackingReads: 0, grantedCleanups: 7, deniedCleanups: 6 },
    fixture: { sourceHex: hex(fixture.source), outlineHex: hex(fixture.outline) },
    freshLifecycles: { count: 2, unit: 'milliseconds per fresh component lifecycle',
      granted: series(), denied: series() },
    timings: { unit: 'milliseconds', order: 'alternating',
      instantiation: series(), granted: series(), denied: series(), nativeOutline: series(),
      embedMs: 1, componentCompileMs: 2 },
  };
}

function capture(platform, elapsed = 12) {
  return {
    schemaVersion: 1, workload: 'markdown-wit-capture', nativeProcessElapsedMs: elapsed,
    provenance: { guest: { bytes: guestBytes.length, sha256: guestSha256 },
      runner: { sha256: runnerSha256 }, gitCommit: 'b'.repeat(40),
      buildLabel: 'ci-debug', trackedChangesPresent: false,
      harnessSha256: { 'tools/bench/capture-markdown.mjs': 'd'.repeat(64) } },
    report: report('unicode', platform),
  };
}

const comparisonSeries = () => ({ baselineSamplesMs: [1, 3], candidateSamplesMs: [1, 3],
  median: { deltaMs: 0 } });
function comparison() {
  return {
    workload: 'markdown-wit-repeat-comparison', guestSha256, runnerSha256,
    configuration: configuration(),
    nativeProcessElapsedMs: { baselineMs: 12, candidateMs: 18, deltaMs: 6 },
    freshLifecycles: { count: 2, unit: 'milliseconds per fresh component lifecycle',
      granted: comparisonSeries(), denied: comparisonSeries() },
    series: Object.fromEntries(['instantiation', 'granted', 'denied', 'nativeOutline']
      .map(name => [name, comparisonSeries()])),
  };
}

let invocation = 0;
async function invoke(t, { platform = process.platform, mutate = () => {}, resultFor = () => null } = {}) {
  const target = mkdtempSync(join(tmpdir(), 'grunkalunka-outline-reports-'));
  const wasm = join(target, 'wasm32-unknown-unknown', 'debug', 'examples', 'outline_guest.wasm');
  mkdirSync(join(target, 'wasm32-unknown-unknown', 'debug', 'examples'), { recursive: true });
  writeFileSync(wasm, guestBytes);
  const calls = [], messages = [], costCalls = new Map();
  const previousArgs = process.argv, previousTarget = process.env.CARGO_TARGET_DIR;
  const previousPlatform = Object.getOwnPropertyDescriptor(process, 'platform');
  let fakeError, error;
  t.mock.method(childProcess, 'spawnSync', (command, args, options) => {
    try {
      const name = basename(command).replace(/\.exe$/, '');
      let phase, payload;
      if (command === 'cargo') phase = 'build';
      else if (name === 'document_outline') {
        phase = 'component';
        payload = fixtures.map(({ source, outline }) => `${hex(source)}\t${hex(outline)}`).join('\n') + '\n';
      } else if (name === 'outline_guest') {
        phase = 'native';
        payload = nativeOutlines.get(options.input.toString());
        assert.notEqual(payload, undefined, 'the native fake cannot infer answers for an unexpected input');
      } else if (name === 'markdown_cost') {
        phase = 'cost';
        const selected = args[args.indexOf('--workload') + 1];
        costCalls.set(selected, (costCalls.get(selected) ?? 0) + 1);
        payload = report(selected, platform);
      } else if (command === process.execPath && args[0] === '--test') {
        phase = 'nested-contract';
      } else if (command === process.execPath && args[0] === 'tools/bench/capture-markdown.mjs') {
        phase = 'capture'; payload = capture(platform);
      } else if (command === process.execPath && args[0] === 'tools/bench/compare-markdown.mjs') {
        if (args[1] === '--capture') {
          phase = 'pair';
          payload = { workload: 'markdown-wit-same-process-pair',
            binding: { method: 'same-node-process-sequential' },
            captures: [capture(platform, 12), capture(platform, 18)] };
        } else {
          assert.deepEqual(args, ['tools/bench/compare-markdown.mjs', '-']);
          phase = 'comparison'; payload = comparison();
          assert.equal(JSON.parse(options.input).captures.length, 2,
            'the real checker serializes the observed pair into the comparison process');
        }
      } else throw new Error(`unexpected subprocess ${command} ${args.join(' ')}`);
      const call = { phase, command, args, options, payload,
        repeat: phase === 'cost' ? costCalls.get(args[args.indexOf('--workload') + 1]) : 1, wasm };
      calls.push(call);
      mutate(call);
      const override = resultFor(call);
      if (override) return override;
      return { status: 0, signal: null, stderr: '',
        stdout: typeof call.payload === 'string' ? call.payload :
          call.payload === undefined ? '' : JSON.stringify(call.payload) };
    } catch (caught) {
      fakeError ??= caught;
      throw caught;
    }
  });
  t.mock.method(console, 'log', message => messages.push(message));
  syncBuiltinESMExports();
  try {
    process.argv = [process.execPath, fileURLToPath(script)];
    process.env.CARGO_TARGET_DIR = target;
    Object.defineProperty(process, 'platform', { ...previousPlatform, value: platform });
    await import(`${script.href}?report-test=${++invocation}`);
  } catch (caught) {
    error = caught;
  } finally {
    process.argv = previousArgs;
    if (previousTarget === undefined) delete process.env.CARGO_TARGET_DIR;
    else process.env.CARGO_TARGET_DIR = previousTarget;
    Object.defineProperty(process, 'platform', previousPlatform);
    t.mock.restoreAll();
    syncBuiltinESMExports();
    rmSync(target, { recursive: true, force: true });
  }
  // A broken stand-in or mutation must fail this unit, never satisfy a test that
  // expects the production validator to reject a malformed external response.
  assert.ifError(fakeError);
  return { calls, messages, error };
}

for (const platform of ['linux', 'win32', 'darwin', 'freebsd']) {
  test(`full document report validation accepts coherent ${platform} receipts without native processes`, async t => {
    const { calls, messages, error } = await invoke(t, { platform });
    assert.ifError(error);
    assert.deepEqual(calls.filter(call => call.phase === 'cost')
      .map(call => call.payload.configuration.case), ['empty', 'unicode', 'dense', 'wide', 'unicode']);
    assert.equal(calls.filter(call => call.phase === 'native').length, 14);
    assert.equal(calls.filter(call => call.phase === 'capture').length, 1);
    assert.equal(calls.filter(call => call.phase === 'pair').length, 1);
    assert.equal(calls.at(-1).phase, 'comparison');
    assert.equal(messages.length, 1);
    assert.match(messages[0], /4 cost fixtures with native parity; repeated Unicode report identity/);
  });
}

const malformedCosts = [
  ['unsupported schema', r => { r.schemaVersion = 2; }],
  ['wrong fixture selection', r => { r.configuration.case = 'unicode'; }],
  ['unrequested sample count', r => { r.configuration.samples = 3; }],
  ['wrong timing unit', r => { r.timings.unit = 'seconds'; }],
  ['different guest digest', r => { r.artifact.guestSha256 = '0'.repeat(64); }],
  ['invalid component digest', r => { r.artifact.componentSha256 = 'unknown'; }],
  ['component smaller than guest', r => { r.artifact.componentBytes = r.artifact.guestBytes; }],
  ['wrong memory source', r => { r.processMemory.source = 'unknown'; }],
  ['missing memory phase', r => { delete r.processMemory.snapshots.afterCompile; }],
  ['fractional resident bytes', r => { r.processMemory.snapshots.beforeEmbed.rssBytes = 0.5; }],
  ['resident bytes above peak', r => { r.processMemory.snapshots.beforeEmbed.rssBytes = 21; }],
  ['decreasing peak bytes', r => { r.processMemory.snapshots.afterCompile.peakRssBytes = 19; }],
  ['denied backing read', r => { r.outcomes.deniedBackingReads = 1; }],
  ['missing owned cleanup', r => { r.outcomes.grantedCleanups = 6; }],
  ['malformed fixture bytes', r => { r.fixture.outlineHex = '0'; }],
  ['different native outline', r => { r.fixture.outlineHex = hex('wrong outline'); }],
  ['missing timing sample', r => { r.timings.granted.samplesMs = [1]; }],
  ['negative lifecycle duration', r => { r.freshLifecycles.denied.samplesMs[0] = -1; }],
  ['unordered timing summary', r => { r.timings.denied.medianMs = 4; }],
  ['negative compilation duration', r => { r.timings.componentCompileMs = -1; }],
];
for (const [name, corrupt] of malformedCosts) {
  test(`document cost receipts reject ${name}`, async t => {
    const { calls, messages, error } = await invoke(t, { platform: 'linux', mutate(call) {
      if (call.phase === 'cost') corrupt(call.payload);
    } });
    assert.equal(error?.code, 'ERR_ASSERTION', error?.stack);
    assert.equal(calls.filter(call => call.phase === 'cost').length, 1);
    assert.ok(calls.every(call => !['capture', 'pair', 'comparison'].includes(call.phase)));
    assert.deepEqual(messages, []);
  });
}

test('repeated document reports must retain their execution environment', async t => {
  const { error, messages } = await invoke(t, { mutate(call) {
    if (call.phase === 'cost' && call.repeat === 2) call.payload.environment.fixture = 'different host';
  } });
  assert.match(error?.message ?? '', /separate Unicode report must repeat environment/);
  assert.deepEqual(messages, []);
});

test('the actual guest file must not change while collecting document reports', async t => {
  const { error, messages } = await invoke(t, { mutate(call) {
    if (call.phase === 'cost' && call.repeat === 2) writeFileSync(call.wasm, 'changed guest bytes');
  } });
  assert.match(error?.message ?? '', /Markdown guest binary changed across report collection/);
  assert.deepEqual(messages, []);
});

const malformedLater = [
  ['capture', 'negative wall time', r => { r.nativeProcessElapsedMs = -1; }],
  ['capture', 'wrong guest byte count', r => { r.provenance.guest.bytes++; }],
  ['capture', 'missing harness identity', r => { r.provenance.harnessSha256['tools/bench/capture-markdown.mjs'] = 'unknown'; }],
  ['capture', 'different captured fixture', r => { r.report.fixture.outlineHex = hex('wrong outline'); }],
  ['pair', 'missing capture', r => { r.captures.pop(); }],
  ['pair', 'different component identity', r => { r.captures[1].report.artifact.componentSha256 = 'e'.repeat(64); }],
  ['pair', 'different runner identity', r => { r.captures[1].provenance.runner.sha256 = 'e'.repeat(64); }],
  ['pair', 'uncommitted paired source', r => { r.captures[1].provenance.trackedChangesPresent = true; }],
  ['comparison', 'incorrect wall-time delta', r => { r.nativeProcessElapsedMs.deltaMs = 7; }],
  ['comparison', 'different lifecycle samples', r => { r.freshLifecycles.granted.candidateSamplesMs = [1, 4]; }],
  ['comparison', 'missing timing samples', r => { r.series.granted.baselineSamplesMs.pop(); }],
  ['comparison', 'nonnumeric timing delta', r => { r.series.granted.median.deltaMs = null; }],
];
for (const [phase, name, corrupt] of malformedLater) {
  test(`document ${phase} validation rejects ${name}`, async t => {
    const { calls, error, messages } = await invoke(t, { mutate(call) {
      if (call.phase === phase) corrupt(call.payload);
    } });
    assert.ok(calls.some(call => call.phase === phase), 'the malformed response reached its validator');
    assert.equal(error?.code, 'ERR_ASSERTION', error?.stack);
    assert.deepEqual(messages, []);
  });
}

test('the actual guest file must retain its identity after paired captures', async t => {
  const { error, messages } = await invoke(t, { mutate(call) {
    if (call.phase === 'comparison') writeFileSync(call.wasm, 'changed paired guest bytes');
  } });
  assert.match(error?.message ?? '', /guest changed across paired captures/);
  assert.deepEqual(messages, []);
});

for (const phase of ['build', 'component', 'native', 'cost', 'capture', 'pair', 'comparison']) {
  test(`document orchestration stops on a failed ${phase} subprocess`, async t => {
    const { calls, error, messages } = await invoke(t, { resultFor(call) {
      return call.phase === phase ? { status: 7, signal: null, stdout: '', stderr: 'fixture process failure' } : null;
    } });
    assert.match(error?.message ?? '', /fixture process failure/);
    assert.equal(calls.at(-1).phase, phase);
    assert.deepEqual(messages, []);
  });
}

test('document orchestration preserves launch errors and rejects signalled or noisy native output', async t => {
  const launchError = Object.assign(new Error('fixture launch error'), { code: 'ENOENT' });
  for (const outcome of [
    { error: launchError },
    { signal: 'SIGTERM', status: null, stderr: '', stdout: '' },
    { signal: null, status: 0, stderr: 'unexpected native warning', stdout: '' },
  ]) {
    const { calls, error, messages } = await invoke(t, { resultFor(call) {
      return call.phase === 'component' ? outcome : null;
    } });
    if (outcome.error) assert.equal(error, launchError);
    else assert.equal(error?.code, 'ERR_ASSERTION', error?.stack);
    assert.equal(calls.at(-1).phase, 'component');
    assert.deepEqual(messages, []);
  }
});
