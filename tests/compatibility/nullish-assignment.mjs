// Mutable primitive identifier ??= uses a nullish test, not ToBoolean.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
const sourceJS='export function run(a,b) { return a ??= b; }';
const sourceTS='export function run(a:number,b:number):number { return a ??= b; }';
export const promotedNullishAssignment={id:'unsupported-compound-10',sourceJS,sourceTS,oracleSource:sourceJS,args:[4,2],expected:4,
  sourceSha256:'f4f8e0f8afbb4aaf5a528360b0944d6d61875e68ecaa0f30d2e69556a66c9eb0'};
export const nullishAssignment=[
  ['unsupported-compound-10','','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}],
  ...[
    ['numeric-skip','return a??=(a=b);'],
    ['false-skip','let value=false;value??=b++;return b;'],
    ['null-store','let value=null;value??=a;return +value;'],
    ['undefined-store','let value;return +(value??=b);'],
    ['capture','let value=null;const read=()=>value;value??=a;return +read();'],
    ['rhs-mutation','let value=null;const out=value??=(value=false,b);return +out+(+value);'],
    ['predicate','let value=null;if(value??=a)return b;return a;'],
  ].map(([id,body])=>[`nullish-assignment-${id}`,body,body]),
];
