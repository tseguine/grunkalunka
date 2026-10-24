// ECMA-262 && and || return the selected operand without converting its kind.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-binary-logical-operators-runtime-semantics-evaluation
export const generalLogicalPromotions=[
 ["const-boolean-discarded-logical", "export function run(a,b) { const flag=true; flag&&(a=b); return a; }", "export function run(a:number,b:number) { const flag=true; flag&&(a=b); return a; }", "974e15fd0ecb05bddeab5ab205673d72e37e8f44b0d4d96713a283580165f097"],
 ["const-boolean-logical-initializer", "export function run(a,b) { const flag=true&&a; return a; }", "export function run(a:number,b:number) { const flag=true&&a; return a; }", "28c9375a8efe96f02444af120853f065faa039d340a602d630749397fee707ad"],
 ["discarded-predicate-binding-reject-root-logical", "export function run(a,b) { const flag=true; flag&&(a=b); return a; }", "export function run(a:number,b:number) { const flag=true; flag&&(a=b); return a; }", "974e15fd0ecb05bddeab5ab205673d72e37e8f44b0d4d96713a283580165f097"],
 ["strict-boolean-equality-reject-root-logical", "export function run(a,b) { const flag=true; return (flag&&true)===false?1:0; }", "export function run(a:number,b:number) { const flag=true; return (flag&&true)===false?1:0; }", "511b126934a43eaa27736f56d0dca00ec805c49c4b69d4604967c70499d84d1c"],
 ];
export const generalLogical=generalLogicalPromotions.map(([id,sourceJS,sourceTS])=>[id,'','',undefined,undefined,{sourceJS,sourceTS,oracleSource:sourceJS}]);
const add=(id,js,ts=js)=>generalLogical.push([`general-logical-${id}`,js,ts]);
add("null-and","return (null&&a)===null?b:a;");
add("undefined-or","return (undefined||true)===true?a:b;");
add("mixed-or","const value=(a?null:b)||false; return value===false?1:0;");
add("mixed-and","const value=(a?true:null)&&(b?null:0); return value===null?a:b;");
add("discard-null","null&&(b=a); return b;");
add("discard-undefined","undefined||(b=a); return b;");
add("nested","const left=a?undefined:false; const value=(left||b)&&(a?null:true); return value===null?a:b;");
add("snapshot","const value=(a?null:b)||(b=a,true); return (value===true?1:0)+b;");
add("effects-once","const chosen=(b++,a?null:false)||(b++,true); return chosen===true?b:0;");
add("boolean-binding","const flag=a<b; const value=flag&&b; return value===false?a:b;");
add("scope","const value=a?null:b; {const value=true; value||(a=b);} return (value||false)===false?a:b;");
add("loop","let sum=0; for(let i=0;i<3;i++){const value=(i<1)&&(a?null:b); if(value===false)sum+=a; else sum+=b;} return sum;");
add("alias","const first=a?true:null,second=b?null:5; const selected=first||second; const alias=selected; return alias===null?a:b;");
add("predicate-composition","const value=(a?null:b)||false; return !value?a:b;");
add('assertion-kind','const value=(a?null:b)||true; return value===true?a:b;','const value:boolean=(a?null:b)||true; return (value as string)===true?a:b;');
generalLogical.push(['general-logical-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y){const value=(x?null:y)||true; return value===true?x:y;} export function run(a,b){return f(a,b);}',
 sourceTS:'function f(x:number,y:number):number{const value:number=(x?null:y)||true; return (value as boolean)===true?x:y;} export function run(a:number,b:number):number{return f(a,b);}',
}]);
