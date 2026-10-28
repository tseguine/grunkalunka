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
function check(generated,guest,units,name){
  for(let repeat=0;repeat<500;repeat++){
    const [tag,payload]=generated.run(1,2);
    assert.equal(tag,4);assert(Number.isInteger(payload)&&payload>0&&payload<=0xffffffff);
    assert.equal(guest.gk_json_string_units(payload),0);
    const ptr=guest.gk_json_output_ptr()>>>0,length=guest.gk_json_output_len()>>>0;
    const actual=Array.from(new Uint16Array(guest.memory.buffer,ptr,length));
    assert.deepEqual(actual,units,name);
    assert.equal(guest.gk_json_release(payload),0);
  }
  return 500;
}
const cases=[
  ['empty','return "";'],
  ['raw','return "A\\ud800\\0";'],
  ['wrapped','return ("wrapped");','return ("wrapped" as unknown as number);'],
  ['json-string','return JSON.stringify("");'],
  ['json-raw-string','return JSON.stringify("A\\ud800\\0");'],
  ['json-wrapped','return JSON.stringify(("wrapped"));',
    'return (JSON.stringify(("wrapped" as unknown as number)) as unknown as number);'],
  ['json-type-string','return JSON.stringify("A\\ud800\\0");',
    'return JSON.stringify<Missing>("A\\ud800\\0");'],
  ['json-type-nested','return JSON.stringify(JSON.parse(\'{"x":[1,true,null]}\'));',
    'return JSON.stringify<never>(JSON.parse<string>(\'{"x":[1,true,null]}\'));'],
  ['json-type-chain','return JSON.stringify(JSON.parse(JSON.stringify("x")));',
    'return JSON.stringify<Missing,Other>(JSON.parse<never>(JSON.stringify<typeof absent>("x")));'],
];
for(const [name,text] of [
  ['null','null'],['boolean','true'],['number','-0'],['overflow','1e400'],
  ['array','[1,true,null,"text",[2]]'],
  ['object','{"z":1,"10":2,"2":3,"z":4,"__proto__":{"x":5}}'],
  ['unicode','"\\ud800\\u0000\\udfff"'],
]){
  cases.push(['json-'+name,'return JSON.stringify(JSON.parse('+JSON.stringify(text)+'));']);
}
let repeated=JSON.stringify('null');
for(let i=0;i<35;i++)repeated='JSON.stringify(JSON.parse('+repeated+'))';
cases.push(['json-chain','return '+repeated+';']);
for(const [name,body,tsBody=body] of cases)for(const language of ['js','ts']){
  const parameters=language==='js'?'a,b':'a:number,b:number';
  fs.writeFileSync(path.join(out,name+'.'+language),'export function run('+parameters+'){'+(language==='ts'?tsBody:body)+'}');
}
run('cargo',['build','--locked','--offline','-j','2','-p','grunkalunka-json-guest','--target','wasm32-unknown-unknown','--release']);
run('cargo',['run','--locked','--offline','-j','2','-p','grunkalunka-compiler','--example','heap_literals','--',out]);
for(const [name] of cases){
  assert.deepEqual(fs.readFileSync(path.join(out,name+'.ts.wasm')),
    fs.readFileSync(path.join(out,name+'.js.wasm')),name+' JS/TS guest bytes');
}
const bytes=fs.readFileSync(path.join(target,'wasm32-unknown-unknown/release/grunkalunka_json_guest.wasm'));
const guestModule=new WebAssembly.Module(bytes);
let calls=0;
const oracles=[];
for(const [name,body] of cases)for(const language of ['js','ts']){
  const expected=vm.runInNewContext('function run(a,b){'+body+'};run(1,2)');
  const units=Array.from({length:expected.length},(_,index)=>expected.charCodeAt(index));
  oracles.push(name+'.'+language+'\t'+units.map(unit=>unit.toString(16).padStart(4,'0')).join(''));
  const guest=new WebAssembly.Instance(guestModule).exports;
  const module=new WebAssembly.Module(fs.readFileSync(path.join(out,name+'.'+language+'.wasm')));
  const generated=new WebAssembly.Instance(module,{runtime:guest}).exports;
  calls+=check(generated,guest,units,name+'.'+language);
}
fs.writeFileSync(path.join(out,'oracles.tsv'),oracles.join('\n')+'\n');
run('cargo',['run','--locked','--offline','-j','2','-p','grunkalunka-runtime','--example','heap_literals_check','--',
  path.join(target,'wasm32-unknown-unknown/release/grunkalunka_json_guest.wasm'),out]);
const rustReceipt=JSON.parse(fs.readFileSync(path.join(out,'wasmtime-receipt.json'),'utf8'));
assert.equal(rustReceipt.cases,cases.length*2);assert.equal(rustReceipt.calls,calls);
let composedCalls=0;
const artifacts=[];
for(const row of oracles){
  const [name,hex]=row.split('\t');
  const units=(hex.match(/.{4}/g)??[]).map(value=>parseInt(value,16));
  const composed=fs.readFileSync(path.join(out,name+'.composed.wasm'));
  const module=new WebAssembly.Module(composed);
  assert.equal(WebAssembly.Module.imports(module).length,0);
  const instance=new WebAssembly.Instance(module).exports;
  composedCalls+=check(instance,instance,units,name+' composed');
  artifacts.push({name,composedSha256:crypto.createHash('sha256').update(composed).digest('hex')});
}
const receipt={cases:cases.length*2,nodeDirectCalls:calls,nodeComposedCalls:composedCalls,
  wasmtimeOwnedCalls:rustReceipt.calls,engine:process.version,
  guestSha256:crypto.createHash('sha256').update(bytes).digest('hex'),artifacts};
fs.writeFileSync(path.join(out,'receipt.json'),JSON.stringify(receipt,null,2)+'\n');
console.log('PASS heap literals '+JSON.stringify({...receipt,artifacts:artifacts.length}));
