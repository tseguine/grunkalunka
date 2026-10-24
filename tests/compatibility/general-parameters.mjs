// Source parameter bindings are initialized once and ordinary assignment captures RHS.
// https://tc39.es/ecma262/multipage/ordinary-and-exotic-objects-behaviours.html#sec-functiondeclarationinstantiation
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
export const generalParameters=[];
const add=(id,js,ts=js)=>generalParameters.push([`general-parameter-${id}`,js,ts]);
add('null','a=null; return +a;');
add('boolean','a=a?false:b; return +a+b;');
add('captured-input','const old=a; a=null; return +old;');
add('parameter-alias','b=a; a=b; b=null; return +a;');
add('numeric-result','a=null; return (a=b++)+b;');
add('unselected-write','if(false)a=null; return +a;');
add('both-parameters','a=null; b=false; return a===null?(b===false?1:0):-1;');
add('logical-result','a=a?null:b; return +(a||b);');
add('write-dependency','a=b; b=null; return +a;');
add('lexical-shadow','{let a=null; a=false;} return a;');
add('return-shadow','a=null; {const a=b; return a;}');
add('loop','for(let i=0;i<3;i++){a=i?a:false;} return +a;');
add('assignment-snapshot','const before=+(a=b); b=null; return before;');
add('local-parameter-cycle','let old=a; a=b; b=old; a=null; return +b;');
add('type-wrapper','a=null; return +a;','a=(null as unknown); return +((a as boolean)!);');
generalParameters.push(['general-parameter-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y,z){const old=x; x=null; y=false; return +old+(+y)+z;} export function run(a,b){return f(a,b,1);}',
 sourceTS:'function f(x:number,y:number,z:number):number{const old=x; x=null; y=false; return +old+(+y)+z;} export function run(a:number,b:number):number{return f(a,b,1);}',
}]);
