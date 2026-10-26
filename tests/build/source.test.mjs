import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, extname, join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import test from 'node:test';
import { compileSource, CompilerError } from '../../tools/build/compile.mjs';

// Node runs these CLI fixtures as scripts named compile and compile-value.
// They exercise real subprocess/file lifetimes without interpreting source text.
const fixture = `
const fs = require('node:fs');
const [input, output] = process.argv.slice(2);
const source = fs.readFileSync(input, 'utf8');
process.stdout.write(JSON.stringify({ input, output, source, command: require('node:path').basename(process.argv[1]) }));
process.stderr.write('fixture diagnostic');
if (source !== 'no-output') fs.writeFileSync(output, Buffer.from([0,97,115,109,1,0,0,0]));
if (source === 'reject') process.exitCode = 7;
if (source === 'overflow') process.stdout.write('x'.repeat(65536));
if (source === 'hang') {
  fs.writeFileSync('started.tmp', String(process.pid));
  fs.renameSync('started.tmp', 'started');
  setInterval(() => {}, 1000);
}
`;

async function setup(t) {
  const cwd = await fs.mkdtemp(join(tmpdir(), 'grunkalunka source é & '));
  t.after(() => fs.rm(cwd, { recursive: true, force: true }));
  await fs.mkdir(join(cwd, 'scratch'));
  await fs.writeFile(join(cwd, 'scratch', 'keep'), 'caller-owned');
  for (const command of ['compile', 'compile-value']) await fs.writeFile(join(cwd, command), fixture);
  return { compiler: relative(cwd, process.execPath), cwd, tempDirectory: 'scratch', language: 'js' };
}

async function clean(options) {
  assert.deepEqual(await fs.readdir(join(options.cwd, 'scratch')), ['keep']);
  assert.equal(await fs.readFile(join(options.cwd, 'scratch', 'keep'), 'utf8'), 'caller-owned');
}

function compilerError(code) {
  return error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.code, code);
    return true;
  };
}

test('source text preserves UTF8, CRLF and explicit language; bytes survive cleanup', async t => {
  const options = await setup(t);
  for (const language of ['js', 'ts']) {
    for (const profile of ['numeric', 'primitive']) {
      const source = '\uFEFF// café 🦀\r\n// \\uD800\r\n';
      const result = await compileSource({ ...options, source, language, profile });
      const observed = JSON.parse(result.stdout);
      assert.equal(observed.source, source);
      assert.equal(extname(observed.input), `.${language}`);
      assert.equal(observed.command, profile === 'numeric' ? 'compile' : 'compile-value');
      assert.equal(result.stderr, 'fixture diagnostic');
      assert.ok(Buffer.isBuffer(result.wasm));
      assert.equal('destination' in result, false);
      await clean(options);
      await assert.rejects(fs.access(dirname(observed.input)), { code: 'ENOENT' });
      assert.equal(WebAssembly.validate(result.wasm), true);
    }
  }
});

test('invalid source text and language reject rather than being rewritten', async t => {
  const options = await setup(t);
  for (const source of [null, 123, '\uD800', '\uDC00', 'a\uD800b']) {
    await assert.rejects(compileSource({ ...options, source }), TypeError);
  }
  for (const language of [undefined, 'tsx', 'javascript', '']) {
    await assert.rejects(compileSource({ ...options, source: '', language }), TypeError);
  }
  for (const change of [{ cwd: null }, { tempDirectory: '' }, { tempDirectory: 'bad\0' }]) {
    await assert.rejects(compileSource({ ...options, source: '', ...change }), TypeError);
  }
  // Empty text reaches the compiler; admission is not decided by the wrapper.
  assert.equal(JSON.parse((await compileSource({ ...options, source: '' })).stdout).source, '');
  await clean(options);
});

test('partial output, launch failure, overflow and missing output all clean up', async t => {
  const options = await setup(t);
  await assert.rejects(compileSource({ ...options, source: 'reject' }), error => {
    compilerError(7)(error);
    assert.equal(error.stderr, 'fixture diagnostic');
    return true;
  });
  await clean(options);
  await assert.rejects(compileSource({ ...options, source: '', compiler: './missing' }), compilerError('ENOENT'));
  await clean(options);
  await assert.rejects(compileSource({ ...options, source: 'overflow', maxBuffer: 128 }),
    compilerError('ERR_CHILD_PROCESS_STDIO_MAXBUFFER'));
  await clean(options);
  await assert.rejects(compileSource({ ...options, source: 'no-output' }), { code: 'ENOENT' });
  await clean(options);
});

test('invalid process options and pre-aborted requests leave no scratch directory', async t => {
  const options = await setup(t);
  for (const [change, expected] of [
    [{ compiler: '' }, TypeError], [{ signal: {} }, TypeError],
    [{ profile: 'other' }, TypeError], [{ timeout: 0 }, RangeError], [{ maxBuffer: 0 }, RangeError],
  ]) {
    await assert.rejects(compileSource({ ...options, source: '', ...change }), expected);
    await clean(options);
  }
  await assert.rejects(compileSource({
    ...options, source: 'hang', signal: AbortSignal.abort(),
  }), compilerError('ABORT_ERR'));
  await assert.rejects(fs.access(join(options.cwd, 'started')), { code: 'ENOENT' });
  await clean(options);
});

test('cancellation waits for child termination before removing its workspace', { timeout: 20000 }, async t => {
  const options = await setup(t);
  const controller = new AbortController();
  const rejected = assert.rejects(compileSource({
    ...options, source: 'hang', signal: controller.signal, timeout: 15000,
  }), compilerError('ABORT_ERR'));
  try {
    let pid;
    for (let count = 0; count < 1000; count++) {
      try { pid = Number(await fs.readFile(join(options.cwd, 'started'), 'utf8')); break; }
      catch (error) { if (error.code !== 'ENOENT') throw error; await delay(10); }
    }
    assert.ok(Number.isInteger(pid) && pid > 0, 'child did not publish its PID');
    controller.abort();
    await rejected;
    assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' });
    await clean(options);
  } finally {
    controller.abort();
    await rejected;
  }
});

test('timeout still removes temporary source and output', async t => {
  const options = await setup(t);
  await assert.rejects(compileSource({ ...options, source: 'hang', timeout: 100 }), error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.killed, true);
    return true;
  });
  await clean(options);
});

test('concurrent calls keep independent sources and directories', async t => {
  const options = await setup(t);
  const sources = ['first', 'second', 'third'];
  const results = await Promise.all(sources.map(source => compileSource({ ...options, source })));
  const observations = results.map(result => JSON.parse(result.stdout));
  assert.deepEqual(observations.map(result => result.source), sources);
  assert.equal(new Set(observations.map(result => dirname(result.input))).size, sources.length);
  await clean(options);
});

test('cleanup errors reject success and preserve an earlier compiler error', async t => {
  const options = await setup(t);
  const cleanupError = new Error('simulated cleanup failure');
  const stub = t.mock.method(fs, 'rm', async () => { throw cleanupError; });
  try {
    await assert.rejects(compileSource({ ...options, source: 'ok' }), error => error === cleanupError);
    await assert.rejects(compileSource({ ...options, source: 'reject' }), error => {
      assert.ok(error instanceof AggregateError);
      assert.equal(error.errors.length, 2);
      compilerError(7)(error.errors[0]);
      assert.equal(error.errors[1], cleanupError);
      assert.equal(error.cause, error.errors[0]);
      return true;
    });
  } finally {
    stub.mock.restore();
  }
});
