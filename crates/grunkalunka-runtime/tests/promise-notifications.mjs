import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createAsyncFunction} from '../js/async.mjs';

const buffer=new ArrayBuffer(8),view=new DataView(buffer);
function bits(value) {
  if (typeof value !== 'number') return String(value);
  if (Number.isNaN(value)) return 'nan';
  view.setFloat64(0,value);
  return view.getBigUint64(0).toString(16).padStart(16,'0');
}
async function observe(run,inputs) {
  const reasons=[],promises=[];
  const listener=(reason,promise)=>{assert(!promises.includes(promise));promises.push(promise);reasons.push(['unhandled',bits(reason),promises.length]);};
  const handled=promise=>{assert(promises.includes(promise));reasons.push(['handled',promises.indexOf(promise)+1]);};
  process.on('unhandledRejection',listener);
  process.on('rejectionHandled',handled);
  try {
    const values=await Promise.all(inputs.map(([a,b])=>run(a,b).then(v=>['fulfilled',bits(v)],e=>['rejected',bits(e)])));
    await new Promise(setImmediate);
    await new Promise(setImmediate);
    return {values,reasons};
  } finally {process.off('unhandledRejection',listener);process.off('rejectionHandled',handled);}
}
const cases=JSON.parse(readFileSync(0,'utf8'));
for (const [index,{source,wasm,protocol,later}] of cases.entries()) {
  if(protocol) {
    const reasons=[];
    const listener=reason=>reasons.push(reason);
    process.on('unhandledRejection',listener);
    try {
      const run=createAsyncFunction(Uint8Array.from(wasm));
      if(later) assert.equal(await run(4,2),4); // Root already settled; later protocol error is terminal.
      else await assert.rejects(run(4,2),e=>/Promise event|primitive value/.test(e.message));
      await new Promise(setImmediate);
      assert.deepEqual(reasons,later?[2]:[]);
    } finally {process.off('unhandledRejection',listener);}
    continue;
  }
  const native=new Function(source.replace('export ','')+';return run;')();
  for (const inputs of [[[4,2]],[[4,-0]],[[4,NaN]],[[0,Infinity]],[[4,2],[0,-0]]]) {
    const expected=await observe(native,inputs);
    if(index<2 && inputs.length===1) assert.deepEqual(expected.reasons,[['unhandled',bits(inputs[0][1]),1]]);
    const actual=await observe(createAsyncFunction(Uint8Array.from(wasm)),inputs);
    assert.deepEqual(actual,expected,source);
  }
}
console.log('NOTIFICATIONS_DONE');
