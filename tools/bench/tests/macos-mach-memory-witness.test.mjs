// Independent macOS Mach memory control, not a runtime collector change.
// Non-macOS skipped native execution is NOT positive Apple Silicon evidence.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const fixture = fileURLToPath(new URL('./fixtures/macos-mach-memory.rs', import.meta.url));

function run(command, args, timeout = 90_000) {
  const child = spawnSync(command, args, {
    shell: false, windowsHide: true, encoding: 'utf8', timeout,
    maxBuffer: 64 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  assert.ifError(child.error);
  assert.equal(child.signal, null, 'native control must not time out');
  assert.equal(child.status, 0, child.stderr || 'native control failed');
  return child.stdout;
}

function validate(stdout) {
  const lines = stdout.trimEnd().split(/\r?\n/);
  assert.equal(lines.length, 3, 'three native process snapshots');
  const observed = [];
  for (const [index, phase] of ['before', 'during', 'after'].entries()) {
    const parts = lines[index].split('\t');
    assert.equal(parts.length, 3);
    assert.equal(parts[0], phase);
    const values = parts.slice(1).map(text => {
      assert.match(text, /^[1-9][0-9]*$/);
      const bytes = Number(text);
      assert.ok(Number.isSafeInteger(bytes), 'positive safe integer bytes');
      return bytes;
    });
    assert.ok(values[1] >= values[0], 'peak at least current resident bytes');
    if (observed.length) {
      assert.ok(values[1] >= observed.at(-1)[1], 'process peak cannot decrease');
    }
    observed.push(values);
  }
  return observed;
}

test('synthetic Mach process-memory receipts reject malformed and false peaks', () => {
  const sample = 'before\t1024\t4096\nduring\t2048\t8192\nafter\t1024\t8192\n';
  assert.deepEqual(validate(sample), [[1024, 4096], [2048, 8192], [1024, 8192]]);
  for (const changed of [
    sample.replace('after\t1024\t8192\n', ''),
    sample.replace('during\t2048\t8192', 'during\t9000\t8192'),
    sample.replace('after\t1024\t8192', 'after\t1024\t100'),
    sample.replace('before\t1024\t4096', 'before\t0\t4096'),
    sample.replace('before\t1024\t4096', 'before\t1.5\t4096'),
    sample.replace('after\t1024\t8192', 'after\t1024\t9007199254740993'),
    sample.replace('during\t2048\t8192', 'other\t2048\t8192'),
  ]) assert.throws(() => validate(changed));
});

test('native macOS Mach task_info reads resident and peak task memory', {
  skip: process.platform !== 'darwin' && 'requires native macOS host',
}, async t => {
  assert.ok(['arm64', 'x64'].includes(process.arch), '64-bit macOS-only ABI');
  const code = await readFile(fixture);
  assert.ok(code.length > 300 && code.length < 16_384, 'bounded Rust fixture');
  const dir = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-macos-memory-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const input = join(dir, 'mach-witness.rs');
  const executable = join(dir, 'mach-witness');
  await writeFile(input, code, { flag: 'wx' });
  run('rustc', ['--edition=2024', '--crate-name', 'p1_macos_memory',
    '-C', 'opt-level=0', '-o', executable, input]);
  const observed = validate(run(executable, [], 10_000));
  assert.equal(observed.length, 3);
  console.log('P1 macOS native Mach memory control ' + JSON.stringify({
    os: process.platform, arch: process.arch, source: 'task_info(MACH_TASK_BASIC_INFO)',
    phases: ['before', 'during', 'after'].map((phase, index) => ({
      phase, residentBytes: observed[index][0], peakResidentBytes: observed[index][1],
    })),
    semantics: 'whole task residency, not guest-only memory or a cross-OS equivalent',
  }));
});
