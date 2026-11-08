export const mixedLiteralArraySpreadOriginal="export function run(a,b){const x=[a,...[b]];return a;}";
export const mixedLiteralArraySpreadPrograms=[
 ["mixed-literal-array-spread-values","const xs=[a,...[b,3],4];return xs[0]+xs[1]+xs[2]+xs[3];","const xs=[a,...[b,3],4];return xs[0]+xs[1]+xs[2]+xs[3];"],
 ["mixed-literal-array-spread-empty","const xs=[a,...[],b];return xs.length===2&&xs[0]===a&&xs[1]===b?1:0;","const xs=[a,...[],b];return xs.length===2&&xs[0]===a&&xs[1]===b?1:0;"],
 ["mixed-literal-array-spread-holes","const xs=[,...[,undefined,a],,b];return xs.length===6&&!Object.hasOwn(xs,0)&&Object.hasOwn(xs,1)&&xs[1]===undefined&&Object.hasOwn(xs,2)&&!Object.hasOwn(xs,4)&&xs[5]===b?1:0;","const xs=[,...[,undefined,a],,b];return xs.length===6&&!Object.hasOwn(xs,0)&&Object.hasOwn(xs,1)&&xs[1]===undefined&&Object.hasOwn(xs,2)&&!Object.hasOwn(xs,4)&&xs[5]===b?1:0;"],
 ["mixed-literal-array-spread-nested","const xs=[a,...[...[b],3],4];return xs[2];","const xs=[a,...[...[b],3],4];return xs[2];"],
 ["mixed-literal-array-spread-function-position","const xs=[a,...[b,b],x=>x+1];return xs[3](a);","const xs=[a,...[b,b],x=>x+1];return xs[3](a);"],
 ["mixed-literal-array-spread-string-position","const xs=[a,...['x',' y '],b];return xs[2].trim().length;","const xs=[a,...['x',' y '],b];return xs[2].trim().length;"],
 ["mixed-literal-array-spread-reference","const child={v:a};const xs=[b,...[child,child],a];xs[1].v=b;return xs[1]===xs[2]&&child.v===b?1:0;","const child={v:a};const xs=[b,...[child,child],a];xs[1].v=b;return xs[1]===xs[2]&&child.v===b?1:0;"],
 ["mixed-literal-array-spread-effects","let n=0;const xs=[(n=1,a),...[(n=n*10+2,b),(n=n*10+3,a)],(n=n*10+4,b)];return n;","let n=0;const xs=[(n=1,a),...[(n=n*10+2,b),(n=n*10+3,a)],(n=n*10+4,b)];return n;"],
 ["mixed-literal-array-spread-multiple","const xs=[a,...[b,...[]],...[3],4];return xs.length*10+xs[2];","const xs=[a,...[b,...[]],...[3],4];return xs.length*10+xs[2];"],
 ["mixed-literal-array-spread-defaults","const xs=[a,...[,],b];const [x,y=9,z]=xs;return y+xs.length;","const xs=[a,...[,],b];const [x,y=9,z]=xs;return y+xs.length;"],
 ["mixed-literal-array-spread-enumeration","const xs=[,...[,undefined]];return Object.keys(xs).length;","const xs=[,...[,undefined]];return Object.keys(xs).length;"],
 ["mixed-literal-array-spread-fresh","const xs=[a,...[b]];const ys=[a,...[b]];return xs!==ys?1:0;","const xs=[a,...[b]];const ys=[a,...[b]];return xs!==ys?1:0;"],
];
mixedLiteralArraySpreadPrograms.push(['mixed-literal-array-spread-original','','',undefined,undefined,{sourceJS:mixedLiteralArraySpreadOriginal,sourceTS:mixedLiteralArraySpreadOriginal}]);
