// Original-source input relations connect module helpers to local callbacks.
const cases = [
  ['object', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,o)===o?1:0;'],
  ['array', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const o=[a,b];return apply(id,o)===o?1:0;'],
  ['string', 'function id(x){return x;}', "const apply=(fn,x)=>fn(x);return apply(id,'😀')==='😀'?1:0;"],
  ['null', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);return apply(id,null)===null?1:0;'],
  ['undefined', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);return apply(id,undefined)===undefined?1:0;'],
  ['mixed', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,a)===a&&apply(id,o)===o?1:0;'],
  ['boolean-object', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const o={};return apply(id,true)===true&&apply(id,o)===o?1:0;'],
  ['numeric-result', 'function size(x){return x.length;}', 'const apply=(fn,x)=>fn(x);return apply(size,[a,b]);'],
  ['forward', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const forward=(fn,x)=>apply(fn,x);const o={};return forward(id,o)===o?1:0;'],
  ['local-alias', 'function id(x){return x;}', 'const apply=(fn,x)=>{let local=fn;const value=x;return local(value);};const o={};return apply(id,o)===o?1:0;'],
  ['factory', 'function id(x){return x;}', 'const make=fn=>x=>fn(x);const apply=make(id);const o={};return apply(o)===o?1:0;'],
  ['block-factory', 'function id(x){return x;}', 'const make=fn=>{const apply=x=>fn(x);return apply;};const apply=make(id);const o={};return apply(o)===o?1:0;'],
  ['nested', 'function id(x){return x;}', 'const apply=(fn,x)=>{const again=y=>fn(y);return again(x);};const o=[];return apply(id,o)===o?1:0;'],
  ['returned-callback', 'function id(x){return x;}', 'const identity=fn=>fn;const apply=identity((fn,x)=>fn(x));const o={};return apply(id,o)===o?1:0;'],
  ['mixed-constructors', 'function id(x){return x;}', 'const apply=(fn,x)=>fn(x);const f=a?id:x=>x;const o={};return apply(f,o)===o?1:0;'],
  ['snapshot', 'function id(x){return x;}function other(x){return x;}', 'let f=id;const apply=(fn,x)=>fn(x);const o={};return apply(f,(f=other,o))===o&&f===other?1:0;'],
  ['receiver-surplus', 'function id(x){return x;}', 'let n=0;const apply=(fn,x)=>fn(x);const o={};const result=apply.call((n++,b),id,(n++,o),(n++,b));return result===o&&n===3?1:0;'],
  ['callee-surplus', 'function id(x){return x;}', 'let n=0;const apply=(fn,x)=>fn(x,(n++,{}));const result=apply(id,a);return n===1&&result===a?1:0;'],
  ['cycle', 'function id(x){return x;}', 'const make=fn=>x=>fn(x);const apply=make(id);const o={};o.self=o;const result=apply(o);return result===o&&result.self===o?1:0;'],
  ['mutation', 'function set(o,x){o.x=x;return o;}', 'const apply=(fn,o,x)=>fn(o,x);const o={x:b};const result=apply(set,o,a);return result===o&&o.x===a?1:0;'],
  ['read', 'function read(o){return o.x;}', 'const apply=(fn,o)=>fn(o);const o={x:a};return apply(read,o)===a?1:0;'],
];
export const moduleCallbackInputs=cases.map(([id,helpers,body])=>[
  `module-callback-${id}`,'','',undefined,undefined,{
    sourceJS:`${helpers}export function run(a,b){${body}}`,
    sourceTS:`${helpers}export function run(a:number,b:number):number{${body}}`,
  },
]);
// Original rejected source, now admitted without changing its observable program.
moduleCallbackInputs.push(['const-boolean-unused-helper','','',undefined,undefined,{
  sourceJS:'function f(x) { const flag=true; return flag; } export function run(a,b) { return a; }',
  sourceTS:'function f(x:number):boolean { const flag=true; return flag; } export function run(a:number,b:number):number { return a; }',
}]);
moduleCallbackInputs.push(['module-callback-erasure','','',undefined,undefined,{
  sourceJS:'function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);const o={x:a};return apply(id,o)===o?1:0;}',
  sourceTS:'function id<T>(x:number):string{return x;}export function run(a:boolean,b:string):void{const apply=(fn:never,x:number):boolean=>fn(x);const o={x:a};return apply<Missing>((id as unknown)!,o)===o?1:0;}',
}]);
moduleCallbackInputs.push(['module-callback-type-only-primitive','','',undefined,undefined,{
  sourceJS:'function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(id,true)===true?1:0;}',
  sourceTS:'type Unused="metadata";function id(x){return x;}export function run(a,b){const apply=(fn,x)=>fn(x);return apply(id,true)===true?1:0;}',
}]);
