export const unaryDefaultOriginal='function f(x,y=+x){return y;}export function run(a,b){return f(a);}';
export const unaryDefaultPrograms=[];
for(const[id,source]of[
 ['plus',unaryDefaultOriginal],
 ['void','function f(x,y=void x){return y;}export function run(a,b){return +f(a);}'],
 ['negative','function f(x,y=-x){return y;}export function run(a,b){return f(a);}'],
 ['null','function f(x,y=-x){return y;}export function run(a,b){return f(null);}'],
 ['false','function f(x,y=+x){return y;}export function run(a,b){return f(false);}'],
 ['undefined','function f(x,y=+x){return y;}export function run(a,b){return f();}'],
 ['not','function f(x,y=!x){return y;}export function run(a,b){return +f(a);}'],
 ['empty-string',"function f(x,y=!x){return y;}export function run(a,b){return +f('');}"],
 ['object','function f(x,y=!x){return y;}export function run(a,b){return +f({});}'],
 ['coerce-boolean',"function f(x,y=+!!x){return y;}export function run(a,b){return f('abc');}"],
 ['void-array','function f(x,y=+void x){return y;}export function run(a,b){return f([]);}'],
 ['emitted','export function run(a,b){const o={f:(x,y=-x)=>y};return +o.f(a);}'],
 ['capture','export function run(a,b){const f=(x,y=-x)=>()=>y;const g=f(a);return +g();}'],
 ['supplied','function f(x,y=-x){return y;}export function run(a,b){return f(a,b);}'],
 ['snapshot','export function run(a,b){const f=(x,y=-x)=>y;return +f(a,undefined,a=b);}'],
])unaryDefaultPrograms.push(['unary-default-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
