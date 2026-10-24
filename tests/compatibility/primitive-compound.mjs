// ECMA-262 13.15.2/13.15.3: left snapshot, RHS, numeric operation, store/result.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
export const primitiveCompoundPromotions = [
  [
    "boolean-arithmetic-compound",
    "export function run(a,b) { a+=true; return a; }",
    "export function run(a:number,b:number) { a+=true; return a; }",
    "7171060f52372ff8b47e8d5fc01e1f27c40ed0f4256a299593b67e675382b664"
  ],
  [
    "boolean-arithmetic-compound-remainder",
    "export function run(a,b) { a%=true; return a; }",
    "export function run(a:number,b:number) { a%=true; return a; }",
    "2256f91aca94f09288a3a77db1dfb4af4dc4af383093353aa67afeffdfd7cd4b"
  ],
  [
    "unary-predicate-binding-reject-compound",
    "export function run(a,b) { const flag=true; a+=flag; return a; }",
    "export function run(a:number,b:number) { const flag=true; a+=flag; return a; }",
    "0d07cc5477cd8440e4582b724a0b1ffab2ce45f98c88ac9d8015ccc9206ff287"
  ],
  [
    "arithmetic-predicate-binding-reject-compound",
    "export function run(a,b) { const flag=true; a+=flag; return a; }",
    "export function run(a:number,b:number) { const flag=true; a+=flag; return a; }",
    "0d07cc5477cd8440e4582b724a0b1ffab2ce45f98c88ac9d8015ccc9206ff287"
  ],
  [
    "discarded-predicate-binding-reject-compound",
    "export function run(a,b) { const flag=true; a+=flag; return a; }",
    "export function run(a:number,b:number) { const flag=true; a+=flag; return a; }",
    "0d07cc5477cd8440e4582b724a0b1ffab2ce45f98c88ac9d8015ccc9206ff287"
  ]
];
export const primitiveCompound=primitiveCompoundPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>primitiveCompound.push(['primitive-compound-'+id,js,ts]);
add('capture','let value=null; value=a; const result=(value+=(value=true)); return result*10+(+value);');
add('sub-capture','let value=true; const result=(value-=(value=null)); return result*10+(+value);');
add('parameter','a=null; const result=(a+=(a=b)); return result+(+a);');
add('once','let count=0,value=null; const result=(value+=(count++,a)); return result+count;');
add('general-rhs','let value=false,rhs=null; rhs=b; return value*=rhs;');
add('divide','let value=true; return value/=a;');
add('undefined','let value=undefined; return value-=b;');
add('chain','let x=null,y=false; return x+=(y+=true);');
add('saved-result','let value=null; const saved=(value+=true); value=null; return saved;');
add('loop','let value=null; for(let i=0;i<3;i++){value+=true;} return +value;');
add('implicit','let value; if(a)value=a; return value+=b;');
add('remainder','return a%=true;');
add('call','let value=null; return value+=Math.abs(b);');
add('recursive-rhs','if(a){let value=null;return value+=run(0,b);}return b;');
add('type','let value=false; return value+=a;','let value:number=false; return (value as number)+=(a as boolean);');
add('remainder-capture','let value=false; const result=(value%=(value=true)); return result+(+value);');
