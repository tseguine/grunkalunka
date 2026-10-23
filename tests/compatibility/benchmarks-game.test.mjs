import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { benchmarkPrograms, benchmarkVectors } from './benchmarks-game.mjs';

test('Mandelbrot kernel and independent loop oracle match named orbit counts', () => {
  assert.equal(benchmarkPrograms.length, 1);
  assert.equal(benchmarkVectors.length, 10);
  const [, js, ts, , oracle] = benchmarkPrograms[0];
  assert.equal(js, ts);
  const run = new Function('a', 'b', js);
  const reference = new Function('a', 'b', oracle);
  for (const { id, a, b, expected } of benchmarkVectors) {
    assert.equal(reference(a, b), expected, `${id}: oracle`);
    assert.equal(run(a, b), expected, `${id}: adaptation`);
  }
});

test('the kernel sign reproduces all pixels of the original 8 by 8 program', () => {
  const original = readFileSync(new URL('./benchmarks-game/original.js', import.meta.url), 'utf8');
  const chunks = [];
  vm.runInNewContext(original, { process: { argv: ['node', 'mandelbrot', '8'],
    stdout: { write(chunk) { chunks.push(Buffer.from(chunk)); } } } }, { timeout: 1000 });
  const output = Buffer.concat(chunks);
  const header = Buffer.from('P4\n8 8\n');
  assert.deepEqual(output.subarray(0, header.length), header);
  assert.equal(output.length, header.length + 8);
  const [, body, , , oracle] = benchmarkPrograms[0];
  for (const source of [body, oracle]) {
    const run = new Function('a', 'b', source);
    for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) {
      const pixel = (output[header.length + y] >> (7 - x)) & 1;
      assert.equal(run(2 * x / 8 - 1.5, 2 * y / 8 - 1) > 0 ? 1 : 0, pixel, `pixel ${x},${y}`);
    }
  }
});

test('named cases expose iteration truncation and a NaN-incorrect escape guard', () => {
  const body = benchmarkPrograms[0][1];
  const reduced = new Function('a', 'b', body.replace('const iter = 50;', 'const iter = 49;'));
  assert.equal(reduced(0.015625, -0.8203125), 49);
  const premature = new Function('a', 'b', body.replace('i = i + 1;', 'i = iter;'));
  assert.notEqual(premature(1, 0), -3);
  const wrongNaN = new Function('a', 'b', body.replace('if (tr + ti <= limit * limit)', 'if (!(tr + ti > limit * limit))'));
  assert.equal(wrongNaN(NaN, 0), -50);
  assert.equal(wrongNaN(0, NaN), -50);
});
