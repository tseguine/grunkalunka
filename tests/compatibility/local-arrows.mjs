// Non-escaping concise arrows share captured variable cells. No function heap ABI.
// https://tc39.es/ecma262/multipage/ecmascript-language-functions-and-classes.html#sec-runtime-semantics-instantiatearrowfunctionexpression
export const localArrowNumericOrigins = [
  [
    "local-arrow-origin-call",
    "gap-call",
    "export function run(a,b) { return (x => x + b)(a); }",
    "08843f1268bfccd6f0b591612d941aa21453d75125a6707cc73f8cf4dca3a91d"
  ],
  [
    "local-arrow-origin-closure-capture",
    "gap-closure-capture",
    "export function run(a,b) { const read=()=>a; a=b; return read(); }",
    "8805be1d8c56acc24526b4e518a2a83cf798e1dd6cb644e94bf25ba922660b60"
  ]
];
export const localArrows = localArrowNumericOrigins.map(([id,,source])=>
  [id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
const rows = [
  ['shared-write','let x=a; const read=()=>x; const add=()=>x+=b; add(); return read();'],
  ['lexical-shadow','let x=a; const read=()=>x; {let x=b; return read()+x;}'],
  ['alias','const read=()=>a; const alias=read; a=b; return alias();'],
  ['argument-snapshot','const pack=(x,y)=>x*10+y+a; return pack(a++,a++);'],
  ['fresh-parameter','const next=x=>x++; return next(a)+next(b)+a;'],
  ['nested-iife','const f=x=>(y=>x+y+a)(b); return f(a);'],
  ['capture-after-argument','const f=x=>a+x; return f(a=b);'],
  ['loop-call','let x=0; const next=()=>x++; for(let i=0;i<5;i++)next(); return x;'],
  ['loop-binding','let x=0; for(let i=0;i<4;i++){const read=()=>i; x+=read();} return x;'],
  ['branch','const read=()=>a; if(b)return read(); return read()+1;'],
  ['const-result','const read=()=>a; const x=read(); return x+b;'],
  ['negative-zero','const read=()=>-0; return read();'],
  ['sequence-body','const f=x=>(x+=b,x); return f(a)+f(a);'],
  ['write-call-back','const read=()=>a; a=read(); return read();'],
  ['mutable-alias-result','const read=()=>a; const alias=read; let x=alias(); return x+b;'],
  ['parameter-shadow','const f=a=>a+b; return f(b)+a;'],
  ['unused-effect','const f=()=>a=b; return a;'],
  ['predicate','const read=()=>a; return read()?a:b;'],
  ['short-circuit','const read=()=>a; return read()||b;'],
  ['wrapped','const f=(x=>x+a); return (f)(b);',
    'const f=((x:never):string=>x+a) as unknown as number; return (f as never)(b);'],
];
localArrows.push(...rows.map(([id,body,typed=body])=>['local-arrow-'+id,body,typed]));
