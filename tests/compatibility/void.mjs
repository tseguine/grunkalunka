// Evaluate/GetValue must complete before void yields intrinsic Undefined.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-void-operator
const sourceJS='export function run(a,b) { return void a === undefined ? 1 : 0; }';
const sourceTS='export function run(a:number,b:number):number { return void a === undefined ? 1 : 0; }';
export const promotedVoidGap={id:'gap-void',sourceJS,sourceTS,oracleSource:sourceJS,args:[4,2],entry:'run',expected:1,
  sourceSha256:'66d77e4e765a76f62ed187251314ec8608540d6b80cf2648f89d2fe9135250e1'};
export const voidPrograms=[
  ['gap-void','','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}],
  ...[
    ['effect','let x=a;const out=void(x=b);return +(out===undefined)+x;'],
    ['update','let x=a;void(x++);return x;'],
    ['predicate','if(void(a=b))return a;return b;'],
    ['tonumber','return +(void a);'],
    ['closure','let x=a;const f=()=>x++;void f();return x;'],
    ['shadow','const undefined=a;return +(void b);'],
  ].map(([id,body])=>[`void-${id}`,body,body]),
];
