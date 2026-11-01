// Module declarations retain one identity while adapting to typed closure calls.
const cases = [
  ['alias', 'function f(x){return x;}', 'const g=f;const o={f,g};return o.f===o.g&&g(a)===f(a)?1:0;'],
  ['distinct', 'function f(x){return x;}function g(x){return x;}', 'const o=[f,g];return o[0]===f&&o[1]===g&&f!==g?1:0;'],
  ['fresh', 'function f(x){return x;}', 'const g=x=>x;return f!==g&&[f,f][0]===[f][0]?1:0;'],
  ['entry', '', 'const f=run;return f===run&&[f][0]===run?1:0;'],
  ['zero', 'function f(){return 3;}', 'const g=f;return g()+f();'],
  ['three', 'function f(x,y,z){return x+y+z;}', 'const g=f;return g(a,b,2);'],
  ['callback', 'function f(x){return x;}', 'const call=(fn,x)=>fn(x);return call(f,a);'],
  ['callback-data', 'function f(x){return x;}', 'const wrap=fn=>({fn});return wrap(f).fn===f?1:0;'],
  ['rebind', 'function f(x){return x;}function g(x){return -x;}', 'let h=f;const old=h;const o={h};h=g;return o.h===old&&old===f&&h===g&&old!==h?1:0;'],
  ['effects', 'function f(x){return x;}', 'let n=0;const h=(n++,f);const o={h:(n++,h)};return n===2&&o.h===f?h(a):b;'],
  ['selected', 'function f(x){return x;}', 'const h=a?f:f;return h===f?h(b):a;'],
  ['object-result', 'function make(x){return {x};}', 'const f=make;const o=f(a);return o.x===a?1:0;'],
  ['object-input', 'function id(x){return x;}', 'const o={x:a};const f=id;return f(o)===o?1:0;'],
  ['array-callback-result', 'function make(x){return [x];}', 'const f=make;const call=(fn,x)=>fn(x);return call(f,a)[0]===a?1:0;'],
  ['string-input', 'function id(x){return x;}', "let f=id;const g=f;return g('text')==='text'?1:0;"],
  ['typeof', 'function f(x){return x;}', "return typeof f==='function'?1:0;"],
  ['length', 'function f(x,y,z){return x+y+z;}', 'return f.length;'],
  ['wrapped-call', 'function f(x){return x;}', 'return ((f))(a);'],
];
export const moduleDeclarationValues = cases.map(([id, helpers, body]) => [
  `module-declaration-${id}`, '', '', undefined, undefined, {
    sourceJS: `${helpers}export function run(a,b){${body}}`,
    sourceTS: `${helpers}export function run(a:number,b:number):number{${body}}`,
  },
]);
moduleDeclarationValues.push(['module-declaration-hoisted-erasure', '', '', undefined, undefined, {
  sourceJS: 'export function run(a,b){const g=f;const o={g};return o.g===f?g(a):b;}function f(x){return x;}',
  sourceTS: 'export function run(a:object,b:string):void{const g=(f as unknown)!;const o={g};return o.g===f?g(a):b;}function f<T>(x:string):boolean{return x;}',
}]);

export const moduleDeclarationPrimitiveSources = [
  'function f(x){if(x)return null;return false;}export function run(a,b){const g=f;return g(a);}',
  'function f(x){return x?null:x;}export function run(a,b){const g=f;return g(a);}',
  'function f(x){return x;}export function run(a,b){const o={f};return o.f===f;}',
];
