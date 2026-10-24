// ECMA-262 IsLessThan/relational operators restricted to four represented primitives.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islessthan
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-relational-operators-runtime-semantics-evaluation
export const primitiveRelationalPromotions = [
  [
    "boolean-arithmetic-comparison",
    "export function run(a,b) { return true<a?1:0; }",
    "export function run(a:number,b:number) { return true<a?1:0; }",
    "690678b0224d420bd5bfb9a9f6586d49310af5f114a372a58e227cb1fa9382bc"
  ],
  [
    "strict-boolean-equality-reject-relational",
    "export function run(a,b) { return true<false?1:0; }",
    "export function run(a:number,b:number) { return true<false?1:0; }",
    "c75a8e4b8270b05535d33223ea234c11e343e2c13af43c8aab8cd04a7a4e9c8b"
  ]
];
export const primitiveRelational=primitiveRelationalPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const rows=[
  [
    "left-write",
    "let value=null; value=a; return value<(value=true)?a:b;",
    "let value=null; value=a; return value<(value=true)?a:b;"
  ],
  [
    "greater-write",
    "let value=true; return value>(value=null)?a:b;",
    "let value=true; return value>(value=null)?a:b;"
  ],
  [
    "parameter-ge",
    "a=null; return a>=(a=b)?1:0;",
    "a=null; return a>=(a=b)?1:0;"
  ],
  [
    "parameter-le",
    "a=false; return a<=(a=b)?1:0;",
    "a=false; return a<=(a=b)?1:0;"
  ],
  [
    "once",
    "let count=0; const result=(count++,null)<=(count++,false); return (result?10:0)+count;",
    "let count=0; const result=(count++,null)<=(count++,false); return (result?10:0)+count;"
  ],
  [
    "conditional",
    "let left=a?null:false,right=b?undefined:true; return left<right?a:b;",
    "let left=a?null:false,right=b?undefined:true; return (left as unknown)<(right as number)?a:b;"
  ],
  [
    "loop",
    "let value=null; for(let i=0;i<3;i++){if(value<=false)value+=true;} return +value;",
    "let value=null; for(let i=0;i<3;i++){if(value<=false)value+=true;} return +value;"
  ],
  [
    "undefined-le",
    "let value=undefined; return value<=null?1:0;",
    "let value=undefined; return value<=null?1:0;"
  ],
  [
    "undefined-ge",
    "let value=undefined; return value>=null?1:0;",
    "let value=undefined; return value>=null?1:0;"
  ],
  [
    "assignment",
    "let value=null; value=a; return (value=b)<a?b:a;",
    "let value=null; value=a; return (value=b)<a?b:a;"
  ],
  [
    "recursive-rhs",
    "if(a){let value=null;return value<run(0,b)?a:b;}return b;",
    "if(a){let value=null;return value<run(0,b)?a:b;}return b;"
  ],
  [
    "snapshot",
    "let value=null; const result=value<a; value=b; return result?a:b;",
    "let value=null; const result=value<a; value=b; return result?a:b;"
  ],
  [
    "predicate",
    "const flag=a<b; return flag<=true?a:b;",
    "const flag=a<b; return flag<=true?a:b;"
  ],
  [
    "implicit",
    "let value; return value>b?a:b;",
    "let value; return value>b?a:b;"
  ],
  [
    "alias",
    "let value=false; const alias=value; value=a; return alias>=value?a:b;",
    "let value=false; const alias=value; value=a; return alias>=value?a:b;"
  ],
  [
    "while",
    "let count=0; while(count<true){count++;} return count+a;",
    "let count=0; while(count<true){count++;} return count+a;"
  ]
];
primitiveRelational.push(...rows.map(([id,js,ts])=>['primitive-relational-'+id,js,ts]));
