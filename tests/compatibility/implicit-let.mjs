// ECMA-262 14.3.1.3: bare let initializes intrinsic undefined at declaration.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-runtime-semantics-bindingevaluation
export const implicitLetPromotions = [
  [
    "for-uninitialized",
    "js",
    "export function run(a,b) { for(let i;0;i++) {} return a; }",
    "576fd857c0c42dfa3bc62f9d02129c40c765673fa415a6ca9da496742a90f002",
    "export function run(a,b) { for(let i;0;i++) {} return a; }",
    "export function run(a:number,b:number) { for(let i;0;i++) {} return a; }"
  ],
  [
    "declarators-missing-later-init",
    "js",
    "export function run(a,b) { let x=a,y; return x; }",
    "1182f48ab3f0474140a4c9d8a160b0f6c38ea3bd57675652a9f0aba2b1af5435",
    "export function run(a,b) { let x=a,y; return x; }",
    "export function run(a:number,b:number) { let x=a,y; return x; }"
  ],
  [
    "annotation-missing-initializer",
    "ts",
    "export function run(a:unknown,b:unknown):unknown { let x:unknown; return a; }",
    "743dbff5fe92bce0f0efe95e87cc75128902a4ca01d644ea54ebd81db2d91e54",
    "export function run(a,b) { let x; return a; }",
    "export function run(a:unknown,b:unknown):unknown { let x:unknown; return a; }"
  ],
  [
    "uninitialized-let",
    "js",
    "export function run(a,b) { let x; return x=a; }",
    "b1ecd30a1d3facec1b95fe929feba349e69082ef3346a01620acceb26ee70822",
    "export function run(a,b) { let x; return x=a; }",
    "export function run(a:number,b:number) { let x; return x=a; }"
  ]
];
export const implicitLet=implicitLetPromotions.map(([id,_extension,_source,_hash,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>implicitLet.push(['implicit-let-'+id,js,ts]);
add('undefined','let value; return value===undefined?1:0;');
add('undefined-shadow','let undefined=a; let value; return value===undefined?1:0;');
add('aliases','let left,right=left; left=a; return right===undefined?+left:b;');
add('source-order','let count=0; let x,y=(count++,x),z=(count++,a); return count*10+(y===undefined?+z:0);');
add('iteration','let sum=0; for(let i=0;i<3;i++){let value; sum+=value===undefined?1:0; value=a;} return sum;');
add('snapshot','let value; const saved=value; value=null; return saved===undefined?a:b;');
add('scope','let value; value=a; {let value; value=b;} return +value;');
add('for','let count=0; for(let value; value===undefined;){count++;value=a;} return count;');
add('assignment','let value; return +(value=a?null:false)+b;');
add('update','let value; return value++;');
add('conditional','let value; return +(a?(value=b):value);');
add('discard','let value; value; return a;');
add('loose-equality','let value; return value==null?a:b;');
add('sequence','let value; let other=value; value=b; return other===undefined?+value:a;');
add('do','let sum=0,i=0; do {let value; if(value===undefined)sum+=a;value=b;i++;}while(i<2); return sum;');
add('type','let value; return value===undefined?a:b;','let value:Missing; return (value as unknown)===undefined?a:b;');
