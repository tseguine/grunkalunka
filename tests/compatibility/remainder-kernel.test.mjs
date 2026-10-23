// Standard: Number::remainder uses a truncated real quotient, not rounded f64 division.
// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-remainder
import assert from 'node:assert/strict';
import test from 'node:test';
import {spawnSync} from 'node:child_process';
import {mkdtempSync,readFileSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {remainderReference, numericRemainderPrograms} from './numeric-remainder.mjs';

const root=fileURLToPath(new URL('../../',import.meta.url));
const binary=join(root,'target','debug',`grunkalunka${process.platform==='win32'?'.exe':''}`);
const directory=mkdtempSync(join(root,'target','remainder-kernel-'));
const source=join(directory,'kernel.js'),wasm=join(directory,'kernel.wasm');
writeFileSync(source,'function number_remainder(x,y) { return x%y; } export function run(a,b) { return number_remainder(a,b); }');
const compiled=spawnSync(binary,['compile',source,wasm],{encoding:'utf8',timeout:30000});
assert.equal(compiled.error,undefined);assert.equal(compiled.status,0,compiled.stderr);
const module=new WebAssembly.Module(readFileSync(wasm));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
assert.deepEqual(WebAssembly.Module.exports(module),[{name:'run',kind:'function'}]);
const run=new WebAssembly.Instance(module).exports.run;
const view=new DataView(new ArrayBuffer(8));
const bits=n=>{view.setFloat64(0,n,false);return view.getBigUint64(0,false);};
const number=b=>{view.setBigUint64(0,b,false);return view.getFloat64(0,false);};
const hex=b=>b.toString(16).padStart(16,'0');
const SIGN=1n<<63n,ABS=SIGN-1n,INF=0x7ff0000000000000n,FRAC=(1n<<52n)-1n;
const isNaNBits=b=>(b&ABS)>INF;
function same(a,b,context){assert.ok(a===b||(isNaNBits(a)&&isNaNBits(b)),`${context}: ${hex(a)} != ${hex(b)}`);}

// Independently align both finite operands to units of 2^-1074 using BigInt.
// This deliberately avoids the helper's streaming significand-reduction algorithm.
function integer(b){const e=b>>52n;return ((b&FRAC)|(e===0n?0n:1n<<52n))<<(e===0n?0n:e-1n);}
function integerOracle(a,b){
  const x=a&ABS,y=b&ABS,sign=a&SIGN;
  if(x>=INF||y>INF||y===0n)return 0x7ff8000000000000n;
  if(y===INF)return a;
  const r=integer(x)%integer(y);
  if(r===0n)return sign;
  const shift=Math.max(0,r.toString(2).length-53);
  assert.equal(r&((1n<<BigInt(shift))-1n),0n,'exact integer remainder must be representable');
  const m=r>>BigInt(shift);
  return sign|(r<(1n<<52n)?r:((BigInt(shift+1)<<52n)|(m-(1n<<52n))));
}

test('production remainder preserves signed zero and real-quotient witnesses',()=>{
  for(const [a,b]of [[1,0.1],[-4,2],[Number.MAX_VALUE,Number.MIN_VALUE],[-0,3],[0,Infinity],
    [Infinity,2],[1,0],[1,NaN],[Number.MIN_VALUE*3,Number.MIN_VALUE*2]]){
    same(bits(run(a,b)),bits(a%b),`${a}%${b}`);
    same(integerOracle(bits(a),bits(b)),bits(a%b),'independent witness');
  }
});

test('promoted reference and finite compound snapshot retain independent expected results', async()=>{
  const reference=await import(`data:text/javascript;base64,${Buffer.from(remainderReference.oracleSource).toString('base64')}`);
  assert.equal(reference[remainderReference.entry](...remainderReference.args),remainderReference.expected);
  const source=join(directory,'snapshot.js'),output=join(directory,'snapshot.wasm');
  const fixture=numericRemainderPrograms.find(p=>p[0]==='remainder-assign-target-snapshot');
  writeFileSync(source,`export function run(a,b) { ${fixture[1]} }`);
  const result=spawnSync(binary,['compile',source,output],{encoding:'utf8',timeout:30000});
  assert.equal(result.status,0,result.stderr);
  const snapshot=new WebAssembly.Instance(new WebAssembly.Module(readFileSync(output))).exports.run;
  assert.equal(snapshot(0,0),22);
});

test('production raw binary64 kernel agrees with Node and independent BigInt in both Wasm engines',()=>{
  const positives=[0n,1n,2n,3n,7n,0x8000000000000n,FRAC-1n,FRAC,1n<<52n,(1n<<52n)+1n,
    0x3fefffffffffffffn,0x3ff0000000000000n,0x3ff0000000000001n,0x7fefffffffffffffn,
    INF,INF+1n,0x7ff8000000000000n,ABS,
    ...[0.1,0.2,0.3,1.5,2,3,5,10,2**32,2**53,Number.MIN_VALUE,Number.MAX_VALUE].map(bits)];
  const edges=[...new Set(positives.flatMap(x=>[x,x|SIGN]))];
  const pairs=edges.flatMap(a=>edges.map(b=>[a,b]));
  let state=0x243f6a8885a308d3n;
  function random(){state^=state<<13n;state^=state>>7n;state^=state<<17n;state=BigInt.asUintN(64,state);return state;}
  for(let i=0;i<20000;i++){
    let a=random(),b=random();
    if(i%7===0)b=edges[i%edges.length];
    if(i%11===0)a=(a&FRAC)|0x7fe0000000000000n;
    if(i%13===0)b=(b&FRAC)|1n;
    pairs.push([a,b]);
  }
  const expected=pairs.map(([a,b],i)=>{
    const want=bits(number(a)%number(b));
    same(integerOracle(a,b),want,`BigInt ${i}`);
    same(bits(run(number(a),number(b))),want,`Node-Wasm ${i}`);
    return want;
  });
  for(let offset=0;offset<pairs.length;offset+=4096){
    const batch=pairs.slice(offset,offset+4096);
    const input=batch.map(([a,b])=>`${hex(a)} ${hex(b)}`).join('\n')+'\n';
    const result=spawnSync(binary,['eval',wasm],{input,encoding:'utf8',timeout:30000,maxBuffer:1024*1024});
    assert.equal(result.error,undefined);assert.equal(result.status,0,result.stderr);
    const outputs=result.stdout.trim().split(/\r?\n/);
    assert.equal(outputs.length,batch.length);
    outputs.forEach((value,i)=>same(BigInt('0x'+value),expected[offset+i],`Wasmtime ${offset+i}`));
  }
  console.log(`PASS production remainder kernel: ${pairs.length} pairs, exact non-NaN bits, Node + BigInt + Node-Wasm + Wasmtime; artifacts ${directory}`);
});
