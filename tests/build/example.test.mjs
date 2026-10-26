import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { access, mkdtemp, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import test from 'node:test';

const execute = promisify(execFile);
const runner = fileURLToPath(new URL('../../examples/numeric-workflow/run.mjs', import.meta.url));
const binary = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));
async function setup(t) {
  await access(binary); // Required integration prerequisite, not a reason to skip.
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka example é & '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  return cwd;
}
function invoke(cwd, args = [], extraEnv = {}) {
  const env = { ...process.env, ...extraEnv };
  if (!Object.hasOwn(extraEnv, 'GRUNKALUNKA_COMPILER')) delete env.GRUNKALUNKA_COMPILER;
  return execute(process.execPath, [runner, ...args], {
    cwd, timeout: 30_000, maxBuffer: 1024 * 1024, killSignal: 'SIGKILL',
    env,
  });
}

test('example builds and runs repeatedly outside the repository working directory', async t => {
  const cwd = await setup(t);
  for (const [args, expected] of [[[], '6'], [[], '6'], [['-84', '30'], '6'],
    [['1071', '462'], '21'], [['0', '0'], '0'], [['0', '-9'], '9'],
    [['9007199254740991', '1'], '1']]) {
    const result = await invoke(cwd, args);
    assert.equal(result.stdout.trim(), expected);
    assert.equal(result.stderr, '');
  }
  assert.equal((await invoke(cwd, [], { GRUNKALUNKA_COMPILER: binary })).stdout.trim(), '6');
  assert.deepEqual(await readdir(cwd), []);
});

test('edited source rebuilds and rejected edits cannot execute stale output', async t => {
  const cwd = await setup(t);
  for (const extension of ['js', 'ts']) {
    const path = `edited 猫 & source.${extension}`;
    const args = ['4', '1', path];
    const signature = extension === 'ts' ? 'a: number, b: number' : 'a, b';
    await writeFile(join(cwd, path), `export function run(${signature}) { return a + b; }`);
    assert.equal((await invoke(cwd, args)).stdout.trim(), '5');
    await writeFile(join(cwd, path), `export function run(${signature}) { return a - b; }`);
    assert.equal((await invoke(cwd, args)).stdout.trim(), '3');
    await writeFile(join(cwd, path), 'export function run( {');
    await assert.rejects(invoke(cwd, args), error => {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, '');
      assert.ok(error.stderr.includes(path));
      assert.match(error.stderr, /ERROR:/);
      return true;
    });
    await writeFile(join(cwd, path), `export function run(${signature}) { return a * b; }`);
    assert.equal((await invoke(cwd, args)).stdout.trim(), '4');
  }
  assert.equal((await readdir(cwd)).filter(name => name.endsWith('.wasm')).length, 0);
});

test('missing compiler and invalid UTF8 report failure without a numeric result', async t => {
  const cwd = await setup(t);
  await assert.rejects(invoke(cwd, [], { GRUNKALUNKA_COMPILER: './missing' }), error => {
    assert.equal(error.code, 1);
    assert.equal(error.stdout, '');
    assert.match(error.stderr, /ENOENT/);
    return true;
  });
  await writeFile(join(cwd, 'bad.js'), Buffer.from([0xff]));
  await assert.rejects(invoke(cwd, ['1', '2', 'bad.js']), error => {
    assert.equal(error.code, 1);
    assert.equal(error.stdout, '');
    assert.match(error.stderr, /bad\.js/);
    return true;
  });
});


test('an infinite guest times out without a result and a subsequent edit runs normally', async t => {
  const cwd = await setup(t);
  const path = 'loop.js';
  await writeFile(join(cwd, path), 'export function run(a,b) { while (a) {} return b; }');
  await assert.rejects(invoke(cwd, ['1', '2', path]), error => {
    assert.equal(error.code, 1);
    assert.equal(error.stdout, '');
    assert.match(error.stderr, /Numeric execution timed out/);
    assert.ok(error.stderr.includes(path));
    return true;
  });
  await writeFile(join(cwd, path), 'export function run(a,b) { return a + b; }');
  assert.equal((await invoke(cwd, ['1', '2', path])).stdout.trim(), '3');
});
