import assert from 'node:assert/strict';
import { access, mkdtemp, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compileSource } from '../../tools/build/compile.mjs';
import { runPrimitive } from '../../tools/runtime/primitive.mjs';

const compiler = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));

test('compiled JS and TS primitive results execute through the worker API', async t => {
  await access(compiler); // Missing native build prerequisites fail, never skip.
  const tempDirectory = await mkdtemp(join(tmpdir(), 'grunkalunka primitive é '));
  t.after(() => rm(tempDirectory, { recursive: true, force: true }));
  for (const language of ['js', 'ts']) {
    const params = language === 'ts' ? 'a: number, b: number' : 'a, b';
    for (const [expression, expected] of [
      ['undefined', undefined], ['null', null], ['false', false], ['true', true],
      ['-0', -0], ['NaN', NaN], ['Infinity', Infinity], ['a < b ? null : a', 4],
    ]) {
      const { wasm } = await compileSource({ compiler, language, profile: 'primitive',
        source: `export function run(${params}) { return ${expression}; }`, tempDirectory });
      assert.ok(Object.is(await runPrimitive(wasm, 4, 1, { timeout: 10_000 }), expected), expression);
      if (expression === 'a < b ? null : a') assert.equal(await runPrimitive(wasm, 0, 1), null);
    }
  }
  assert.deepEqual(await readdir(tempDirectory), []);
});
