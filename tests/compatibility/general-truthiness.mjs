// ECMA-262 ToBoolean over the four represented primitive kinds.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean
export const generalTruthiness=[];
const add=(id,js,ts=js)=>generalTruthiness.push([`general-truthiness-${id}`,js,ts]);
add("null-condition","if(null) return a; return b;");
add("undefined-condition","if(undefined) return a; return b;");
add("not-null","return +!null;");
add("not-undefined","return +!undefined;");
add("mixed-binding","const value=a?null:b; return value?a:b;");
add("boolean-binding","const value=a?true:undefined; return !value?a:b;");
add("nested-predicate","return (a?null:(b?undefined:false))?1:0;");
add("and-effects","let n=0; if(null&&(n=1)) n+=10; return n;");
add("or-effects","let n=0; if(undefined||(n=2)) n+=10; return n;");
add("sequence-test","let n=0; if((n+=1,a?null:b)) n+=10; return n;");
add("while","const stop=null; while(stop) {a+=1;} return a;");
add("for","for(;undefined;) {a+=1;} return a;");
add("do","do {a+=b;} while(null); return a;");
add("shadow-undefined","const undefined=a?null:b; return undefined?1:0;");
add('assertion-kind','const value=a?null:b; return value?a:b;','const value:number=a?null:b; return (value as string)?a:b;');
generalTruthiness.push(['general-truthiness-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y) {const value=x?null:y; return value?x:y;} export function run(a,b) {return f(a,b);}',
 sourceTS:'function f(x:number,y:number):number {const value:boolean=x?null:y; return (value as number)?x:y;} export function run(a:number,b:number):number {return f(a,b);}',
}]);
