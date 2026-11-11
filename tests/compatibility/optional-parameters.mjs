// Optional markers erase; declared arity and calling profiles remain intact.
export const optionalParameterPromotions = [
  {
    "id": "optional-single-parameter",
    "sourceTS": "export function run(a?:number){return a;}",
    "sourceJS": "export function run(a){return a;}",
    "sourceSha256": "f7909809c2761b6f30ba0044f3d05789d9b190fa6f59c6d2e20a4afb4cce92d7"
  },
  {
    "id": "optional-param",
    "sourceTS": "export function run(a?:number,b?:number):number { return a; }",
    "sourceJS": "export function run(a,b) { return a; }",
    "sourceSha256": "eaa562ed332c5c1c9322f7d9816b98d22a104f395a48a89a5ceeb0905dc1469c"
  },
  {
    "id": "annotation-optional-parameter",
    "sourceTS": "export function run(a?:unknown,b?:unknown):unknown { return a; }",
    "sourceJS": "export function run(a,b) { return a; }",
    "sourceSha256": "2650c0bceb1b308cb60c9f550bef4ee99bc818a697692f0abc523636e2fcdae2"
  },
  {
    "id": "direct-call-helper-optional",
    "sourceTS": "function helper(x?:number) { return x; } export function run(a,b) { return helper(a); }",
    "sourceJS": "function helper(x) { return x; } export function run(a,b) { return helper(a); }",
    "sourceSha256": "2576772908a210ddeac9da4fc407220a6605b6c58178cc149b7a8e0a77ed6ff5"
  },
  {
    "id": "generic-declaration-reject-optional",
    "sourceTS": "export function run<T>(a?:T,b?:T):T { return a; }",
    "sourceJS": "export function run(a,b) { return a; }",
    "sourceSha256": "2e416cdf03bbe8b193da433edf296984af39e4ac3e4b2b166ea410b9c6cc7403"
  }
];
export const optionalParameterPrograms = [
  ...optionalParameterPromotions.map(p=>[p.id,'','',undefined,undefined,{sourceJS:p.sourceJS,sourceTS:p.sourceTS}]),
  ...[
    ['optional-arrow','const f=(x,y)=>x+y;return f(a,b);','const f=(x?:string,y?:unknown):unknown=>x+y;return f(a,b);'],
    ['optional-capture','let n=a;const f=x=>{n+=x;return n;};return f(b)+f(a);','let n=a;const f=(x?:number)=>{n+=x;return n;};return f(b)+f(a);'],
    ['optional-length','const f=(x,y)=>x+y;return f.length+f(a,b);','const f=(x?:unknown,y?:unknown)=>x+y;return f.length+f(a,b);'],
    ['optional-stored','const o={f:x=>x+a};return o.f(b);','const o={f:(x?:unknown)=>x+a};return o.f(b);'],
    ['optional-callback','const apply=(f,x)=>f(x);const id=x=>x+a;return apply(id,b);','const apply=(f?:unknown,x?:unknown)=>f(x);const id=(x?:unknown)=>x+a;return apply(id,b);'],
  ].map(([id,js,ts])=>[id,js,ts]),
  ['optional-module-helper','','',undefined,undefined,{
    sourceJS:'function add(x,y){return x+y;}export function run(a,b){return add(a,b);}',
    sourceTS:'function add(x?:string,y?:unknown):unknown{return x+y;}export function run(a?:number,b?:number){return add(a,b);}',
  }],
];
