// ECMA-262 lexical initialization and plain assignment return captured RHS values.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
export const mutableGeneralPromotions=[
 ["declarators-later-boolean", "export function run(a,b) { let x=a,y=true; return x; }", "export function run(a:number,b:number) { let x=a,y=true; return x; }", "0b70bc4c3e757ed13cebda78963466f7ca210b7e1670f3d97afb7ba2298bb156"],
 ["break-initializer-still-numeric", "export function run(a,b) { for(let i=(true);0;) { break; } return a; }", "export function run(a:number,b:number) { for(let i=(true);0;) { break; } return a; }", "fccdb260d7c791f3f2b2913fdbcde2cdd78561343a691f79335d7f4a62f4f430"],
 ["continue-numeric-initializer", "export function run(a,b) { for(let i=true;0;) { continue; } return a; }", "export function run(a:number,b:number) { for(let i=true;0;) { continue; } return a; }", "12675a05174aa36bb5d7095353e8dcabe1706c9b899a24bab085dd7c2d5492e4"],
 ["discarded-number-local", "export function run(a,b) { let x=true; return a; }", "export function run(a:number,b:number) { let x=true; return a; }", "352e161b8424eb4a19693e27dbbc966b411e36442b442eb70a33d9b1d10ec348"],
 ["const-boolean-let-literal", "export function run(a,b) { let flag=true; return a; }", "export function run(a:number,b:number) { let flag=true; return a; }", "3921fb7e6314b7b69081533cb3b898dae58520a2a060f31164c2f0c2583185cb"],
 ["const-boolean-let-alias", "export function run(a,b) { const flag=true; let copy=flag; return a; }", "export function run(a:number,b:number) { const flag=true; let copy=flag; return a; }", "992b8d05bf3d3781f9916432a2ff745ea37f055896ad040ce9c47727b239a5cf"],
 ["discarded-predicate-binding-reject-mutable", "export function run(a,b) { let flag=true; flag; return a; }", "export function run(a:number,b:number) { let flag=true; flag; return a; }", "eca7560dd60d2bfb8766b6bf9bd51d51a5957fdb7f607c6fb8caeea2993df964"],
 ["strict-boolean-equality-reject-mutable", "export function run(a,b) { let flag=true; return flag===false?1:0; }", "export function run(a:number,b:number) { let flag=true; return flag===false?1:0; }", "6d7521be39cfbb6efe199ea11ca087cfa246dcdfc763a607bb686634569cfcd8"],
 ];
export const mutableGeneralLocals=mutableGeneralPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>mutableGeneralLocals.push([`mutable-general-${id}`,js,ts]);
add("later-kind","let value=a; value=null; return value===null?1:0;");
add("write-alias-plan","let first=a,second=b; first=second; second=null; return first===b?a:b;");
add("numeric-result","let value=null; const result=(value=b); return result;");
add("predicate-result","let value=null; if(value=b) return a; return b;");
add("chained","let left=null,right=false; left=(right=a?true:b); return left===true?a:b;");
add("loop","let sum=0; for(let i=0;i<3;i++){let value=a; if(i<2)value=null; if(value===null)sum+=b; else sum+=a;} return sum;");
add("scalar-interleaving","let value=0,n=a; value=null; n+=b; value=n; return value===n?1:0;");
add("snapshot-before-write","let value=a; const captured=value; value=null; return captured===a?1:0;");
add("boolean-cell","let flag=true; flag=a?false:true; return flag?a:b;");
add("numeric-rhs-once","let value=0; value=null; const old=(value=b++); return old+b;");
add("nested-assignment","let value=null; const captured=(value=(value=a,b)); return captured;");
add("sequential-declarators","let value=null; const first=(value=a),second=(value=b); return first+second;");
add("scope","let value=a; {let value=null; value=false;} return value;");
add("return-comparison","let value=true; value=a?null:b; return value===null?a:b;");
add('assertion-kind','let value=a; value=null; return value===null?a:b;','let value:number=a; value=(null as unknown); return (value as boolean)===null?a:b;');
mutableGeneralLocals.push(['mutable-general-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y){let value=x; value=null; return value===null?x:y;} export function run(a,b){return f(a,b);}',
 sourceTS:'function f(x:number,y:number):number{let value:number=x; value=null; return (value as boolean)===null?x:y;} export function run(a:number,b:number):number{return f(a,b);}',
}]);
