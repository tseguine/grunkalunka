import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import test from 'node:test';
import { parseArguments } from '../../examples/numeric-workflow/run.mjs';
import { run } from '../../examples/numeric-workflow/gcd.js';

test('example source agrees with fixed GCD cases and an independent divisor oracle', () => {
  for (const [a, b, expected] of [[84, 30, 6], [-84, 30, 6], [0, -9, 9], [0, 0, 0],
    [1071, 462, 21], [Number.MAX_SAFE_INTEGER, 1, 1]]) assert.equal(run(a, b), expected);
  for (let a = -20; a <= 20; a++) for (let b = -20; b <= 20; b++) {
    let expected = 0;
    for (let d = 1; d <= Math.max(Math.abs(a), Math.abs(b)); d++) {
      if (a % d === 0 && b % d === 0) expected = d;
    }
    assert.equal(run(a, b), expected, `${a},${b}`);
  }
});

test('example arguments select defaults or explicit inputs without ambient source lookup', () => {
  const defaults = parseArguments([]);
  assert.deepEqual(defaults.inputs, [84, 30]);
  assert.ok(isAbsolute(defaults.sourcePath));
  assert.equal(defaults.language, 'js');
  assert.deepEqual(parseArguments(['-84', '+30', 'custom.ts']).inputs, [-84, 30]);
  assert.equal(parseArguments(['1', '2', 'custom.ts']).language, 'ts');
});

test('invalid example arguments reject before any compilation', () => {
  for (const args of [['1'], ['1', '2', 'a.js', 'extra'], ['1', '2', 'a.txt'],
    ...['', '1.5', 'NaN', 'Infinity', '1e3', '0xff', ' 1', '9007199254740992']
      .map(value => [value, '1'])]) assert.throws(() => parseArguments(args));
});

test('package script invokes the maintained example entry point', async () => {
  const pkg = JSON.parse(await readFile(new URL('../../package.json', import.meta.url), 'utf8'));
  assert.equal(pkg.scripts['example:numeric'], 'node examples/numeric-workflow/run.mjs');
});
