export const inertModuleBlockPrograms=[];
for(const[id,source]of [
 ['empty','{}export function run(a,b){return a+b;}'],
 ['labels','first:debugger;outer:{inner:{0;}}export function run(a,b){return a+b;}first:;'],
 ['nested','{;debugger;0;false;null;"text";`template`;void 0;{1n;}}export function run(a,b){return a+b;}'],
 ['around-helper','{;}function helper(x){return x+1;}{debugger;}export function run(a,b){return helper(a)+b;}{void 0;}'],
])inertModuleBlockPrograms.push(['inert-module-block-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
