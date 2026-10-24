// GetValue/evaluation effects remain observable even when the result is unused.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-expression-statement-runtime-semantics-evaluation
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-comma-operator-runtime-semantics-evaluation
export const generalDiscardPromotions = [
  [
    "discarded-null",
    "export function run(a,b) { null; return a; }",
    "export function run(a:number,b:number) { null; return a; }",
    "4750db473ffb0e353284ee4cc39e329223c95ea62075563a94c9cf1ab49b4110"
  ],
  [
    "discarded-undefined",
    "export function run(a,b) { undefined; return a; }",
    "export function run(a:number,b:number) { undefined; return a; }",
    "67ca4cf7ef0e0582b9b3dd7ab1725b95a3545d8ab17239678a2276cffafab6b2"
  ],
  [
    "discarded-predicate-binding-reject-root-conditional",
    "export function run(a,b) { const flag=true; a?flag:flag; return a; }",
    "export function run(a:number,b:number) { const flag=true; a?flag:flag; return a; }",
    "f1b61f72291efa73923c67e9912a8e26d30c59a09d5da4cee5e96179f24a4697"
  ],
  [
    "discarded-predicate-binding-reject-root-sequence",
    "export function run(a,b) { const flag=true; a,flag; return a; }",
    "export function run(a:number,b:number) { const flag=true; a,flag; return a; }",
    "947b65bf0435d83dd7ba5ebffa82468f5cb1afba8670f54504cab950f07a9cac"
  ]
];
export const generalDiscard=generalDiscardPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>generalDiscard.push([`general-discard-${id}`,js,ts]);
add('sequence','let x=0; (x++,null); return x;');
add('conditional','let x=0; a?(x++,null):(x+=2,undefined); return x;');
add('for-update','let x=0; for(let i=0;i<3;(i++,null))x+=1; return x;');
add('for-init','let x=0; for((x++,null);x<3;(x++,undefined)){} return x;');
add('for-nullish','let i=0; for(null;i<2;undefined){i++;} return i;');
add('logical-skip','let x=0; false&&(x++,null); true||(x++,undefined); return x;');
add('logical-select','let x=0; a&&(x++,null); a||(x+=2,undefined); return x;');
add('parameter-sequence','a=null,b=false; return +a+(+b);');
add('local-sequence','let x=null; (x=false,x=null); return x===null?1:0;');
add('if-body','let x=0; if(a){x++,null;}else{x+=2,undefined;} return x;');
add('while','let i=0; while(i<3){i++,null;} return i;');
add('do','let i=0; do{i++,undefined;}while(i<2); return i;');
add('predicate-branch','let x=0; const flag=true; flag?(x++,null):(x+=2,undefined); return x;');
add('captured-input','const captured=a; (a=null,b=false); return +captured;');
add('nested','let x=0; ((x++,null),(x+=2,undefined)); return x;');
add('type-wrapper','null; undefined; return a;','((null as number)!); ((undefined satisfies number)!); return a;');
