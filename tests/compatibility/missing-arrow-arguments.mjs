export const missingArrowArguments = [
  ["missing-arrow-undefined", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>x===undefined?1:0;return f();}",
    sourceTS:"export function run(a,b){const f=x=>x===undefined?1:0;return f();}",
  }],
  ["missing-arrow-nullish-fallback", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=(x,y)=>y??x;return f(a);}",
    sourceTS:"export function run(a,b){const f=(x,y)=>y??x;return f(a);}",
  }],
  ["missing-arrow-numeric-coercion", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>+x;return f();}",
    sourceTS:"export function run(a,b){const f=x=>+x;return f();}",
  }],
  ["missing-arrow-call-effects", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>x===undefined?1:0;let n=0;const x=f.call((n++,a));return x*10+n;}",
    sourceTS:"export function run(a,b){const f=x=>x===undefined?1:0;let n=0;const x=f.call((n++,a));return x*10+n;}",
  }],
  ["missing-arrow-capture", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>{const g=()=>x===undefined?1:0;return g();};return f();}",
    sourceTS:"export function run(a,b){const f=x=>{const g=()=>x===undefined?1:0;return g();};return f();}",
  }],
  ["missing-arrow-mixed-calls", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>x===undefined?1:0;return f()*100+f(a)*10+f(NaN);}",
    sourceTS:"export function run(a,b){const f=x=>x===undefined?1:0;return f()*100+f(a)*10+f(NaN);}",
  }],
  ["missing-arrow-object-fallback", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=(x,y)=>y??x;const x=f({v:a});return +x.v;}",
    sourceTS:"export function run(a,b){const f=(x,y)=>y??x;const x=f({v:a});return +x.v;}",
  }],
  ["missing-arrow-factory", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x===undefined?1:0;const g=f();return g();}",
    sourceTS:"export function run(a,b){const f=x=>()=>x===undefined?1:0;const g=f();return g();}",
  }],
];
