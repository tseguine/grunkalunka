export const managedFactoryResults = [
  ["managed-factory-result-undefined-number", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x;const g=f();return +g();}",
    sourceTS:"export function run(a,b){const f=x=>()=>x;const g=f();return +g();}",
  }],
  ["managed-factory-result-string", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x;const g=f('x');return g()==='x'?a:b;}",
    sourceTS:"export function run(a,b){const f=x=>()=>x;const g=f('x');return g()==='x'?a:b;}",
  }],
  ["managed-factory-result-object", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x;const g=f({v:a});return +g().v;}",
    sourceTS:"export function run(a,b){const f=x=>()=>x;const g=f({v:a});return +g().v;}",
  }],
  ["managed-factory-result-array", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x;const g=f([a]);return +g()[0];}",
    sourceTS:"export function run(a,b){const f=x=>()=>x;const g=f([a]);return +g()[0];}",
  }],
  ["managed-factory-result-effects", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){let n=0;const f=x=>()=>{n++;return x;};const g=f();return +(g()??b)+n;}",
    sourceTS:"export function run(a,b){let n=0;const f=x=>()=>{n++;return x;};const g=f();return +(g()??b)+n;}",
  }],
  ["managed-factory-result-mutation", '', '', undefined, undefined, {
    sourceJS:"export function run(a,b){const f=x=>()=>x;const o={v:a};const g=f(o);o.v=b;return +g().v;}",
    sourceTS:"export function run(a,b){const f=x=>()=>x;const o={v:a};const g=f(o);o.v=b;return +g().v;}",
  }],
];
