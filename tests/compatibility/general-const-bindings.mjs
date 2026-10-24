// Immutable primitive bindings: ECMA-262 lexical declarations and initialization.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations
export const generalConstBindingPromotion=["const-boolean-conditional-initializer", "export function run(a,b) { const flag=a?true:false; return a; }", "export function run(a:number,b:number) { const flag=a?true:false; return a; }", "384e1d8a041cb1f7ae91b4794bd7643f1ce0b3cadf68f7b3e8d36b8034ff69f8"];
const [id,sourceJS,sourceTS]=generalConstBindingPromotion;
export const generalConstBindings=[[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]];
const add=(id,js,ts=js)=>generalConstBindings.push([`general-const-${id}`,js,ts]);
add("null-alias","const value=null,alias=value; return alias===null?a:b;");
add("undefined-alias","const value=undefined,alias=value; return alias===null?a:b;");
add("mixed-join","const value=a?null:b; const alias=value; return alias===null?a:b;");
add("boolean-join","const value=a?true:false; const alias=value; return alias===true?a:b;");
add("scalar-interleaving","const first=null; let n=a; const flag=a<b; const second=a?true:undefined; const alias=second; n+=b; return first===null?(alias===true?n:flag?b:a):0;");
add("loop","let result=0; for(let i=0;i<3;i++) {const value=i<2?null:undefined; const alias=value; if(alias===null) result+=a; else result+=b;} return result;");
add("scope-shadow","const value=a?null:b; {const value=b?true:undefined; value;} return value===null?a:b;");
add("discard-wrappers","const value=a?null:b; ((value)); return value===null?a:b;");
generalConstBindings.push(["general-const-helper", "", "", undefined, undefined, {"sourceJS": "function f(x,y) {const value=x?null:y; const alias=value; return alias===null?x:y;} export function run(a,b) {return f(a,b);}", "sourceTS": "function f(x:number,y:number):number {const value:number=x?null:y; const alias:boolean=value; return alias===null?x:y;} export function run(a:number,b:number):number {return f(a,b);}", "oracleSource": "function f(x,y) {const value=x?null:y; const alias=value; return alias===null?x:y;} export function run(a,b) {return f(a,b);}"}]);
