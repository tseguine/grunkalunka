import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { test262Programs, assertionBody } from './test262.mjs';

const manifest = JSON.parse(readFileSync(new URL('./test262/manifest.json', import.meta.url), 'utf8'));

test('all four adapted numeric assertion programs return success', () => {
  assert.equal(test262Programs.length, 4);
  for (const [, js, ts, expected] of test262Programs) {
    assert.equal(js, ts);
    assert.equal(expected, 0);
    assert.equal(new Function('a', 'b', js)(0, 0), 0);
  }
});

test('every mapped assertion can report its own failure index', () => {
  for (const item of manifest.cases) {
    for (let index = 0; index < item.checks.length; index++) {
      const checks = structuredClone(item.checks);
      checks[index].condition = '1';
      assert.equal(new Function('a', 'b', assertionBody(checks))(0, 0), index + 1, item.id);
    }
  }
});

test('the four retained originals pass as scripts in the Node oracle', () => {
  for (const item of manifest.cases) {
    const source = readFileSync(new URL(`./test262/${item.retainedPath}`, import.meta.url), 'utf8');
    vm.runInNewContext(source, { Test262Error: class Test262Error extends Error {} }, { timeout: 1000 });
  }
});
