import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, access, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import test from 'node:test';
import { compile, CompilerError } from '../../tools/build/compile.mjs';
import { invoke } from '../../tools/build/process.mjs';

const options = { cwd: process.cwd(), timeout: 5000, maxBuffer: 4096 };
const node = (source, args = [], extra = {}) => invoke(process.execPath,
  ['--input-type=module', '--eval', source, '--', ...args], { ...options, ...extra });

async function temporary(t) {
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka build é '));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

function failed(code) {
  return error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.code, code);
    assert.ok(error.cause instanceof Error);
    return true;
  };
}

test('arguments remain separate and streams remain distinct', async t => {
  const cwd = await temporary(t);
  const args = ['file with spaces.ts', '猫', 'a;b', '$(echo no)', '&echo no', '"quoted"'];
  const result = await node(
    "process.stdout.write(JSON.stringify([process.argv.slice(1), process.cwd()])); process.stderr.write('diagnostic\\n');",
    args, { cwd },
  );
  const [received, actualCwd] = JSON.parse(result.stdout);
  assert.deepEqual(received, args);
  assert.equal(await realpath(actualCwd), await realpath(cwd));
  assert.equal(result.stderr, 'diagnostic\n');
});

test('nonzero exit retains diagnostics, output and exit code', async () => {
  await assert.rejects(node("process.stdout.write('partial'); process.stderr.write('bad source'); process.exitCode = 7;"), error => {
    failed(7)(error);
    assert.equal(error.stdout, 'partial');
    assert.equal(error.stderr, 'bad source');
    return true;
  });
});

test('missing compiler is a launch error, not a successful build', async t => {
  const cwd = await temporary(t);
  await assert.rejects(compile({ compiler: './absent', source: 'in.ts', destination: 'out.wasm', cwd }), failed('ENOENT'));
});

test('invalid configuration fails before invoking an executable', async () => {
  const valid = { compiler: '/absent', source: 'input.ts', destination: 'out.wasm' };
  for (const change of [
    { compiler: '' }, { source: 'bad\0.ts' }, { destination: 123 }, { cwd: null },
    { profile: 'other' }, { signal: {} },
  ]) await assert.rejects(compile({ ...valid, ...change }), TypeError);
  for (const value of [0, -1, 1.5, Infinity, NaN, 2 ** 31]) {
    await assert.rejects(compile({ ...valid, timeout: value }), RangeError);
    await assert.rejects(compile({ ...valid, maxBuffer: value }), RangeError);
  }
});

test('pre-aborted calls do not launch even a missing executable', async () => {
  await assert.rejects(compile({
    compiler: '/absent', source: 'in.ts', destination: 'out.wasm', signal: AbortSignal.abort(),
  }), failed('ABORT_ERR'));
});

test('cancellation stops a started direct child before rejecting', { timeout: 15000 }, async t => {
  const cwd = await temporary(t);
  const marker = join(cwd, 'started');
  const controller = new AbortController();
  const pending = node(`
    import { writeFileSync, renameSync } from 'node:fs';
    const marker = process.argv[1];
    writeFileSync(marker + '.tmp', String(process.pid));
    renameSync(marker + '.tmp', marker);
    setInterval(() => {}, 1000);
  `, [marker], { signal: controller.signal, timeout: 10000 });
  const rejected = assert.rejects(pending, failed('ABORT_ERR'));
  // The rename publishes a complete PID; file creation alone can race its write.
  try {
    let started = false;
    for (let count = 0; count < 500; count++) {
      try { await access(marker); started = true; break; } catch { await delay(10); }
    }
    assert.ok(started, 'child did not reach startup marker');
    const pid = Number(await readFile(marker, 'utf8'));
    controller.abort();
    await rejected;
    assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' });
  } finally {
    controller.abort();
    await rejected;
  }
});

test('timeout terminates a nonterminating direct child', async () => {
  await assert.rejects(node('setInterval(() => {}, 1000);', [], { timeout: 50 }), error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.killed, true);
    return true;
  });
});

test('stdout and stderr capture limits both reject excessive output', async () => {
  for (const stream of ['stdout', 'stderr']) {
    await assert.rejects(node(`process.${stream}.write('x'.repeat(65536));`, [], { maxBuffer: 128 }),
      failed('ERR_CHILD_PROCESS_STDIO_MAXBUFFER'));
  }
});
