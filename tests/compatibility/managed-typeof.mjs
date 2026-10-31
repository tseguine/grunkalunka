// Type inspection over represented values; results are observed through Number exports.
const cases = [
  ['managed-typeof-number', "const o={v:a};return typeof o.v==='number'?a:b;"],
  ['managed-typeof-undefined', "const o={v:undefined};return typeof o.v==='undefined'?a:b;"],
  ['managed-typeof-null', "const o={v:null};return typeof o.v==='object'?a:b;"],
  ['managed-typeof-boolean', "const o={v:false};return typeof o.v==='boolean'?a:b;"],
  ['managed-typeof-string', "const o={v:''};return typeof o.v==='string'?a:b;"],
  ['managed-typeof-object', "const o={v:{}};return typeof o.v==='object'?a:b;"],
  ['managed-typeof-array', "const o={v:[]};return typeof o.v==='object'?a:b;"],
  ['managed-typeof-missing', "const o={};return typeof o.missing==='undefined'?a:b;"],
  ['managed-typeof-array-parts', "const x=[,a];return typeof x[0]==='undefined'&&typeof x[1]==='number'&&typeof x.length==='number'?a:b;"],
  ['managed-typeof-mutation', "const x={v:a};const before=typeof x.v;x.v='s';const after=typeof x.v;return before==='number'&&after==='string'?a:b;"],
  ['managed-typeof-order', "let n=0;const x={v:a};const kind=typeof(n++,x.v=(n++,b));return n===2&&kind==='number'?a:b;"],
  ['managed-typeof-discard', 'let n=0;typeof(n++,{});return n;'],
  ['managed-typeof-update', "let n=0;const kind=typeof(n++);return kind==='number'&&n===1?a:b;"],
  ['managed-typeof-selected', "const kind=typeof(a?{}:'');return kind===(a?'object':'string')?a:b;"],
  ['managed-typeof-nested', "return typeof typeof a==='string'?a:b;"],
  ['managed-typeof-globals', "return typeof undefined==='undefined'&&typeof NaN==='number'&&typeof Infinity==='number'?a:b;"],
  ['managed-typeof-function', "let n=0;const f=()=>++n;const kind=typeof f;return kind==='function'&&n===0&&f()===1?a:b;"],
  ['managed-typeof-function-effects', "let n=0;const kind=typeof(n++,()=>a);return n===1&&kind==='function'?a:b;"],
];
export const managedTypeof = cases.map(([id,body])=>[id,body,body]);
const helperJS="function kind(x){return typeof x;}export function run(a,b){const o={};const first=kind(a);const second=kind('');const third=kind(o);return first==='number'&&second==='string'&&third==='object'?a:b;}";
const helperTS="function kind(x:number):number{return typeof(x as object);}export function run(a:string,b:null):number{const o={};const first=kind(a);const second=kind('');const third=kind(o);return first==='number'&&second==='string'&&third==='object'?a:b;}";
managedTypeof.push(['managed-typeof-helper-erasure','','',undefined,undefined,{sourceJS:helperJS,sourceTS:helperTS}]);
for(const [id,source] of [
  ['managed-typeof-hoisted',"function f(x){return x+1;}export function run(a,b){return typeof f==='function'?a:b;}"],
  ['managed-typeof-shadowed',"function f(x){return x;}export function run(f,b){return typeof f==='number'?f:b;}"],
  ['managed-typeof-helper-discard',"function kind(x){return typeof x;}export function run(a,b){let n=0;kind(n++);return n;}"],
]) managedTypeof.push([id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
// Exact original gap source is now executed; its parent requirement remains open.
export const promotedTypeof = {
  id: 'gap-typeof',
  sourceSha256: '34e0f464195e83123a20d4854639000916133aa27bcb6d79b7b51cf8c361f23a',
  sourceJS: 'export function run(a,b) { return typeof a === "number" ? 1 : 0; }',
};
managedTypeof.push([promotedTypeof.id,'','',1,undefined,{sourceJS:promotedTypeof.sourceJS,sourceTS:promotedTypeof.sourceJS}]);

managedTypeof.push(
  ['managed-typeof-nullish-default', "const o={v:a?null:''};return typeof(o.v??[])===(a?'object':'string')?a:b;", "const o={v:a?null:''};return typeof(o.v??[])===(a?'object':'string')?a:b;"],
  ['managed-typeof-logical', "const o={v:a};return typeof(o.v&&'s')===(a?'string':'number')?a:b;", "const o={v:a};return typeof(o.v&&'s')===(a?'string':'number')?a:b;"],
);
const nullable="function kind(x){return typeof x;}export function run(a,b){return kind(null)==='object'?a:b;}";
managedTypeof.push(['managed-typeof-nullable-helper','','',undefined,undefined,{sourceJS:nullable,sourceTS:nullable}]);
