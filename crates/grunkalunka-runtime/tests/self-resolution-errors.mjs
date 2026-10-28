import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createAsyncFunction} from '../js/async.mjs';

async function observe(run) {
  const reasons=[];
  const listener=reason=>reasons.push(reason);
  process.on('unhandledRejection',listener);
  try {
    const first=await run(4,2);
    await new Promise(setImmediate);
    assert(first instanceof TypeError);
    assert.equal(reasons.length,1);
    assert.strictEqual(reasons[0],first,'root result and rejection must retain the same Error object');
    const second=await run(4,2);
    await new Promise(setImmediate);
    assert(second instanceof TypeError);
    assert.notStrictEqual(first,second,'instances must not share Error objects');
    assert.equal(reasons.length,2);
    assert.strictEqual(reasons[1],second);
    return [first.name,second.name,reasons.length];
  } finally {process.off('unhandledRejection',listener);}
}
for(const {source,wasm,protocol} of JSON.parse(readFileSync(0,'utf8'))) {
  const run=createAsyncFunction(Uint8Array.from(wasm));
  if(protocol){await assert.rejects(run(4,2));continue;}
  const native=new Function(source.replace('export ','')+';return run;')();
  assert.deepEqual(await observe(run),await observe(native));
}
console.log('ERROR_OBJECTS_DONE');
