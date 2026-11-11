export const debuggerStatementPrograms=[];
for(const[id,source]of [
 ["module","debugger;export function run(a,b){return a+b;}"],
 ["direct","export function run(a,b){debugger;return a+b;}"],
 ["branches","export function run(a,b){if(a<0){debugger;}else{debugger;}return b;}"],
 ["loop","export function run(a,b){let n=0;for(let i=0;i<3;i++){debugger;n+=i;}return n;}"],
 ["helper","function f(x){debugger;return x;}export function run(a,b){return f(a);}"],
 ["arrow","export function run(a,b){const f=x=>{debugger;return x+b;};return f(a);}"],
 ["catch","export function run(a,b){try{debugger;throw a;}catch(e){debugger;return +e;}}"],
 ["finally","export function run(a,b){try{debugger;return a;}finally{debugger;}}"],
 ["switch","export function run(a,b){switch(a){case 0:debugger;break;default:debugger;}return b;}"],
])debuggerStatementPrograms.push(['debugger-statement-'+id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
