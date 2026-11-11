export const arithmeticDefaultOriginals=[
 ['helper','function f(x=1+2){return x;}export function run(a,b){return f();}'],
 ['arrow','export function run(a,b){const f=(x=1+2)=>x;return f();}'],
];
export const arithmeticDefaultPrograms=arithmeticDefaultOriginals.map(([id,source])=>['arithmetic-default-original-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
for(const[id,source]of[
 ['minus','function f(x=7-9){return x;}export function run(a,b){return f();}'],
 ['null-product-original','function f(x=null*3){return x;}export function run(a,b){return f();}'],
 ['negative-null','function f(x=-null){return x;}export function run(a,b){return f();}'],
 ['string-power','export function run(a,b){const f=(x="0x10"**2)=>x;return f();}'],
 ['boolean-remainder','function f(x=true%"3"){return x;}export function run(a,b){return f();}'],
 ['nan-default','function f(x=+"bad"){return x;}export function run(a,b){return f();}'],
 ['supplied-coercion','function f(x=+"42"){return x;}export function run(a,b){return f(a);}'],
 ['multiply','function f(x=2*(3+4)){return x;}export function run(a,b){return f();}'],
 ['divide','function f(x=9/2){return x;}export function run(a,b){return f();}'],
 ['remainder','function f(x=9%4){return x;}export function run(a,b){return f();}'],
 ['signed-zero','function f(x=-4%2){return x;}export function run(a,b){return f();}'],
 ['infinity','function f(x=1/0){return x;}export function run(a,b){return f();}'],
 ['nan','function f(x=Infinity-Infinity){return x;}export function run(a,b){return f();}'],
 ['void','function f(x=void (1+2)){return x;}export function run(a,b){return +f();}'],
 ['supplied','function f(x=1+2){return x;}export function run(a,b){return f(a);}'],
 ['null','function f(x=1+2){return x;}export function run(a,b){return +f(null);}'],
 ['closure','export function run(a,b){const o={f:(x=2*(3+4))=>x};return +o.f();}'],
 ['capture','export function run(a,b){const f=(x=1+2,y=x)=>()=>y;const g=f();return +g();}'],
])arithmeticDefaultPrograms.push(['arithmetic-default-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
