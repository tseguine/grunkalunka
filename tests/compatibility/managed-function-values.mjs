// Known typed function values become data without losing creation identity.
const cases = [
  ['binding', 'const f=()=>a;const o={f};return o.f===f?a:b;'],
  ['aliases', 'const f=()=>a;const g=f;const o={f,g};return o.f===o.g&&o.f===f?a:b;'],
  ['reboxing', 'const f=()=>a;const o={f};o.g=f;const x=[f,o.f,o.g];return x[0]===x[1]&&x[1]===x[2]?a:b;'],
  ['fresh', 'const make=()=>()=>0;const f=make();const g=make();return f!==g&&[f][0]===f?a:b;'],
  ['wrapped-fresh', 'const make=()=>()=>a;return (a?(make()):(make()))!==(make())?a:b;'],
  ['captured-fresh', 'const make=n=>()=>n;const f=make(a);const g=make(a);return f!==g&&[f][0]===f&&f()===a?a:b;'],
  ['rebind', 'let f=()=>a;const old=f;const o={f};f=()=>b;return o.f===old&&o.f!==f&&old()===a&&f()===b?a:b;'],
  ['heterogeneous', "const f=()=>a;const g=(x,y)=>x+y;const x=[f,g,a,'text',null,{}];return x[0]===f&&x[1]===g&&typeof x[0]==='function'&&typeof x[1]==='function'&&g(a,b)===a+b?a:b;"],
  ['callback', 'const wrap=fn=>({fn});const f=()=>a;const o=wrap(f);const p=wrap(f);return o.fn===p.fn&&o.fn===f?a:b;'],
  ['creation-effects', 'let n=0;const make=()=>{n++;return ()=>a;};const f=make();const o={f};return n===1&&o.f===f?a:b;'],
  ['forward', 'const wrap=fn=>({fn});const forward=fn=>wrap(fn);const f=()=>a;return forward(f).fn===f?a:b;'],
  ['argument-snapshot', 'let f=()=>a;const wrap=(fn,x)=>({fn});const old=f;const o=wrap(f,(f=()=>b,b));return o.fn===old&&o.fn!==f?a:b;'],
  ['cycle', 'const o={};const f=()=>o;const x=[f,o];o.f=f;o.self=o;o.x=x;return o.f===f&&o.self===o&&x[0]===f&&f()===o?a:b;'],
  ['body-local-iteration', 'let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const n=i;const f=()=>n;if(i===0)first=f;last=f;}const x=[first,last];return x[0]!==x[1]&&first()===0&&last()===2?a:b;'],
  ['former-object-guard', 'const f=()=>a;const o={f};return a;'],
  ['former-array-guard', 'const o=[()=>a];return a;'],
  ['former-result-guard', 'const f=()=>({x:()=>a});return a;'],
  ['former-property-literal-guard', 'const o={x:()=>a};return a;'],
  ['former-property-binding-guard', 'const f=()=>a;const o={x:f};return a;'],
  ['former-logical-guard', 'const x={}||(()=>a);return a;'],
  ['former-typeof-guard', "return typeof(a?()=>a:b)==='function'?a:b;"],
  ['rooted-cycle', 'const o={x:a};const f=()=>o.x===b?1:0;o.f=f;o.self=o;o.x=b;return f();'],
];
export const managedFunctionValues = cases.map(([id, body]) => [`managed-function-${id}`, body, body]);
managedFunctionValues.push(['managed-function-erasure', '', '', undefined, undefined, {
  sourceJS: 'export function run(a,b){const f=x=>x;const o={f};return o.f===f&&f(a)===a?a:b;}',
  sourceTS: 'export function run(a:string,b:boolean):void{const f=(x:boolean):string=>x;const o={f};return o.f===f&&f(a)===a?a:b;}',
}]);
