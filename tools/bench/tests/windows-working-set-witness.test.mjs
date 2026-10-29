// Windows-only native API proof for a future P1 memory collector boundary.
// Non-Windows skips are NOT evidence of Windows support.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const fixture = fileURLToPath(new URL('./fixtures/windows-working-set.rs', import.meta.url));

function run(exe, args, timeout = 90_000) {
  const child = spawnSync(exe, args, {
    shell: false, windowsHide: true, encoding: 'utf8', timeout,
    maxBuffer: 64 * 1024, stdio: ['ignore', 'pipe', 'pipe'],
  });
  assert.ifError(child.error);
  assert.equal(child.signal, null, exe + ' terminated by signal');
  assert.equal(child.status, 0, child.stderr || exe + ' returned an error');
  return child.stdout;
}

function validate(stdout) {
  const lines = stdout.trimEnd().split(/\r?\n/);
  assert.equal(lines.length, 3, 'three real native snapshots');
  const values = [];
  for (const [index, phase] of ['before', 'during', 'after'].entries()) {
    const parts = lines[index].split('\t');
    assert.equal(parts.length, 3);
    assert.equal(parts[0], phase);
    const observed = parts.slice(1).map(value => {
      assert.match(value, /^(0|[1-9][0-9]*)$/);
      const bytes = Number(value);
      assert.ok(Number.isSafeInteger(bytes) && bytes > 0, 'positive integer bytes');
      return bytes;
    });
    assert.ok(observed[1] >= observed[0], 'peak at least current');
    if (values.length) {
      assert.ok(observed[1] >= values.at(-1)[1], 'process lifetime peak never decreases');
    }
    values.push(observed);
  }
  return values;
}

test('synthetic Windows process memory receipt rejects truncation and false peaks', () => {
  const sample = 'before\t1024\t4096\nduring\t2048\t8192\nafter\t2048\t8192\n';
  assert.deepEqual(validate(sample), [[1024,4096],[2048,8192],[2048,8192]]);
  for (const changed of [
    sample.replace('after\t2048\t8192\n', ''),
    sample.replace('during\t2048\t8192', 'during\t9000\t8192'),
    sample.replace('after\t2048\t8192', 'after\t2048\t100'),
    sample.replace('before\t1024\t4096', 'before\t0\t4096'),
    sample.replace('before\t1024\t4096', 'before\t1.5\t4096'),
    sample.replace('after\t2048\t8192', 'after\t2048\t9007199254740993'),
  ]) assert.throws(() => validate(changed));
});

test('native Windows K32GetProcessMemoryInfo observes real process working sets', {
  skip: process.platform !== 'win32' && 'requires native Windows host',
}, async t => {
  assert.equal(process.arch, 'x64', 'CI witness validates its declared native x64 layout');
  const code = await readFile(fixture);
  assert.ok(code.length > 300 && code.length < 16_384, 'bounded fixture bytes');
  const dir = await mkdtemp(join(tmpdir(), 'grunkalunka-p1-windows-memory-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const input = join(dir, 'witness.rs');
  const exe = join(dir, 'witness.exe');
  await writeFile(input, code, { flag: 'wx' });
  run('rustc', ['--edition=2024', '--crate-name', 'p1_windows_memory',
    '-C', 'opt-level=0', '-o', exe, input]);
  const values = validate(run(exe, [], 10_000));
  // Current RSS can decrease after release; do not assert deltas or thresholds.
  assert.equal(values.length, 3);
  console.log('P1 Windows native working-set control ' + JSON.stringify({
    os: process.platform, arch: process.arch, source: 'Kernel32!K32GetProcessMemoryInfo',
    phases: ['before','during','after'].map((phase,index) => ({
      phase, workingSetBytes: values[index][0], peakWorkingSetBytes: values[index][1],
    })),
    semantics: 'whole native process working set, not guest allocation or Linux VmRSS equivalence',
  }));
});
