// A handler receives the original throw value; its own throw belongs to an
// enclosing protected body, and returns/breaks/continues bypass that handler.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-try-statement
const sourceJS='export function run(a,b) { try { throw a; } catch (e) { return e+b; } }';
const sourceTS='export function run(a:number,b:number):number { try { throw a; } catch (e) { return e+b; } }';
export const promotedCatchGap={id:'gap-catch',sourceJS,sourceTS,oracleSource:sourceJS,args:[4,2],entry:'run',expected:6,
  sourceSha256:'982acc099faf201b17b7567aa5c2df65403c0136b17cc97ba70dc37b85fe79bc'};
export const syncCatchPrograms=[
  ['gap-catch','','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}],
  ...[
    ['kind','try{throw(a?null:false);}catch(e){return e===null?1:e===false?2:3;}'],
    ['effect','let n=0;try{throw(n++,b);}catch(e){return n*10+(+e);}'],
    ['rethrow','try{try{throw a;}catch(e){throw(+e+b);}}catch(e){return +e;}'],
    ['capture','let read=()=>0;try{throw a;}catch(e){read=()=>+e;e=b;}return read();'],
    ['loop','let n=0;for(let i=0;i<4;i++){try{throw i;}catch(e){if(e===1)continue;if(e===3)break;n=n*10+(+e);}}return n;'],
  ].map(([id,body])=>[`sync-catch-${id}`,body,body]),
];
