// Normal cleanup preserves the pending completion; abrupt cleanup replaces it.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-try-statement
export const syncFinallyPromotions=[
  {
    id:'gap-finally-return',
    sourceJS:'export function run(a,b) { try { return a; } finally { return b; } }',
    sourceSha256:'6b4ab88449bd24de349b188ca007fc0c7bd6dc720d3291b3a400b1d6b2b4c114',
    args:[4,2],entry:'run',expected:2,
  },
  {
    id:'continue-finally-remains',
    sourceJS:'export function run(a,b) { while(0) { try { continue; } finally { a=b; } } return b; }',
    sourceSha256:'f5aac6d9a2c65911532fd31a6abbb6d88c2092a9fb978b981468aa5c59d0b491',
    args:[4,2],entry:'run',expected:2,
  },
].map(record=>({...record,sourceTS:record.sourceJS.replace('run(a,b)','run(a:number,b:number):number'),oracleSource:record.sourceJS}));
export const syncFinallyPrograms=[
  ...syncFinallyPromotions.map(({id,sourceJS,sourceTS,oracleSource})=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource}]),
  ...[
    ['return-snapshot','let x=a;try{return x;}finally{x=b;}'],
    ['normal','let n=1;try{n*=10;}finally{n+=2;}return n;'],
    ['throw-order','let n=0;try{try{throw a;}finally{n=2;}}catch(e){return +e+n;}'],
    ['loop-exits','let n=0;for(let i=0;i<3;i++){try{if(i===0)continue;if(i===2)break;n+=10;}finally{n++;}}return n;'],
    ['reentry','let n=0;for(let i=0;i<3;i++){try{if(i===0)continue;n+=10;}finally{n++;}n+=100;}return n;'],
    ['override','let n=0;for(let i=0;i<2;i++){try{return a;}finally{n++;continue;}}return n;'],
  ].map(([id,body])=>[`sync-finally-${id}`,body,body]),
];
