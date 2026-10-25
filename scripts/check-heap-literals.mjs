// Source counterexamples for the experimental heap-return profile.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import vm from 'node:vm';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const root=fileURLToPath(new URL('../',import.meta.url));
const target=path.resolve(root,process.env.CARGO_TARGET_DIR??'target');
fs.mkdirSync(target,{recursive:true});
const out=fs.mkdtempSync(path.join(target,'heap-literals-'));
function run(command,args){
  const result=spawnSync(command,args,{cwd:root,stdio:'inherit',shell:false});
  if(result.error)throw result.error;
  assert.equal(result.status,0);
}
const cases=[
  ['empty','return "";'],
  ['raw','return "A\\ud800\\0";'],
  ['wrapped','return ("wrapped");','return ("wrapped" as unknown as number);'],
];
for(const [name,body,tsBody=body] of cases)for(const language of ['js','ts']){
  const parameters=language==='js'?'a,b':'a:number,b:number';
  fs.writeFileSync(path.join(out,name+'.'+language),'export function run('+parameters+'){'+(language==='ts'?tsBody:body)+'}');
}
run('cargo',['build','--locked','--offline','-j','2','-p','grunkalunka-json-guest','--target','wasm32-unknown-unknown','--release']);
run('cargo',['run','--locked','--offline','-j','2','-p','grunkalunka-compiler','--example','heap_literals','--',out]);
const bytes=fs.readFileSync(path.join(target,'wasm32-unknown-unknown/release/grunkalunka_json_guest.wasm'));
const guestModule=new WebAssembly.Module(bytes);
let calls=0;
for(const [name,body] of cases)for(const language of ['js','ts']){
  const expected=vm.runInNewContext('function run(a,b){'+body+'};run(1,2)');
  const units=Array.from({length:expected.length},(_,index)=>expected.charCodeAt(index));
  const guest=new WebAssembly.Instance(guestModule).exports;
  const module=new WebAssembly.Module(fs.readFileSync(path.join(out,name+'.'+language+'.wasm')));
  const generated=new WebAssembly.Instance(module,{runtime:guest}).exports;
  for(let repeat=0;repeat<500;repeat++){
    const [tag,payload]=generated.run(1,2);
    assert.equal(tag,4);assert(Number.isInteger(payload)&&payload>0&&payload<=0xffffffff);
    assert.equal(guest.gk_json_string_units(payload),0);
    const ptr=guest.gk_json_output_ptr()>>>0,length=guest.gk_json_output_len()>>>0;
    const actual=Array.from(new Uint16Array(guest.memory.buffer,ptr,length));
    assert.deepEqual(actual,units,name+'.'+language);
    assert.equal(guest.gk_json_release(payload),0);
    calls++;
  }
}
const receipt={cases:cases.length*2,calls,engine:process.version,
  guestSha256:crypto.createHash('sha256').update(bytes).digest('hex'),scope:'direct module imports; Node source literal examples'};
fs.writeFileSync(path.join(out,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log('PASS heap literals '+JSON.stringify(receipt));
