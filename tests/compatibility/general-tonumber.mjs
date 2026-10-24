// ECMA-262 ToNumber and arithmetic over the four represented primitive kinds.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-applystringornumericbinaryoperator
export const generalToNumberPromotions = [
  [
    "boolean-numeric-unary-reject-root-logical",
    "export function run(a,b) { return +(true&&false); }",
    "export function run(a:number,b:number) { return +(true&&false); }",
    "70ce8d976ce1a4b398c180236044bdce107ec4a3f146a05622747bf6cd6bca74"
  ],
  [
    "boolean-numeric-unary-reject-root-conditional",
    "export function run(a,b) { return -(a?true:false); }",
    "export function run(a:number,b:number) { return -(a?true:false); }",
    "ae3bb7cd15229787f394f2cc79ea72010fd1dbf16108435f15f8562a53dd10ee"
  ],
  [
    "boolean-numeric-unary-reject-root-sequence",
    "export function run(a,b) { return +(0,true); }",
    "export function run(a:number,b:number) { return +(0,true); }",
    "13870ed25d5474cee333cc9cd91fd2fc1c7cfa176ea49c3493355676caa2d746"
  ],
  [
    "boolean-numeric-unary-reject-null",
    "export function run(a,b) { return +null; }",
    "export function run(a:number,b:number) { return +null; }",
    "a4c46dfc1802c8a3211ba61bfa4da9dadd0576b0f4e49fd794f32ae8ca035bfc"
  ],
  [
    "boolean-arithmetic-logical-boolean",
    "export function run(a,b) { return (true&&a)+1; }",
    "export function run(a:number,b:number) { return (true&&a)+1; }",
    "ce220bb9b3b98858d1b926951e499340207df92cdcbaceee3aab6cd996f29439"
  ],
  [
    "boolean-arithmetic-logical-bool-result",
    "export function run(a,b) { return (a&&true)+1; }",
    "export function run(a:number,b:number) { return (a&&true)+1; }",
    "bfdc7bd11392f46517cb14e9b1297b67b9acb47705e4f3f40d894ea53a0b41f3"
  ],
  [
    "boolean-arithmetic-conditional-boolean",
    "export function run(a,b) { return (a?true:false)+1; }",
    "export function run(a:number,b:number) { return (a?true:false)+1; }",
    "4bdf6513fb4667b6b46c52b6246eecb2bb995e304ac245c7a67ab7134b1f552c"
  ],
  [
    "boolean-arithmetic-root-comma-boolean",
    "export function run(a,b) { return (a,true)+1; }",
    "export function run(a:number,b:number) { return (a,true)+1; }",
    "2da0ce28df906d995d8497069d41e8d2d5fccaa784b46fcfa47e2e636359ac91"
  ],
  [
    "boolean-arithmetic-null",
    "export function run(a,b) { return false+null; }",
    "export function run(a:number,b:number) { return false+null; }",
    "c7efa834a2bd39d868fb18e0291ad26c90056052a702b74e3d04245d625db4f6"
  ],
  [
    "boolean-arithmetic-undefined",
    "export function run(a,b) { return true+undefined; }",
    "export function run(a:number,b:number) { return true+undefined; }",
    "5317dc0dca8ae495de0322485b2d9fa0b3fc4cbef420dc1dfb6fcbc5527b2885"
  ],
  [
    "unary-predicate-binding-reject-mutable",
    "export function run(a,b) { let flag=true; return +flag; }",
    "export function run(a:number,b:number) { let flag=true; return +flag; }",
    "5991104a2a19911c79f893ea5d1d93ff10fdb90ff26c48b6d960cb11eed1e30f"
  ],
  [
    "unary-predicate-binding-reject-root-logical",
    "export function run(a,b) { const flag=true; return +(flag&&false); }",
    "export function run(a:number,b:number) { const flag=true; return +(flag&&false); }",
    "89822def306645bc8e6ed5e2fa01e09c88b1013a7b2b0e3ee60eb7be86cf0ce4"
  ],
  [
    "unary-predicate-binding-reject-root-conditional",
    "export function run(a,b) { const flag=true; return +(a?flag:false); }",
    "export function run(a:number,b:number) { const flag=true; return +(a?flag:false); }",
    "7999e0993e13a164d868a98f3f7c749f8fb0b1933cf605bb42566b671e00358d"
  ],
  [
    "unary-predicate-binding-reject-root-sequence",
    "export function run(a,b) { const flag=true; return +(0,flag); }",
    "export function run(a:number,b:number) { const flag=true; return +(0,flag); }",
    "52c378dad70be010b4b957dc96762384f627ce412bbbd3c7cf869078bfba35a9"
  ],
  [
    "arithmetic-predicate-binding-reject-mutable",
    "export function run(a,b) { let flag=true; return flag+a; }",
    "export function run(a:number,b:number) { let flag=true; return flag+a; }",
    "c6452ea50cdeb3a69519b1a8575d1b3b3b8a66d5434e3e29af08e61d1d525bcc"
  ],
  [
    "arithmetic-predicate-binding-reject-root-conditional",
    "export function run(a,b) { const flag=true; return (a?flag:false)+b; }",
    "export function run(a:number,b:number) { const flag=true; return (a?flag:false)+b; }",
    "52e6ef51355561ca46416550f795022e60badc4b6b718982d3f839f0ec012135"
  ],
  [
    "arithmetic-predicate-binding-reject-root-logical",
    "export function run(a,b) { const flag=true; return (flag&&true)+b; }",
    "export function run(a:number,b:number) { const flag=true; return (flag&&true)+b; }",
    "e58757eb5a34a9e0c38c0f1d9899ae8d75fa69477ce712e9ffce5c796fe26d1a"
  ],
  [
    "arithmetic-predicate-binding-reject-root-sequence",
    "export function run(a,b) { const flag=true; return (0,flag)+b; }",
    "export function run(a:number,b:number) { const flag=true; return (0,flag)+b; }",
    "12677d7129d8e022d255bbb89864227e2b8db0597424419d41991e48de0c4d5b"
  ],
  [
    "predicate-conditional-arithmetic",
    "export function run(a,b) { return (a?true:false)+b; }",
    "export function run(a:number,b:number) { return (a?true:false)+b; }",
    "325cf93f49fc8ea30d59be6ae5004caacee5eea005822b56034a2b6c402c6401"
  ]
];
export const generalToNumber = generalToNumberPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>generalToNumber.push([`general-tonumber-${id}`,js,ts]);
add('undefined','const value=undefined; return +value;');
add('null-negative-zero','const value=null; return -value;');
add('number-bits','let value=null; value=a; return +value;');
add('late-kind','let value=a; value=a?false:null; return value+b;');
add('assignment-result','let value=null; return +(value=a?true:undefined);');
add('captured-left','let value=true; return value+(value=false);');
add('captured-number','let value=null; value=a; return value-(value=b);');
add('mixed-division','let value=null; return (value=a?true:null)/(value=b?false:true);');
add('general-const','const value=a?null:b; return value*b;');
add('rhs-once','let value=null; const before=+(value=b++); return before+b;');
add('selected-effect','let value=null; return +(a?(value=true):(value=false))+(value?b:a);');
add('logical','const value=a&&false; return value+b;');
add('sequence','let value=null; return (value=a,b)+(value=true);');
add('alias','let value=null; value=a; const alias=value; value=null; return +alias+b;');
add('loop','let result=0; for(let i=0;i<3;i++){let value=null; value=i?false:a; result=result+(+value);} return result;');
add('type-wrapper','const value=a?null:b; return -value;','const value:number=a?null:b; return -((value as unknown)!);');
