import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
const {createAsyncFunction}=await import(pathToFileURL(process.argv[2]));
const cases=JSON.parse(readFileSync(0,'utf8'));
let unhandled=[],handled=[];
process.on('unhandledRejection',(reason,promise)=>unhandled.push({reason,promise}));
process.on('rejectionHandled',promise=>handled.push(promise));
const checkpoint=()=>new Promise(resolve=>setTimeout(resolve,0));
async function reason(run,a,b){try{await run(a,b);}catch(value){return {value};}assert.fail('expected rejection');}
function nativeFor(name){return async(a,b)=>{
    let p;
    if(name==='pending')p=(async()=>{await a;await b;return b;})();
    else if(name==='rejected'||name==='rejected-unhandled')p=Promise.reject(b);
    else if(name==='handled'){p=Promise.reject(b);p.catch(()=>{});}
    else if(name==='self'){let reject;p=new Promise((_,r)=>{reject=r;});reject(p);}
    else if(name==='nested')p=Promise.reject(Promise.resolve(b));
    else {p=Promise.resolve(b);if(name==='shared')Promise.reject(p);}
    throw p;
};}
async function observe(run,name,a,b){
    unhandled=[];handled=[];
    const p=(await reason(run,a,b)).value;
    assert(p instanceof Promise,'a rejected Promise reason retains its native Promise brand');
    let value;
    if(name!=='rejected-unhandled'){
        try{value=['fulfilled',await p];}
        catch(error){
            if(error===p)value=['self-rejected',true];
            else if(error instanceof Promise)value=['rejected-promise',await error];
            else value=['rejected',error];
        }
    }
    await checkpoint();await checkpoint();
    const reports=unhandled.slice();
    assert.equal(reports.length,['shared','rejected-unhandled'].includes(name)?1:0,`${name}: duplicate or missing rejection report`);
    if(name==='shared')assert.strictEqual(reports[0].reason,p,'root and notification share a reason object');
    if(name==='rejected-unhandled')assert.strictEqual(reports[0].promise,p,'notification uses the exposed Promise itself');
    for(const event of reports)event.promise.catch(()=>{});
    await checkpoint();await checkpoint();
    assert.equal(handled.length,reports.length);
    for(const event of reports)assert(handled.includes(event.promise));
    return value;
}
const valueKey=value=>typeof value==='number'&&Object.is(value,-0)?'-0':String(value);
async function reactionTrace(run,a,b){
    const trace=[];
    run(a,b).then(()=>trace.push('unexpected fulfillment'),p=>{
        trace.push(p instanceof Promise?'root-promise':'root-non-promise');
        if(!(p instanceof Promise))return;
        p.then(value=>trace.push('fulfilled:'+valueKey(value)),reason=>{
            if(reason===p)trace.push('rejected-self');
            else if(reason instanceof Promise){
                trace.push('rejected-promise');
                reason.then(value=>trace.push('nested:'+valueKey(value)),()=>trace.push('nested-rejection'));
            }else trace.push('rejected:'+valueKey(reason));
        });
    });
    await new Promise(resolve=>{let n=0;function tick(){trace.push('tick'+(++n));if(n<34)queueMicrotask(tick);else resolve();}queueMicrotask(tick);});
    return trace;
}
for(const {name,wasm} of cases){
    const compiled=createAsyncFunction(Uint8Array.from(wasm));
    if(name.startsWith('invalid:')){
        unhandled=[];
        const {value}=await reason(compiled,4,2);
        assert(value instanceof TypeError||value instanceof WebAssembly.RuntimeError,`${name}: reject malformed protocol as a terminal host error`);
        await checkpoint();await checkpoint();
        assert.equal(unhandled.length,0,`${name}: protocol failure must not invent guest rejection events`);
        continue;
    }
    if(name==='changing'){
        const {value}=await reason(compiled,4,2);
        assert(value instanceof TypeError,'a previously settled snapshot cannot become pending');
        continue;
    }
    if(name==='trap'){
        const {value}=await reason(compiled,4,2);
        assert(value instanceof Promise);
        const result=await Promise.race([value.then(()=>false,error=>error instanceof WebAssembly.RuntimeError),new Promise(resolve=>setImmediate(()=>resolve(false)))]);
        assert.equal(result,true,'a terminal late trap rejects exposed pending Promise objects');
        continue;
    }
    for(const [a,b] of [[4,2],[0,-0],[-2,-4],[NaN,Infinity]]){
        assert.deepEqual(await observe(compiled,name,a,b),await observe(nativeFor(name),name,a,b));
        if(!['shared','rejected-unhandled'].includes(name))assert.deepEqual(await reactionTrace(compiled,a,b),await reactionTrace(nativeFor(name),a,b),`${name}: root and reason reaction order`);
    }
    if(name==='fulfilled'||name==='pending'){
        const [first,second]=await Promise.all([reason(compiled,4,2),reason(compiled,4,2)]);
        assert.notStrictEqual(first.value,second.value,'separate invocations have separate objects');
        assert.equal(await first.value,2);assert.equal(await second.value,2);
    }
}
