export const missingRuntimeArguments = [
  ["missing-runtime-selection", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);return f();}",
    sourceTS:"export function run(a,b){const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);return f();}",
  }],
  ["missing-runtime-mutable", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){let f=x=>x===undefined?1:0;return f();}",
    sourceTS:"export function run(a,b){let f=x=>x===undefined?1:0;return f();}",
  }],
  ["missing-runtime-fallback", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=a?((x,y)=>y??x):((x,y)=>y??x+1);return f(b);}",
    sourceTS:"export function run(a,b){const f=a?((x,y)=>y??x):((x,y)=>y??x+1);return f(b);}",
  }],
  ["missing-runtime-call-effects", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);let n=0;const r=f.call((n++,a));return r*10+n;}",
    sourceTS:"export function run(a,b){const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);let n=0;const r=f.call((n++,a));return r*10+n;}",
  }],
  ["missing-runtime-object-field", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const o={f:x=>x===undefined?1:0};return o.f();}",
    sourceTS:"export function run(a,b){const o={f:x=>x===undefined?1:0};return o.f();}",
  }],
  ["missing-runtime-array-field", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const fs=[x=>x===undefined?1:0];return fs[0]();}",
    sourceTS:"export function run(a,b){const fs=[x=>x===undefined?1:0];return fs[0]();}",
  }],
  ["missing-runtime-factory", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const make=x=>{return y=>y===undefined?x:0;};return (a?make(a):make(b))();}",
    sourceTS:"export function run(a,b){const make=x=>{return y=>y===undefined?x:0;};return (a?make(a):make(b))();}",
  }],
  ["missing-runtime-callee-snapshot", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){let f=(x,y)=>y===undefined?x:0;const old=f;const r=f((f=(x,y)=>0,a));return r+old(b);}",
    sourceTS:"export function run(a,b){let f=(x,y)=>y===undefined?x:0;const old=f;const r=f((f=(x,y)=>0,a));return r+old(b);}",
  }],
];
