import assert from 'node:assert/strict';

const module = new WebAssembly.Module(bytes);
assert.equal(WebAssembly.Module.imports(module).length, 0);
const buffer = new ArrayBuffer(8), view = new DataView(buffer);
function decode(tag, payload) {
  return tag === 0 ? undefined : tag === 1 ? null : tag === 2 ? payload === 1 : payload;
}
function bits(value) {
  if (value === undefined || value === null || typeof value === 'boolean') return String(value);
  if (Number.isNaN(value)) return 'nan';
  view.setFloat64(0, value);
  return view.getBigUint64(0).toString(16).padStart(16, '0');
}
function native(trace) {
  return {
    new() { let resolve, reject; const promise = new Promise((a,b) => { resolve=a; reject=b; }); return {promise,resolve,reject}; },
    subscribe(p, frame, pc, action) {
      p.promise.then(v => deliver(frame,pc,0,v,action), v => deliver(frame,pc,1,v,action));
    },
    settle(p, rejected, tag, payload) { (rejected ? p.reject : p.resolve)(decode(tag,payload)); },
    schedule(frame,pc,rejected,tag,payload) {
      const p=this.new();this.subscribe(p,frame,pc);this.settle(p,rejected,tag,payload);
    },
    flush() {},
  };
  function deliver(frame,pc,rejected,value,action) { trace.push([frame,pc,rejected,bits(value)]); action?.(); }
}
function guest(trace) {
  const e = new WebAssembly.Instance(module).exports, actions = new Map();
  return {
    new() { return e.new(); },
    subscribe(p,frame,pc,action) { if(action)actions.set(frame,action);e.subscribe(p,frame,pc); },
    settle(p,rejected,tag,payload) { e.settle(p,rejected,tag,payload); },
    schedule(frame,pc,rejected,tag,payload) { e.schedule(frame,pc,rejected,tag,payload); },
    flush() {
      const count=e.issued();
      for(let i=0;i<count;i++) queueMicrotask(() => {
        const job=e.take();assert.notEqual(job,0);
        const [frame,pc,rejected,tag,payload]=e.read(job);
        trace.push([frame,pc,rejected,bits(decode(tag,payload))]);
        actions.get(frame)?.();
        this.flush();
      });
    },
  };
}
async function observe(create,rejected,tag,payload) {
  const trace=[], api=create(trace), p=api.new(),q=api.new(),r=api.new();
  api.subscribe(p,10,3,() => {
    api.subscribe(p,40,7);
    api.settle(r,1,2,0);
    api.flush();
  });
  api.subscribe(q,20,4);api.subscribe(p,30,5);api.subscribe(r,50,8);
  api.settle(p,rejected,tag,payload);api.flush();
  queueMicrotask(() => trace.push('external'));
  api.settle(q,1,1,0);api.flush();
  api.schedule(60,9,0,2,1);api.flush();
  api.settle(p,1-rejected,3,99);api.flush();
  trace.push('returned');
  await new Promise(resolve => {
    let count=0;
    function tick() { trace.push('tick'+(++count));if(count<34)queueMicrotask(tick);else resolve(); }
    queueMicrotask(tick);
  });
  return trace;
}
for(const rejected of [0,1]) for(const [tag,payload] of [[0,0],[1,0],[2,0],[2,1],[3,-0],[3,NaN],[3,Infinity]]) {
  assert.deepEqual(await observe(guest,rejected,tag,payload),await observe(native,rejected,tag,payload));
}

// The same private validation/resource controls execute in a second engine.
{
  const e=new WebAssembly.Instance(module).exports,p=e.new();
  for(const bad of [0,-1,p+1,p+16,1048576])assert.throws(()=>e.state(bad),WebAssembly.RuntimeError);
  assert.throws(()=>e.result(p),WebAssembly.RuntimeError);
  for(const [rejected,tag,payload]of [[2,3,0],[0,4,0],[0,0,-0],[0,1,1],[0,2,-0],[0,2,2],[0,2,NaN]]) {
    assert.throws(()=>e.settle(p,rejected,tag,payload),WebAssembly.RuntimeError);
    assert.throws(()=>e.schedule(1,1,rejected,tag,payload),WebAssembly.RuntimeError);
    assert.equal(e.state(p),0);assert.equal(e.ready(),0);
  }
}
for(const waiters of [false,true]) {
  const e=new WebAssembly.Instance(module).exports,p=e.new();let count=1;
  let trapped=false;
  for(;count<=32768;count++) {
    try { if(waiters)e.subscribe(p,count,count);else e.new(); }
    catch(error) { assert(error instanceof WebAssembly.RuntimeError);trapped=true;break; }
  }
  assert(trapped);assert.equal(count,32767);assert.equal(e.memory.buffer.byteLength,1048576);
}
