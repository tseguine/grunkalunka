export const literalCallSpreadOriginals=[["helper","function f(x){return x;}export function run(a,b){return f(...[a]);}"],["runtime","export function run(a,b){let f=x=>x;return f(...[a]);}"]];
export const literalCallSpreadPrograms=[];
for(const[id,source]of [
 ["helper","function f(x){return x;}export function run(a,b){return f(...[a]);}"],
 ["runtime","export function run(a,b){let f=x=>x;return f(...[a]);}"],
 ["sum","function f(x,y){return x+y;}export function run(a,b){return f(...[a,b]);}"],
 ["mixed","export function run(a,b){const f=(x,y)=>x+y;return f(...[a],b);}"],
 ["stored","export function run(a,b){const o={f:(x,y)=>x+y};return o.f(...[a,b]);}"],
 ["empty","function f(x=3){return x;}export function run(a,b){return +f(...[]);}"],
 ["omitted","function f(x,y){return y;}export function run(a,b){return +f(...[a]);}"],
 ["effects","function f(x){return x;}export function run(a,b){let n=0;const x=f(...[(n=1,a),(n=n*10+2,b)],n=n*10+3);return n;}"],
 ["callee-snapshot","export function run(a,b){let f=x=>x;return f(...[a,f=x=>b]);}"],
 ["call","function f(x,y){return x+y;}export function run(a,b){return f.call(null,...[a,b]);}"],
 ["nested","function f(x,y){return x+y;}export function run(a,b){return f(...[...[a],b]);}"],
 ["null","function f(x=0){return x;}export function run(a,b){return +f(...[null]);}"],
 ["false","function f(x=0){return x;}export function run(a,b){return +f(...[false]);}"],
 ["undefined","function f(x,y=x){return y;}export function run(a,b){return +f(...[a,undefined]);}"],
 ["identity","function f(x){return x;}export function run(a,b){const o={x:a};return f(...[o])===o?b:a;}"],
 ["callback","export function run(a,b){const invoke=(fn,x)=>fn(x);const f=x=>x+1;return invoke(...[f,a]);}"],
])
 literalCallSpreadPrograms.push(['literal-call-spread-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
