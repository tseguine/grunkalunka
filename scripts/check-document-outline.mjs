// Actual Markdown guest + WIT document capability, compared with native outlines.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
function run(command, args, capture = false, input) {
  const result = spawnSync(command, args, { cwd: root, input, shell: false,
    encoding: 'utf8', timeout: 180_000, killSignal: 'SIGKILL', maxBuffer: 2 * 1024 * 1024,
    stdio: capture ? ['pipe', 'pipe', 'pipe'] : 'inherit' });
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.equal(result.status, 0, result.stderr || `${command} failed`);
  if (capture) assert.equal(result.stderr, '');
  return result.stdout;
}
const build = ['build', '-p', 'grunkalunka-markdown', '--example', 'outline_guest', '--locked', '--offline', '-j', '4'];
run('cargo', build);
run('cargo', [...build, '--target', 'wasm32-unknown-unknown']);
run('cargo', ['build', '-p', 'grunkalunka-runtime', '--example', 'document_outline', '--locked', '--offline', '-j', '4']);
const executable = name => resolve(target, 'debug/examples', name + (process.platform === 'win32' ? '.exe' : ''));
const output = run(executable('document_outline'), [resolve(target, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm')], true);
assert.ok(output.endsWith('\n'));
const records = output.slice(0, -1).split('\n');
assert.equal(records.length, 9, 'every supplied fixture must be compared');
for (const record of records) {
  const parts = record.split('\t');
  assert.equal(parts.length, 2);
  for (const part of parts) assert.match(part, /^(?:[a-f0-9]{2})*$/);
  const source = Buffer.from(parts[0], 'hex');
  const actual = Buffer.from(parts[1], 'hex');
  const expected = Buffer.from(run(executable('outline_guest'), [], true, source));
  assert.deepEqual(actual, expected, 'component records must equal the native library projection');
}
run('cargo', ['build', '-p', 'grunkalunka-runtime', '--example', 'markdown_cost', '--locked', '--offline', '-j', '4']);
const wasm = resolve(target, 'wasm32-unknown-unknown/debug/examples/outline_guest.wasm');
// A second process must reproduce the same source, output and declared conditions.
// Hash the actual guest before and after; byte size alone cannot establish identity.
const guestHash = () => createHash('sha256').update(readFileSync(wasm)).digest('hex');
const initialGuestHash = guestHash();
function runAndValidateCost(selected) {
  const raw = run(executable('markdown_cost'), [
    wasm, '--workload', selected, '--samples', '2', '--iterations', '2', '--warmup', '1',
  ], true);
  const report = JSON.parse(raw);
  assert.equal(report.schemaVersion, 1);
  assert.equal(report.workload, 'markdown-wit-inspection');
  assert.equal(report.configuration.case, selected);
  assert.equal(report.configuration.samples, 2);
  assert.equal(report.configuration.iterations, 2);
  assert.equal(report.configuration.warmup, 1);
  assert.equal(report.timings.unit, 'milliseconds');
  assert.equal(report.timings.order, 'alternating');
  assert.ok(report.artifact.guestBytes > 0);
  assert.equal(report.artifact.guestSha256, initialGuestHash,
    'native Rust runner must hash the exact Wasm file checked independently by Node');
  assert.ok(report.artifact.componentBytes > report.artifact.guestBytes);
  assert.match(report.artifact.componentSha256, /^[a-f0-9]{64}$/,
    'native runner must identify its actual embedded component bytes');
  const memory = report.processMemory;
  assert.equal(memory.scope, 'entire-rust-process');
  assert.equal(memory.unit, 'bytes');
  if (['linux', 'win32', 'darwin'].includes(process.platform)) {
    const source = {
      linux: 'linux-proc-status',
      win32: 'windows-psapi-working-set',
      darwin: 'macos-mach-task-basic-info',
    }[process.platform];
    assert.equal(memory.source, source);
    const phases = [
      'beforeEmbed', 'afterCompile', 'afterInstantiation',
      'afterWarmup', 'afterBatches',
    ];
    assert.deepEqual(Object.keys(memory.snapshots), phases);
    let previousPeak = 0;
    for (const phase of phases) {
      const sample = memory.snapshots[phase];
      for (const value of [sample.rssBytes, sample.peakRssBytes]) {
        assert.ok(Number.isSafeInteger(value) && value >= 0);
      }
      assert.ok(sample.peakRssBytes >= sample.rssBytes);
      assert.ok(sample.peakRssBytes >= previousPeak);
      previousPeak = sample.peakRssBytes;
    }
  } else {
    assert.equal(memory.source, 'unsupported-on-this-host');
    assert.equal(memory.snapshots, null);
  }
  assert.equal(report.outcomes.grantedBackingReads, 6);
  assert.equal(report.outcomes.deniedBackingReads, 0);
  assert.equal(report.outcomes.grantedCleanups, 7);
  assert.equal(report.outcomes.deniedCleanups, 6);
  const fixture = report.fixture;
  for (const encoded of [fixture.sourceHex, fixture.outlineHex]) {
    assert.match(encoded, /^(?:[0-9a-f]{2})*$/);
  }
  const source = Buffer.from(fixture.sourceHex, 'hex');
  const expected = Buffer.from(run(executable('outline_guest'), [], true, source));
  assert.deepEqual(Buffer.from(fixture.outlineHex, 'hex'), expected,
    'cost workload must equal the unchanged native Markdown projection');
  assert.equal(report.freshLifecycles.count, 2);
  assert.equal(report.freshLifecycles.unit, 'milliseconds per fresh component lifecycle');
  for (const series of [
    report.timings.instantiation, report.timings.granted, report.timings.denied,
    report.timings.nativeOutline,
    report.freshLifecycles.granted, report.freshLifecycles.denied,
  ]) {
    assert.equal(series.samplesMs.length, 2);
    assert.ok(series.samplesMs.every(ms => Number.isFinite(ms) && ms >= 0));
    assert.ok(series.minMs <= series.medianMs && series.medianMs <= series.maxMs);
  }
  for (const observed of [report.timings.embedMs, report.timings.componentCompileMs]) {
    assert.ok(Number.isFinite(observed) && observed >= 0);
  }
  return report;
}
let initialUnicode;
for (const selected of ['empty', 'unicode', 'dense', 'wide']) {
  const report = runAndValidateCost(selected);
  if (selected === 'unicode') initialUnicode = report;
}
const repeatedUnicode = runAndValidateCost('unicode');
for (const field of ['workload', 'schemaVersion', 'configuration', 'environment', 'artifact', 'fixture']) {
  assert.deepEqual(repeatedUnicode[field], initialUnicode[field],
    `separate Unicode report must repeat ${field} before any timing comparison`);
}
assert.equal(guestHash(), initialGuestHash, 'Markdown guest binary changed across report collection');
run(process.execPath, ['--test', 'tests/build/markdown-capture.test.mjs']);
const captured = JSON.parse(run(process.execPath, [
  'tools/bench/capture-markdown.mjs', executable('markdown_cost'), wasm,
  '--workload', 'unicode', '--samples', '2', '--iterations', '2', '--warmup', '1',
  '--build-label', 'ci-debug',
], true));
assert.equal(captured.schemaVersion, 1);
assert.equal(captured.workload, 'markdown-wit-capture');
assert.ok(Number.isFinite(captured.nativeProcessElapsedMs) &&
  captured.nativeProcessElapsedMs >= 0,
  'capture must record monotonic wall time for the real native subprocess');
assert.equal(captured.provenance.guest.sha256, initialGuestHash);
assert.equal(captured.report.artifact.guestSha256, initialGuestHash);
assert.equal(captured.report.artifact.componentSha256, repeatedUnicode.artifact.componentSha256);
assert.equal(captured.provenance.guest.bytes, readFileSync(wasm).byteLength);
assert.match(captured.provenance.runner.sha256, /^[0-9a-f]{64}$/);
assert.match(captured.provenance.gitCommit, /^[0-9a-f]{40}$/);
assert.equal(captured.provenance.buildLabel, 'ci-debug');
assert.equal(typeof captured.provenance.trackedChangesPresent, 'boolean');
assert.match(captured.provenance.harnessSha256['tools/bench/capture-markdown.mjs'], /^[0-9a-f]{64}$/);
assert.deepEqual(captured.report.configuration, repeatedUnicode.configuration);
assert.deepEqual(captured.report.fixture, repeatedUnicode.fixture);
assert.equal(captured.report.outcomes.deniedBackingReads, 0);
run(process.execPath, ['--test', 'tests/build/markdown-comparison.test.mjs']);
const paired = JSON.parse(run(process.execPath, [
  'tools/bench/compare-markdown.mjs', '--capture', executable('markdown_cost'), wasm,
  '--workload', 'unicode', '--samples', '2', '--iterations', '2', '--warmup', '1',
  '--build-label', 'ci-debug',
], true));
assert.equal(paired.workload, 'markdown-wit-same-process-pair');
assert.equal(paired.binding.method, 'same-node-process-sequential');
assert.equal(paired.captures.length, 2);
for (const sample of paired.captures) {
  assert.equal(sample.provenance.guest.sha256, initialGuestHash);
  assert.ok(Number.isFinite(sample.nativeProcessElapsedMs) &&
    sample.nativeProcessElapsedMs >= 0);
  assert.equal(sample.report.artifact.guestSha256, initialGuestHash);
  assert.equal(sample.report.artifact.componentSha256, captured.report.artifact.componentSha256);
  assert.equal(sample.provenance.runner.sha256, captured.provenance.runner.sha256);
  assert.deepEqual(sample.report.fixture, captured.report.fixture);
  assert.deepEqual(sample.report.configuration, captured.report.configuration);
  assert.equal(sample.provenance.trackedChangesPresent, false);
}
const compared = JSON.parse(run(process.execPath, [
  'tools/bench/compare-markdown.mjs', '-',
], true, JSON.stringify(paired)));
assert.equal(compared.workload, 'markdown-wit-repeat-comparison');
assert.equal(compared.guestSha256, initialGuestHash);
assert.deepEqual(compared.nativeProcessElapsedMs, {
  baselineMs: paired.captures[0].nativeProcessElapsedMs,
  candidateMs: paired.captures[1].nativeProcessElapsedMs,
  deltaMs: paired.captures[1].nativeProcessElapsedMs -
    paired.captures[0].nativeProcessElapsedMs,
});
assert.equal(compared.runnerSha256, captured.provenance.runner.sha256);
assert.deepEqual(compared.configuration, captured.report.configuration);
assert.equal(compared.freshLifecycles.count, 2);
assert.equal(compared.freshLifecycles.unit, 'milliseconds per fresh component lifecycle');
for (const name of ['granted', 'denied']) {
  assert.deepEqual(compared.freshLifecycles[name].baselineSamplesMs,
    paired.captures[0].report.freshLifecycles[name].samplesMs);
  assert.deepEqual(compared.freshLifecycles[name].candidateSamplesMs,
    paired.captures[1].report.freshLifecycles[name].samplesMs);
  assert.ok(Number.isFinite(compared.freshLifecycles[name].median.deltaMs));
}
for (const name of ['instantiation', 'granted', 'denied', 'nativeOutline']) {
  assert.equal(compared.series[name].baselineSamplesMs.length, 2);
  assert.equal(compared.series[name].candidateSamplesMs.length, 2);
  assert.ok(Number.isFinite(compared.series[name].median.deltaMs));
}
assert.equal(guestHash(), initialGuestHash, 'guest changed across paired captures');
console.log('PASS capability-backed Markdown outlines: 9 native/component comparisons, policy errors, cleanup and fuel isolation; 4 cost fixtures with native parity; repeated Unicode report identity; guest sha256 ' + initialGuestHash);

