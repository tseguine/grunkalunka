// Synchronous source closures retain traced bindings; numeric observations only.
const cases = [
  ['managed-capture-object-rebind', 'let o={x:a};const old=o;let f=()=>o===old?a:b;o={x:b};return f();'],
  ['managed-capture-object-property', 'const o={x:a};const old=o.x;let f=()=>{o.x=b;return 0;};f();return old===a&&o.x===b?a:b;'],
  ['managed-capture-array-rebind', 'let x=[a];const old=x;let f=()=>{x=[b];return old[0]===a?a:b;};const r=f();return x!==old&&x[0]===b?r:b;'],
  ['managed-capture-array-cycle', 'const x=[];x[0]=x;let f=()=>x[0]===x?a:b;return f();'],
  ['managed-capture-string-rebind', "let text='old';let f=()=>text==='old'?a:b;text='new';return f();"],
  ['managed-capture-string-write', "let text='old';let f=()=>{text='new';return 0;};f();return text==='new'?a:b;"],
  ['managed-capture-value-kind', "const o={x:a};let v=o.x;let f=()=>v===a?a:b;v='changed';return f();"],
  ['managed-capture-closure-store', 'let o={x:a};const old=o;let set=()=>{o={x:b};return o===old?1:0;};set();return o===old?a:b;'],
  ['managed-capture-receiver', 'let o={x:a};const old=o;const choose=()=>{o={x:b};return a;};old.x=choose();return old!==o&&old.x===a?a:b;'],
  ['managed-capture-selected', 'let o={x:a};const old=o;let f=a?()=>o===old?a:b:()=>o!==old?b:a;o={x:b};return f();'],
  ['managed-capture-scalar-sharing', 'let n=a;const o={x:b};const one=()=>{n++;o.x=n;return 0;};const two=()=>{n--;return n;};let f=a?one:two;f();return o.x===n?a:b;'],
  ['managed-capture-general-sharing', 'let p=null;const o={};let f=a?()=>{p=true;return o===o?1:0;}:()=>{p=false;return o===o?1:0;};f();return p===true?a:b;'],
  ['managed-capture-factory-object', 'let make=n=>{const o={x:n};return ()=>o.x===n?a:b;};const f=make(a);const g=make(b);return f()+g();'],
  ['managed-capture-factory-array', 'let make=n=>{const x=[n];return ()=>x[0]===n?a:b;};const f=make(a);const g=make(b);return f()+g()+f();'],
  ['managed-capture-factory-string', "let make=n=>{let text='old';return ()=>{const old=text;text='new';return old==='old'?n:b;};};const f=make(a);return f()+f();"],
  ['managed-capture-factory-shared-object', 'const o={x:a};let make=n=>()=>{o.x=n;return o.x===n?a:b;};const f=make(a);const g=make(b);return f()+g()+f();'],
  ['managed-capture-callback', 'const o={x:a};const f=()=>o.x===b?a:b;const apply=(fn,x)=>{o.x=x;return fn();};return apply(f,b);'],
  ['managed-capture-selected-callback', 'const o={x:a};const apply=a?(fn,x)=>{o.x=x;return fn();}:(fn,x)=>{o.x=x;return fn();};return apply(()=>o.x===b?a:b,b);'],
  ['managed-capture-iteration', 'let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const o={x:i};if(i===0)first=()=>o.x===0?a:b;last=()=>o.x===2?b:a;}return first()+last();'],
  ['managed-capture-extra-args', 'let o={};let f=()=>o===o?a:b;return f((o={},a),(o={},b));'],
  ['managed-capture-former-object-guard', 'const o={};const f=()=>o?a:b;return f();'],
  ['managed-capture-former-array-guard', 'const x=[a];const f=()=>x.length;return f();'],
  ['managed-capture-former-factory-guard', 'const x=[a];const f=()=>()=>x.length;return f()();'],
];
cases.push(
  ['managed-capture-typeof-property', "const o={x:a};let f=()=>typeof o.x==='number'?a:b;return f();"],
  ['managed-capture-typeof-function', "const o={};let f=()=>o===o?a:b;let g=()=>typeof f==='function'?f():b;return g();"],
);
export const managedCaptures=cases.map(([id,body])=>[id,body,body]);
managedCaptures.push(['managed-capture-erasure','','',undefined,undefined,{
  sourceJS:"export function run(a,b){let o={x:a};let f=()=>o.x===a?a:b;o={x:b};return f();}",
  sourceTS:"export function run(a:string,b:boolean):void{let o:number={x:a};let f=():object=>o.x===(a as number)?a:b;o=({x:b} satisfies string);return (f as Function)();}",
}]);
