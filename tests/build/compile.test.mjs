import assert from 'node:assert/strict';
import { access, chmod, copyFile, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compile, compileSource, CompilerError } from '../../tools/build/compile.mjs';

const binary = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));
const source = 'export function run(a,b) { return (a * 2 + b) / 3; }';

async function setup(t) {
  // Missing build prerequisites fail rather than silently skipping integration.
  await access(binary);
  const cwd = await mkdtemp(join(tmpdir(), 'grunkalunka build é & '));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  return cwd;
}

async function instantiate(path) {
  const { instance } = await WebAssembly.instantiate(await readFile(path));
  return instance.exports;
}

test('real compiler supports both profiles, JS/TS, and literal path characters', async t => {
  const cwd = await setup(t);
  const compiler = `compiler with spaces${process.platform === 'win32' ? '.exe' : ''}`;
  await copyFile(binary, join(cwd, compiler));
  await chmod(join(cwd, compiler), 0o755);
  for (const extension of ['js', 'ts']) {
    const input = `input & ; $ 猫.${extension}`;
    const text = extension === 'ts'
      ? source.replace('run(a,b)', 'run(a: number,b: number): number') : source;
    await writeFile(join(cwd, input), text);
    for (const profile of ['numeric', 'primitive']) {
      const destination = `-out & ; $ ${extension} ${profile}.wasm`;
      const result = await compile({ compiler, source: input, destination, profile, cwd });
      assert.equal(result.destination, join(cwd, destination));
      assert.match(result.stdout, /^COMPILED /);
      assert.equal(result.stderr, '');
      const exports = await instantiate(result.destination);
      const value = exports.run(4, 1);
      if (profile === 'numeric') assert.equal(value, 3);
      else {
        assert.ok(Array.isArray(value), 'primitive profile returns tag and payload');
        assert.equal(value.length, 2);
        assert.equal(value[1], 3);
      }
    }
  }
});

test('existing outputs and compiler diagnostics are preserved', async t => {
  const cwd = await setup(t);
  const options = { compiler: relative(cwd, binary), source: 'in.js', destination: 'out.wasm', cwd };
  await writeFile(join(cwd, 'in.js'), source);
  await writeFile(join(cwd, 'out.wasm'), 'keep existing bytes');
  await assert.rejects(compile(options), error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.code, 1);
    assert.match(error.stderr, /ERROR:/);
    return true;
  });
  assert.equal(await readFile(join(cwd, 'out.wasm'), 'utf8'), 'keep existing bytes');
  await writeFile(join(cwd, 'in.js'), 'export function run( {');
  await assert.rejects(compile({ ...options, destination: 'invalid.wasm' }), error => {
    assert.equal(error.code, 1);
    assert.match(error.stderr, /ERROR:/);
    return true;
  });
  await assert.rejects(access(join(cwd, 'invalid.wasm')), { code: 'ENOENT' });
});

test('independent concurrent compilations return their own outputs', async t => {
  const cwd = await setup(t);
  await writeFile(join(cwd, 'in.js'), source);
  const results = await Promise.all(['first.wasm', 'second.wasm'].map(destination =>
    compile({ compiler: binary, source: 'in.js', destination, cwd })));
  assert.notEqual(results[0].destination, results[1].destination);
  for (const result of results) assert.equal((await instantiate(result.destination)).run(4, 1), 3);
});

test('source-text builds return executable owned bytes for JS/TS and both profiles', async t => {
  const cwd = await setup(t);
  for (const language of ['js', 'ts']) {
    const text = '// café 🦀\r\n' + (language === 'ts'
      ? source.replace('run(a,b)', 'run(a: number,b: number): number') : source);
    for (const profile of ['numeric', 'primitive']) {
      const result = await compileSource({
        compiler: relative(cwd, binary), source: text, language, profile, cwd, tempDirectory: cwd,
      });
      assert.deepEqual(await readdir(cwd), []);
      assert.ok(Buffer.isBuffer(result.wasm));
      assert.match(result.stdout, /^COMPILED /);
      assert.equal(result.stderr, '');
      const { instance } = await WebAssembly.instantiate(result.wasm);
      assert.deepEqual(instance.exports.run(4, 1), profile === 'numeric' ? 3 : [3, 3]);
    }
  }
});

test('source-text admission failures keep compiler diagnostics and remove scratch files', async t => {
  const cwd = await setup(t);
  await assert.rejects(compileSource({
    compiler: binary, source: 'export function run( {', language: 'ts', tempDirectory: cwd,
  }), error => {
    assert.ok(error instanceof CompilerError);
    assert.equal(error.code, 1);
    assert.match(error.stderr, /ERROR:/);
    return true;
  });
  assert.deepEqual(await readdir(cwd), []);
});
