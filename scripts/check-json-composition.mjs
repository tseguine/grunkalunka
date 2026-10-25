// Actual runtime composition regression; all subprocesses are local and offline.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
const run=(command,args)=>{
  const r=spawnSync(command,args,{cwd:root,stdio:'inherit',shell:false});
  if(r.error)throw r.error;assert.equal(r.status,0,`${command} failed: ${r.signal??r.status}`);
};
run(process.execPath,['scripts/check-json-guest.mjs']);
const target=path.resolve(root,process.env.CARGO_TARGET_DIR??'target');
const out=path.join(target,'wasm-composition');fs.mkdirSync(out,{recursive:true});
const texts=['{"x":41,"s":"\\ud800","10":true,"2":false,"x":42}',
  '["\ud800",-0,1e400,[true,null],"\\u0000","😀"]'];
const expected=texts.map(s=>JSON.stringify(JSON.parse(s)));
const hex=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i).toString(16).padStart(4,'0')).join('');
const data=s=>Array.from({length:s.length},(_,i)=>{
  const u=s.charCodeAt(i);return `\\${(u&255).toString(16).padStart(2,'0')}\\${(u>>>8).toString(16).padStart(2,'0')}`;
}).join('');
const wat=`(module
 (import "runtime" "memory" (memory 1))
 (import "runtime" "gk_json_input_resize" (func $input (param i32) (result i32)))
 (import "runtime" "gk_json_parse" (func $parse (result i32)))
 (import "runtime" "gk_json_stringify" (func $stringify (param i32) (result i32)))
 (type $pair (func (result i32 f64)))
 (func $decode (param $which i32) (result i32) (local $p i32)
   local.get $which i32.eqz
   if
     i32.const ${texts[0].length} call $input local.tee $p i32.eqz if unreachable end
     local.get $p i32.const 0 i32.const ${texts[0].length*2} memory.init 0
   else
     i32.const ${texts[1].length} call $input local.tee $p i32.eqz if unreachable end
     local.get $p i32.const 0 i32.const ${texts[1].length*2} memory.init 1
   end
   call $parse)
 (func (export "roundtrip") (param i32) (result i32) (local $id i32)
   local.get 0 call $decode local.tee $id i32.eqz if unreachable end
   local.get $id call $stringify if unreachable end local.get $id)
 (func $add (param f64 f64) (result f64) local.get 0 local.get 1 f64.add)
 (func (export "sum") (param f64 f64) (result f64) local.get 0 local.get 1 call $add)
 (func (export "value") (result i32 f64) block (type $pair) i32.const 3 f64.const 42 end)
 (data "${data(texts[0])}") (data "${data(texts[1])}"))`;
const generated=path.join(out,'generated.wat'), oracle=path.join(out,'expected.tsv'), linked=path.join(out,'linked.wasm');
fs.writeFileSync(generated,wat);fs.writeFileSync(oracle,expected.map(hex).join('\n')+'\n');
run('cargo',['run','--offline','--locked','-j','2','-p','grunkalunka-wasm-link','--example','json_composition','--',
  path.join(target,'json-guest/module.wasm'),generated,linked,oracle]);
const bytes=fs.readFileSync(linked), module=new WebAssembly.Module(bytes);
assert.deepEqual(WebAssembly.Module.imports(module),[]);
const a=new WebAssembly.Instance(module).exports,b=new WebAssembly.Instance(module).exports;
assert(a.memory.buffer instanceof ArrayBuffer);
const units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
const read=e=>{
  const p=e.gk_json_output_ptr()>>>0,n=e.gk_json_output_len()>>>0;
  assert.equal(p%2,0);assert(n<=65536);assert(p+n*2<=e.memory.buffer.byteLength);
  return Array.from(new Uint16Array(e.memory.buffer,p,n));
};
let snapshot;
for(let i=0;i<500;i++) {
  const kind=i%2,id=a.roundtrip(kind)>>>0;assert.notEqual(id,0);assert.equal(a.gk_json_status(),0);
  const value=read(a);assert.deepEqual(value,units(expected[kind]));if(i===0)snapshot=value;
  assert.equal(a.gk_json_release(id),0);assert.equal(a.gk_json_output_len(),0);
}
for(const [x,y] of [[0,-0],[-0,-0],[41,1],[Infinity,-Infinity],[NaN,1]])assert(Object.is(a.sum(x,y),x+y));
assert.deepEqual(a.value(),[3,42]);assert.notEqual(b.roundtrip(1),0);assert.deepEqual(read(b),units(expected[1]));
assert.deepEqual(snapshot,units(expected[0]));
const sha256=crypto.createHash('sha256').update(bytes).digest('hex');
// Retained runtime exports must still pass the full original guest transport oracle.
run('cargo',['run','--offline','--locked','-j','2','-p','grunkalunka-runtime','--example','json_guest_check','--',
  linked,path.join(target,'json-guest/cases.tsv')]);
assert.equal(crypto.createHash('sha256').update(fs.readFileSync(linked)).digest('hex'),sha256);
const receipt={sha256,bytes:bytes.length,imports:0,sameArtifact:true,engines:[process.version,'Wasmtime50'],
  generatedLiteralSegments:2,repeatsPerEngine:500,numericCases:5,primitivePair:true,instanceIsolation:true,
  retainedRuntimeOracleRecords:1130,sourceLowering:false};
fs.writeFileSync(path.join(out,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log('PASS JSON composition '+JSON.stringify(receipt));
