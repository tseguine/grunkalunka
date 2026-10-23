// Behavior baseline only: no compiled output is compared by this spike.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

async function load(name) {
  const source = await readFile(new URL(`fixtures/${name}.js`, import.meta.url), 'utf8');
  // A data URL selects ES module mode independently of package settings.
  return import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
}

assert.equal((await load('bindings')).run(), 42);
const ambient = await load('ambient');
assert.throws(() => ambient.run(), ReferenceError);
assert.deepEqual((await load('coercion')).run(), [5, 'rhs,valueOf', '23', -Infinity, NaN]);
assert.equal((await load('type-mismatch')).run(), 'not a number');
console.log(`PASS JS oracle (${process.version}): bindings=42; ambient=ReferenceError; coercion order=rhs,valueOf; result=5; concat=23; -Infinity; NaN; annotation-erased baseline=string`);
