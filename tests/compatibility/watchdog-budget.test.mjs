import assert from 'node:assert/strict';
import test from 'node:test';
import { watchdogBudget, corpusWatchdogCounts, assertWithinWatchdog } from './watchdog-budget.mjs';

test('empty inventory retains the three-minute floor and one-minute outer margin', () => {
  assert.deepEqual(watchdogBudget({ paired: 0, rejected: 0, curated: 0 }),
    { innerMs: 180_000, outerMs: 240_000 });
});

test('current and arithmetic inventories allocate their exact bounded budgets', () => {
  assert.deepEqual(watchdogBudget({ paired: 1049, rejected: 815, curated: 38 }),
    { innerMs: 263_975, outerMs: 323_975 });
  assert.deepEqual(watchdogBudget({ paired: 1115, rejected: 839, curated: 38 }),
    { innerMs: 277_775, outerMs: 337_775 });
});

test('floor and cap boundaries preserve a finite outer timeout', () => {
  for (const [paired, innerMs] of [[749, 180_000], [750, 180_000], [751, 180_200],
    [2849, 599_800], [2850, 600_000], [2851, 600_000], [1_000_000, 600_000]]) {
    assert.deepEqual(watchdogBudget({ paired, rejected: 0, curated: 0 }),
      { innerMs, outerMs: innerMs + 60_000 });
  }
});

test('each inventory dimension grows the allocation below the cap', () => {
  const base = { paired: 1000, rejected: 100, curated: 10 };
  const { innerMs } = watchdogBudget(base);
  for (const [dimension, allowance] of [['paired', 200], ['rejected', 25], ['curated', 100]]) {
    assert.deepEqual(watchdogBudget({ ...base, [dimension]: base[dimension] + 1 }),
      { innerMs: innerMs + allowance, outerMs: innerMs + allowance + 60_000 });
  }
});

test('budgets are monotone and saturate at the cap for each dimension', () => {
  for (const dimension of ['paired', 'rejected', 'curated']) {
    let previous = 0;
    for (const size of [0, 1, 1000, 10_000, 20_000]) {
      const counts = { paired: 1000, rejected: 20_000, curated: 0, [dimension]: size };
      const budget = watchdogBudget(counts);
      assert.ok(budget.innerMs >= previous);
      assert.ok(budget.innerMs <= 600_000);
      assert.equal(budget.outerMs, budget.innerMs + 60_000);
      previous = budget.innerMs;
    }
    assert.equal(previous, 600_000);
  }
});

test('malformed counts and unsafe intermediate arithmetic fail before allocation', () => {
  const valid = { paired: 1049, rejected: 815, curated: 38 };
  for (const invalid of [undefined, null, [], 3, 'counts', {},
    { paired: 1, rejected: 1 }, { ...valid, extra: 0 }]) {
    assert.throws(() => watchdogBudget(invalid), /watchdog counts/);
  }
  for (const dimension of Object.keys(valid)) {
    for (const invalid of [undefined, null, '1', -1, 0.5, NaN, Infinity, -Infinity,
      Number.MAX_SAFE_INTEGER + 1]) {
      assert.throws(() => watchdogBudget({ ...valid, [dimension]: invalid }), /watchdog counts/);
    }
    const counts = { ...valid, [dimension]: Number.MAX_SAFE_INTEGER };
    if (dimension === 'curated') counts.rejected = Number.MAX_SAFE_INTEGER;
    assert.throws(() => watchdogBudget(counts), /unsafe watchdog arithmetic/);
  }
  // Each product is safe here; their sum (including the startup allowance) is not.
  assert.throws(() => watchdogBudget({
    paired: Math.floor(Number.MAX_SAFE_INTEGER / 200), rejected: 1, curated: 0,
  }), /unsafe watchdog arithmetic/);
  assert.throws(() => watchdogBudget({ paired: 1, rejected: 0, curated: 1 }), /curated.*rejected/);
});

test('corpus counts use paired rows and curated metadata without executing an oracle', () => {
  const gap = { get oracleSource() { throw new Error('oracle must not be read'); } };
  assert.deepEqual(corpusWatchdogCounts([['one'], ['two']], [['ordinary'], ['curated', '', 'js', '', gap]]),
    { paired: 2, rejected: 2, curated: 1 });
  assert.deepEqual(corpusWatchdogCounts([], [['ordinary'], ['no-gap', '', 'js', '', undefined]]),
    { paired: 0, rejected: 2, curated: 0 });
  for (const args of [[null, []], [[], {}], [[null], []], [[], [undefined]]]) {
    assert.throws(() => corpusWatchdogCounts(...args), /watchdog inventory/);
  }
});

test('elapsed guard allows the deadline and prevents the next launch over budget', () => {
  const { innerMs } = watchdogBudget({ paired: 1049, rejected: 815, curated: 38 });
  for (const elapsedMs of [0, innerMs - 1, innerMs]) {
    assert.doesNotThrow(() => assertWithinWatchdog(elapsedMs, innerMs));
  }
  let launched = false;
  assert.throws(() => {
    assertWithinWatchdog(innerMs + 1, innerMs);
    launched = true;
  }, /exceeded 263975ms compatibility watchdog budget/);
  assert.equal(launched, false);
});
