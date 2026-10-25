// This dedicated guest check uses only installed toolchains and cached packages.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {checkHeapFrames} from '../tests/compatibility/heap-frames.mjs';
import {checkHeapJson} from '../tests/compatibility/heap-json.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
const run=(command,args)=>{
  const r=spawnSync(command,args,{cwd:root,stdio:'inherit',shell:false});
  if(r.error)throw r.error;
  assert.equal(r.status,0,`${command} failed: ${r.signal??r.status}`);
};
run('cargo',['build','--locked','--offline','-j','2','-p','grunkalunka-json-guest','--target','wasm32-unknown-unknown','--release']);
const target=path.resolve(root,process.env.CARGO_TARGET_DIR??'target');
const wasm=fs.readFileSync(path.join(target,'wasm32-unknown-unknown/release/grunkalunka_json_guest.wasm'));
const module=new WebAssembly.Module(wasm);
const heapFrames=checkHeapFrames(new WebAssembly.Instance(module).exports);
assert.deepEqual(WebAssembly.Module.imports(module),[]);
const guest=new WebAssembly.Instance(module).exports;
assert(guest.memory.buffer instanceof ArrayBuffer, 'guest memory must be nonshared');
const ptr=guest.gk_json_input_resize(4);
assert.notEqual(ptr,0,'valid input preparation must return a usable buffer');
assert.deepEqual(Array.from(new Uint16Array(guest.memory.buffer,ptr,4)),[0,0,0,0]);
new Uint16Array(guest.memory.buffer,ptr,4).set([110,117,108,108]);
const nullId=guest.gk_json_parse();
assert.notEqual(nullId,0,'valid JSON must produce an owned root ID');
assert.equal(guest.gk_json_input_resize(32769),0);
assert.equal(guest.gk_json_status(),1,'oversized input must report its specific error');
assert.equal(guest.gk_json_input_len(),0);
assert.equal(guest.gk_json_stringify(nullId),0);
assert.equal(guest.gk_json_status(),0);
assert.equal(guest.gk_json_release(nullId),0);

const outDir=path.join(target,'json-guest');
fs.mkdirSync(outDir,{recursive:true});
const modulePath=path.join(outDir,'module.wasm');
fs.writeFileSync(modulePath,wasm);
const units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
const hex=s=>units(s).map(u=>u.toString(16).padStart(4,'0')).join('');
const prepare=(e,text)=>{
  const p=e.gk_json_input_resize(text.length)>>>0;
  assert.notEqual(p,0);assert.equal(p%2,0);assert.equal(e.gk_json_status(),0);
  assert.equal(e.gk_json_input_len(),text.length);
  assert(p+text.length*2<=e.memory.buffer.byteLength);
  new Uint16Array(e.memory.buffer,p,text.length).set(units(text));
};
const read=e=>{
  const p=e.gk_json_output_ptr()>>>0,n=e.gk_json_output_len()>>>0;
  assert.equal(p%2,0);assert(n<=65536);assert(p+n*2<=e.memory.buffer.byteLength);
  return Array.from(new Uint16Array(e.memory.buffer,p,n));
};
const cases=[];
const valid=text=>cases.push(['valid',text,JSON.stringify(JSON.parse(text))]);
for(const text of ['null','false','true','-0','0','1e400','-1e400','1e-400','5e-324',
  '""','[]','{}','[1,null,false,"s",[],{}]',
  '{"z":1,"2":true,"1":false,"z":2,"__proto__":{"x":1}}',
  '{"4294967295":1,"4294967294":2,"01":3,"-0":4,"0":5}',
  '"\\ud800"','"\\udc00"','"\\ud800\\udc00"','"\ud800x\udfff"']) valid(text);
for(let start=0;start<65536;start+=1024) {
  valid(JSON.stringify(String.fromCharCode(...Array.from({length:1024},(_,i)=>start+i))));
}
let seed=0x4a534f4e5f574153n;
const bits=Buffer.alloc(8);
for(let i=0;i<1024;i++) {
  seed=BigInt.asUintN(64,seed*6364136223846793005n+1442695040888963407n);
  bits.writeBigUInt64LE(seed);const n=bits.readDoubleLE();
  if(Number.isFinite(n))valid(String(n));
}
valid('null'+' '.repeat(32764));
valid('['.repeat(128)+'0'+']'.repeat(128));
valid('['+Array(4095).fill('0').join(',')+']');
valid('"'+'\ud800'.repeat(10922)+'aa"'); // Exactly 65,536 output units.
for(const text of ['', ' ', 'NaN','undefined','01','1.','1e','[0,]','{"x":1,}',
  '{x:1}','"\\q"','"\u0001"','"\\uZZZZ"','"missing-end','[true false]']) {
  assert.throws(()=>JSON.parse(text));cases.push(['invalid',text,'5']);
}
cases.push(['parse-limit','['.repeat(129)+'0'+']'.repeat(129),'6']);
cases.push(['parse-limit','['+Array(4096).fill('0').join(',')+']','6']);
cases.push(['parse-limit','{'+Array.from({length:650},(_,i)=>`"k${String(i).padStart(5,'0')}":0`).join(',')+'}','6']);
cases.push(['output-limit','"'+'\ud800'.repeat(10922)+'aaa"','7']);
let validCount=0;
for(const [kind,text,want] of cases) {
  prepare(guest,text);const id=guest.gk_json_parse()>>>0;
  if(kind==='invalid'||kind==='parse-limit') {
    assert.equal(id,0);assert.equal(guest.gk_json_status(),Number(want));
    assert(guest.gk_json_error_offset()<=text.length);
  } else {
    assert.notEqual(id,0);assert.equal(guest.gk_json_status(),0);
    assert.equal(guest.gk_json_stringify(id),kind==='valid'?0:Number(want));
    if(kind==='valid'){assert.deepEqual(read(guest),units(want));validCount++;}
    else assert.deepEqual(read(guest),[]);
    assert.equal(guest.gk_json_release(id),0);
    assert.equal(guest.gk_json_stringify(id),4);assert.deepEqual(read(guest),[]);
  }
  assert.equal(guest.gk_json_output_len(),0);
}
const heapJson=checkHeapJson(new WebAssembly.Instance(module).exports,cases);
const sample='{"s":"\\ud800","n":-0}';
prepare(guest,sample);const keep=guest.gk_json_parse()>>>0;
assert.equal(guest.gk_json_stringify(keep),0);const snapshot=read(guest);
for(let i=0;i<500;i++) {
  prepare(guest,sample);const id=guest.gk_json_parse()>>>0;
  assert.notEqual(id,keep);assert.equal(guest.gk_json_stringify(id),0);
  assert.deepEqual(read(guest),snapshot);assert.equal(guest.gk_json_release(id),0);
}
assert.equal(guest.gk_json_stringify(keep),0);assert.deepEqual(read(guest),snapshot);
assert.equal(guest.gk_json_release(keep),0);
prepare(guest,'0');const ids=Array.from({length:64},()=>guest.gk_json_parse()>>>0);
assert(ids.every(id=>id!==0));assert.equal(new Set(ids).size,64);
assert.equal(guest.gk_json_parse(),0);assert.equal(guest.gk_json_status(),2);
assert.equal(guest.gk_json_release(ids[0]),0);const replacement=guest.gk_json_parse()>>>0;
assert(replacement>Math.max(...ids));assert.equal(guest.gk_json_release(ids[0]),4);
for(const id of [...ids.slice(1),replacement])assert.equal(guest.gk_json_release(id),0);
for(const n of [32769,0xffffffff]) {
  assert.equal(guest.gk_json_input_resize(n),0);assert.equal(guest.gk_json_status(),1);
  assert.equal(guest.gk_json_input_len(),0);assert.equal(guest.gk_json_parse(),0);
  assert.equal(guest.gk_json_status(),5);
}
const other=new WebAssembly.Instance(module).exports;
prepare(other,'true');const otherId=other.gk_json_parse()>>>0;
prepare(guest,'false');const ownId=guest.gk_json_parse()>>>0;
assert.equal(other.gk_json_stringify(otherId),0);assert.deepEqual(read(other),units('true'));
assert.equal(guest.gk_json_stringify(ownId),0);assert.deepEqual(read(guest),units('false'));
assert.deepEqual(snapshot,units(JSON.stringify(JSON.parse(sample))));
// Raw String transport is distinct from parsing JSON text; retain the original oracle above.
const stringCases=['','\ud800','\udc00','\ud800\udc00','\0"\\\n\ud800\0\udfff'];
for(let start=0;start<65536;start+=32768) {
  stringCases.push(Array.from({length:32768},(_,i)=>String.fromCharCode(start+i)).join(''));
}
assert.equal(stringCases.length,7);
const seen=new Uint8Array(65536), strings=new WebAssembly.Instance(module).exports;
for(const text of stringCases) {
  for(const u of units(text))seen[u]=1;
  prepare(strings,text);const id=strings.gk_json_string_from_input()>>>0;
  assert.notEqual(id,0);assert.equal(strings.gk_json_status(),0);assert.equal(strings.gk_json_error_offset(),0);
  assert.equal(strings.gk_json_string_units(id),0);assert.deepEqual(read(strings),units(text));
  const copy=read(strings);prepare(strings,'overwrite');
  assert.equal(strings.gk_json_string_units(id),0);assert.deepEqual(read(strings),copy);
  assert.equal(strings.gk_json_stringify(id),0);assert.deepEqual(read(strings),units(JSON.stringify(text)));
  assert.equal(strings.gk_json_release(id),0);assert.equal(strings.gk_json_string_units(id),4);
  assert.deepEqual(read(strings),[]);assert.deepEqual(copy,units(text));
}
assert(seen.every(x=>x===1));
prepare(strings,'"\\ud800"');const parsedString=strings.gk_json_parse()>>>0;
assert.notEqual(parsedString,0);assert.equal(strings.gk_json_string_units(parsedString),0);
assert.deepEqual(read(strings),[0xd800]);assert.equal(strings.gk_json_release(parsedString),0);
prepare(strings,'[raw\ud800]');const stringKeep=strings.gk_json_string_from_input()>>>0;
assert.notEqual(stringKeep,0);assert.equal(strings.gk_json_string_units(stringKeep),0);
const stringCopy=read(strings);
prepare(strings,'null');const nonString=strings.gk_json_parse()>>>0;assert.notEqual(nonString,0);
assert.equal(strings.gk_json_string_units(stringKeep),0);
assert.equal(strings.gk_json_string_units(nonString),9);assert.deepEqual(read(strings),[]);
assert.equal(strings.gk_json_error_offset(),0);assert.equal(strings.gk_json_status(),9);
assert.equal(strings.gk_json_string_units(0),4);assert.equal(strings.gk_json_error_offset(),0);
assert.equal(strings.gk_json_string_units(0xffffffff),4);assert.equal(strings.gk_json_release(nonString),0);
for(let i=0;i<500;i++) {
  prepare(strings,'[raw\ud800]');const id=strings.gk_json_string_from_input()>>>0;
  assert.notEqual(id,0);assert.notEqual(id,stringKeep);assert.equal(strings.gk_json_string_units(id),0);
  assert.deepEqual(read(strings),stringCopy);assert.equal(strings.gk_json_release(id),0);
}
for(const n of [32769,0xffffffff]) {
  assert.equal(strings.gk_json_input_resize(n),0);assert.equal(strings.gk_json_status(),1);
  const empty=strings.gk_json_string_from_input()>>>0;assert.notEqual(empty,0);
  assert.equal(strings.gk_json_status(),0);assert.equal(strings.gk_json_error_offset(),0);
  assert.equal(strings.gk_json_string_units(empty),0);assert.deepEqual(read(strings),[]);
  assert.equal(strings.gk_json_release(empty),0);
}
prepare(strings,'\ud800'.repeat(12000));const expanding=strings.gk_json_string_from_input()>>>0;
assert.notEqual(expanding,0);assert.equal(strings.gk_json_stringify(expanding),7);assert.deepEqual(read(strings),[]);
assert.equal(strings.gk_json_string_units(expanding),0);assert.deepEqual(read(strings),units('\ud800'.repeat(12000)));
assert.equal(strings.gk_json_release(expanding),0);
const stringOther=new WebAssembly.Instance(module).exports;
prepare(stringOther,'\udfff');const stringOtherId=stringOther.gk_json_string_from_input()>>>0;
assert.notEqual(stringOtherId,0);assert.equal(stringOther.gk_json_string_units(stringOtherId),0);
assert.deepEqual(read(stringOther),[0xdfff]);assert.equal(strings.gk_json_string_units(stringKeep),0);
assert.deepEqual(read(strings),stringCopy);assert.equal(strings.gk_json_release(stringKeep),0);
assert.equal(stringOther.gk_json_release(stringOtherId),0);
prepare(strings,'0');const mixedIds=Array.from({length:64},(_,i)=>
  (i%2===0?strings.gk_json_string_from_input():strings.gk_json_parse())>>>0);
assert(mixedIds.every(id=>id!==0));assert.equal(new Set(mixedIds).size,64);
assert.equal(strings.gk_json_string_units(mixedIds[0]),0);
assert.equal(strings.gk_json_string_from_input(),0);assert.equal(strings.gk_json_status(),2);
assert.deepEqual(read(strings),[]);assert.equal(strings.gk_json_parse(),0);assert.equal(strings.gk_json_status(),2);
assert.equal(strings.gk_json_release(mixedIds[0]),0);
const mixedNext=strings.gk_json_string_from_input()>>>0;assert(mixedNext>Math.max(...mixedIds));
assert.equal(strings.gk_json_string_units(mixedIds[0]),4);
for(const id of [...mixedIds.slice(1),mixedNext])assert.equal(strings.gk_json_release(id),0);
const stringOraclePath=path.join(outDir,'string-cases.tsv');
fs.writeFileSync(stringOraclePath,stringCases.map(text=>`${hex(text)}\t${hex(JSON.stringify(text))}`).join('\n')+'\n');
const oraclePath=path.join(outDir,'cases.tsv');
fs.writeFileSync(oraclePath,cases.map(([kind,text,want])=>`${kind}\t${hex(text)}\t${kind==='valid'?hex(want):want}`).join('\n')+'\n');
const sha256=crypto.createHash('sha256').update(wasm).digest('hex');
run('cargo',['run','--locked','--offline','-j','2','-p','grunkalunka-runtime','--example','json_guest_check','--',modulePath,oraclePath]);
assert.equal(crypto.createHash('sha256').update(fs.readFileSync(modulePath)).digest('hex'),sha256);
const receipt={sha256,bytes:wasm.length,imports:0,node:process.version,wasmtime:'50.0.0',
  sameArtifact:true,cases:cases.length,validCount,allUtf16Units:65536,repeatsPerEngine:500,
  liveRoots:64,releasedIdsNeverReused:true,instanceIsolation:true,ownedSnapshots:true,
  rawStringCases:stringCases.length,rawStringAllUtf16Units:65536,rawStringRepeatsPerEngine:500,heapFrames,heapJson};
fs.writeFileSync(path.join(outDir,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log('PASS JSON guest '+JSON.stringify(receipt));
