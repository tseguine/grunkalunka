import assert from 'node:assert/strict';
import fs, { readFile, mkdtemp, rename, rm, unlink, writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';
import { followSource } from '../../examples/numeric-workflow/follow.mjs';
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
  assert.equal(pkg.scripts['example:watch'], 'node examples/numeric-workflow/watch.mjs');
  assert.equal(pkg.scripts['example:numeric'], 'node examples/numeric-workflow/run.mjs');
});

async function setup(t, bytes = 'first') {
  const directory = await mkdtemp(join(tmpdir(), 'grunkalunka watch é '));
  const path = join(directory, 'input.js');
  await writeFile(path, bytes);
  const controller = new AbortController();
  let pending;
  t.after(async () => {
    controller.abort();
    await pending?.catch(() => {});
    await rm(directory, { recursive: true, force: true });
  });
  return { path, controller, start(build, report) {
    pending = followSource(path, build, report, { signal: controller.signal, interval: 25 });
    return pending;
  } };
}

test('initial build preserves exact source bytes and unchanged content is not rebuilt', { timeout: 5000 }, async t => {
  const bytes = Buffer.from('\ufeff// 猫\r\nfirst\r\n');
  const state = await setup(t, bytes);
  let builds = 0;
  const pending = state.start(async source => {
    builds++;
    assert.deepEqual(source, bytes);
    return 42;
  }, async outcome => {
    assert.deepEqual(outcome, { value: 42 });
    await writeFile(state.path, bytes); // A timestamp change is not a source edit.
    setTimeout(() => state.controller.abort(), 125);
  });
  await pending;
  assert.equal(builds, 1);
});

test('edits during a build suppress its stale success and error outcomes', { timeout: 5000 }, async t => {
  for (const fail of [false, true]) {
    const state = await setup(t);
    const observed = [];
    const built = [];
    await state.start(async source => {
      const text = source.toString();
      built.push(text);
      if (text === 'first') {
        await writeFile(state.path, 'latest');
        if (fail) throw new Error('superseded diagnostic');
        return 'superseded result';
      }
      return text;
    }, outcome => { observed.push(outcome); state.controller.abort(); });
    assert.deepEqual(built, ['first', 'latest']);
    assert.deepEqual(observed, [{ value: 'latest' }]);
  }
});

test('failed edits and deletion recover without using an earlier result', { timeout: 5000 }, async t => {
  const state = await setup(t, 'invalid');
  const observed = [];
  let builds = 0;
  await state.start(async source => {
    builds++;
    if (source.toString() === 'invalid') throw new Error('bad source');
    return source.toString();
  }, async outcome => {
    observed.push(outcome.error?.code ?? outcome.error?.message ?? outcome.value);
    if (observed.length === 1) await writeFile(state.path, 'valid');
    else if (observed.length === 2) await unlink(state.path);
    else if (observed.length === 3) await writeFile(state.path, 'valid');
    else state.controller.abort();
  });
  assert.deepEqual(observed, ['bad source', 'valid', 'ENOENT', 'valid']);
  assert.equal(builds, 3);
});

test('atomic file replacement is detected without retaining the old file identity', { timeout: 5000 }, async t => {
  const state = await setup(t);
  const observed = [];
  await state.start(async bytes => bytes.toString(), async outcome => {
    observed.push(outcome.value);
    if (observed.length === 1) {
      await writeFile(state.path + '.replacement', 'second');
      await rename(state.path + '.replacement', state.path);
    } else state.controller.abort();
  });
  assert.deepEqual(observed, ['first', 'second']);
});

test('cancellation waits for the active build to settle and suppresses its outcome', { timeout: 5000 }, async t => {
  const state = await setup(t);
  let finished = false;
  const pending = state.start(async (_bytes, signal) => {
    state.controller.abort();
    try { await delay(10, undefined, { signal }); }
    finally { finished = true; }
  }, () => assert.fail('cancelled build was reported'));
  await pending;
  assert.equal(finished, true);
});

test('reporting failures remain fatal rather than becoming rebuild diagnostics', { timeout: 5000 }, async t => {
  const state = await setup(t);
  const expected = new Error('output closed');
  await assert.rejects(state.start(async () => 1, () => { throw expected; }), error => error === expected);
});

test('invalid options reject and an already-aborted watch performs no build or report', async () => {
  const unused = () => assert.fail('unexpected callback');
  for (const interval of [0, 24, 1.5, NaN, Infinity, 60001]) {
    await assert.rejects(followSource('absent', unused, unused, { interval }), RangeError);
  }
  await assert.rejects(followSource('absent', unused, unused, { signal: {} }), TypeError);
  await followSource('absent', unused, unused, { signal: AbortSignal.abort() });
});

// Script the observed reads, not wall-clock races between real file writes.
for (const failed of [false, true]) {
  test(`an unpublished ${failed ? 'failure' : 'success'} is rebuilt after an A-B-A edit`, { timeout: 2000 }, async t => {
    const controller = new AbortController();
    const sequence = ['first', 'intermediate', 'first', 'first'];
    let reads = 0;
    let builds = 0;
    const reports = [];
    t.mock.method(fs, 'readFile', async () => {
      if (reads >= sequence.length) controller.abort();
      return Buffer.from(sequence[Math.min(reads++, sequence.length - 1)]);
    });
    await followSource('scripted.js', async source => {
      builds++;
      if (builds === 1 && failed) throw new Error('discarded error');
      return `${source}:build-${builds}`;
    }, outcome => {
      reports.push(outcome);
      controller.abort();
    }, { interval: 25, signal: AbortSignal.any([controller.signal, t.signal]) });
    assert.equal(builds, 2);
    assert.equal(reads, 4);
    assert.deepEqual(reports, [{ value: 'first:build-2' }]);
  });
}

test('a discarded edit does not invalidate an already reported identical snapshot', { timeout: 2000 }, async t => {
  const controller = new AbortController();
  const sequence = ['first', 'first', 'second', 'first', 'first'];
  let reads = 0;
  const builds = [];
  const reports = [];
  t.mock.method(fs, 'readFile', async () => {
    if (reads >= sequence.length) controller.abort();
    return Buffer.from(sequence[Math.min(reads++, sequence.length - 1)]);
  });
  await followSource('scripted.js', async source => {
    builds.push(source.toString());
    return source.toString();
  }, outcome => reports.push(outcome), {
    interval: 25, signal: AbortSignal.any([controller.signal, t.signal]),
  });
  assert.deepEqual(builds, ['first', 'second']);
  assert.deepEqual(reports, [{ value: 'first' }]);
});
