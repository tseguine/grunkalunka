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
  assert.ok(report.artifact.componentBytes > report.artifact.guestBytes);
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
  for (const series of [
    report.timings.instantiation, report.timings.granted, report.timings.denied,
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
run(process.execPath, ['--test', 'tests/applications/note-preview.test.mjs']);
console.log('PASS capability-backed Markdown outlines: 9 native/component comparisons, policy errors, cleanup and fuel isolation; 4 cost fixtures with native parity; repeated Unicode report identity; guest sha256 ' + initialGuestHash);

