import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

const runner = new URL('../../scripts/check-command.mjs', import.meta.url).href;
async function invoke(t, command, args) {
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka check é '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const source = `
    import { runCheck } from ${JSON.stringify(runner)};
    try { runCheck(${JSON.stringify(command)}, ${JSON.stringify(args)}, process.cwd()); }
    catch (error) { console.error('CAUGHT ' + (error.code || error.message)); process.exitCode = 1; }
  `;
  const result = spawnSync(process.execPath, ['--input-type=module', '--eval', source], {
    cwd, encoding: 'utf8', timeout: 10_000, maxBuffer: 65536, killSignal: 'SIGKILL', shell: false,
  });
  assert.ifError(result.error);
  const events = prefix => result.stdout.split(/\r?\n/)
    .filter(line => line.startsWith(prefix)).map(line => JSON.parse(line.slice(prefix.length)));
  const starts = events('CHECK START ');
  const ends = events('CHECK END ');
  assert.equal(starts.length, 1);
  assert.equal(ends.length, 1);
  assert.deepEqual(starts[0], { command, args });
  assert.equal(ends[0].command, command);
  assert.deepEqual(ends[0].args, args);
  assert.ok(Number.isInteger(ends[0].elapsedMs) && ends[0].elapsedMs >= 0);
  return { ...result, end: ends[0] };
}

test('successful checks retain literal arguments, working directory and separate output streams', async t => {
  const args = ['space here', '猫', '; echo no', '$(echo no)', '"quoted"'];
  const body = `
    import { basename } from 'node:path';
    import { realpathSync } from 'node:fs';
    import assert from 'node:assert/strict';
    assert.match(basename(realpathSync(process.cwd())).normalize('NFC'), /^grunkalunka check é /);
    process.stdout.write('CHILD ' + JSON.stringify(process.argv.slice(1)));
    process.stderr.write('diagnostic\\n');
  `;
  const result = await invoke(t, process.execPath, ['--input-type=module', '--eval', body, '--', ...args]);
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.stdout.includes('CHILD ' + JSON.stringify(args)));
  assert.equal(result.stderr, 'diagnostic\n');
  assert.equal(result.end.status, 0);
  assert.equal(result.end.signal, null);
  assert.equal(result.end.error, null);
});

test('nonzero checks record the failed status without weakening failure propagation', async t => {
  const result = await invoke(t, process.execPath,
    ['--eval', "process.stdout.write('partial'); process.stderr.write('bad check\\n'); process.exitCode = 7;"]);
  assert.equal(result.status, 1);
  assert.equal(result.end.status, 7);
  assert.equal(result.end.error, null);
  assert.ok(result.stdout.includes('partial\nCHECK END '));
  assert.match(result.stderr, /bad check\nCAUGHT .* failed: 7/);
});

test('launch failure records and preserves the original error code', async t => {
  const result = await invoke(t, './definitely-absent-grunkalunka-check', []);
  assert.equal(result.status, 1);
  assert.equal(result.end.error, 'ENOENT');
  assert.match(result.stderr, /CAUGHT ENOENT/);
});
