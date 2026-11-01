// Original-source own-field calls retain the selected function and its effects.
const cases = [
  ['object', 'const f=()=>a;const o={f};return o.f();'],
  ['array', 'const o=[()=>a];return o[0]();'],
  ['number', 'const o={f:x=>x};return o.f(a);'],
  ['alias', 'const o={f:x=>x};const g=o.f;return g(a);'],
  ['write-alias', 'const o={f:()=>a};const p=o;p.f=()=>b;return o.f();'],
  ['snapshot', 'const o={f:()=>a};const old=o.f;o.f=()=>b;return old()+o.f();'],
  ['argument-snapshot', 'const o={f:x=>x};return o.f((o.f=x=>b,a));'],
  ['receiver-effects', 'let n=0;const o={f:x=>x};return (n++,o).f((n++,a),(n++,b))+n;'],
  ['selected-receiver', 'const o={f:()=>a};const p={f:()=>b};return (a?o:p).f();'],
  ['object-input', 'const o={f:x=>x};const value={};return o.f(value)===value?1:0;'],
  ['array-input', 'const o={f:x=>x};const value=[a,b];return o.f(value)===value?1:0;'],
  ['string-input', "const o={f:x=>x};return o.f('text')==='text'?1:0;"],
  ['boolean-input', 'const o={f:x=>x};return o.f(true)===true?1:0;'],
  ['null-input', 'const o={f:x=>x};return o.f(null)===null?1:0;'],
  ['undefined-input', 'const o={f:x=>x};return o.f(undefined)===undefined?1:0;'],
  ['callback', 'const o={f:fn=>fn(a)};const g=x=>x;return o.f(g);'],
  ['returned-function', 'const o={f:()=>()=>a};const g=o.f();return g();'],
  ['factory', 'const make=()=>({f:()=>a});const o=make();return o.f();'],
  ['factory-argument', 'const make=fn=>({f:fn});const o=make(()=>a);return o.f();'],
  ['returned-factory', 'const make=x=>()=>x;const o={f:make(a)};return o.f();'],
  ['helper-receiver', 'const apply=o=>o.f();const o={f:()=>a};return apply(o);'],
  ['captured-input', 'const make=n=>({f:()=>n});const o=make(a),p=make(b);return o.f()+p.f();'],
  ['reference-environment', 'const value={};const make=x=>({f:()=>x});const o=make(value);return o.f()===value?1:0;'],
  ['call-receiver', 'let n=0;const o={f:x=>x};return o.f.call((n++,b),(n++,a),(n++,b))+n;'],
  ['typeof-effects', "let n=0;const o={f:()=>a};const yes=typeof(n++,o).f==='function';return yes?n:a;"],
  ['unicode', "const o={'😀':x=>x};return o['😀'](a);"],
  ['duplicate-key', 'const o={f:()=>b,f:()=>a};return o.f();'],
  ['helper-write', 'const o={f:()=>a};const change=x=>{x.f=()=>b;return x;};change(o);return o.f();'],
  ['array-growth', 'const o=[()=>a];o.length=3;return o[0]();'],
  ['cycle', 'const state={x:a};const f=()=>state.x===b?1:0;const o={f};state.o=o;state.self=state;state.x=b;return o.f();'],
  ['receiver-replacement', 'let o={f:x=>x};return o.f((o={f:x=>b},a));'],
  ['factory-call', 'let n=0;const make=fn=>({f:fn});const o=make.call((n++,b),(n++,()=>a),(n++,b));return o.f()+n;'],
  ['surplus-write', 'const o={f:()=>a};return o.f((o.f=()=>b,b))+o.f();'],
  ['mixed-inputs', 'const o={f:x=>x};const value={};return o.f(value)===value&&o.f(a)===a?1:0;'],
  ['mixed-boolean', 'const o={f:x=>x};const value={};return o.f(value)===value&&o.f(true)===true?1:0;'],
  ['object-result', 'const o={f:()=>({x:a})};return o.f().x===a?1:0;'],
  ['array-result', 'const o={f:()=>[a,b]};return o.f()[0]===a?1:0;'],
  ['string-result', "const o={f:()=>'text'};return o.f()==='text'?1:0;"],
];
export const storedFunctionCalls=cases.map(([id,body])=>[`stored-function-${id}`,body,body]);
storedFunctionCalls.push(['stored-function-module-input','','',undefined,undefined,{
  sourceJS:'function id(x){return x;}export function run(a,b){const o={id};const x={};return o.id(x)===x?1:0;}',
  sourceTS:'function id<T>(x:boolean):string{return x;}export function run(a:string,b:never):void{const o={id};const x={};return o.id<Missing>(x as number)===x?1:0;}',
}]);
storedFunctionCalls.push(['stored-function-module-factory','','',undefined,undefined,{
  sourceJS:'function make(n){return {f:()=>n};}export function run(a,b){const o=make(a);return o.f();}',
  sourceTS:'function make<T>(n:boolean):string{return {f:()=>n};}export function run(a:string,b:boolean):never{const o=make<Missing>(a);return o.f();}',
}]);
storedFunctionCalls.push(['stored-function-erasure','','',undefined,undefined,{
  sourceJS:'export function run(a,b){const o=[()=>a];o.length=3;return o[0]();}',
  sourceTS:"type Unused='metadata';export function run(a:string,b:boolean):void{const o=[()=>a];o.length=(3 as unknown)!;return o[(0 as never)]();}",
}]);
storedFunctionCalls.push(['stored-function-former-module-callback','','',undefined,undefined,{
  sourceJS:'function id(x){return x;}export function run(a,b){const o={id};const apply=(fn,x)=>fn(x);return apply(o.id,{})===o?1:0;}',
  sourceTS:'function id(x){return x;}export function run(a,b){const o={id};const apply=(fn,x)=>fn(x);return apply(o.id,{})===o?1:0;}',
}]);

export const storedFunctionPrimitiveBodies=[
  ['stored-function-undefined','const o={f:x=>x};return o.f(undefined);'],
  ['stored-function-null','const o={f:x=>x};return o.f(null);'],
  ['stored-function-boolean','const o={f:x=>x};return o.f(true);'],
  ['stored-function-number','const o={f:x=>x};return o.f(a);'],
  ['stored-function-predicate','const o={f:x=>x};return o.f(a<b);'],
  ['stored-function-snapshot','const o={f:()=>null};const old=o.f;o.f=()=>false;return old();'],
  ['stored-function-callback','const o={f:x=>x?null:b};const apply=(fn,x)=>fn(x);return apply(o.f,a);'],
];
