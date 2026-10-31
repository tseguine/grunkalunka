// Traced source arrays; numeric observations retain the existing public ABI.
const cases = [
  ['managed-array-length', 'const x=[a,b];return x.length;'],
  ['managed-array-holes', 'const x=[a,,undefined,];return x.length===3&&x[1]===undefined&&x[2]===undefined?a:b;'],
  ['managed-array-order', 'let n=0;const x=[(n=n*10+1,a),(n=n*10+2,b),,];return n*10+x.length;'],
  ['managed-array-snapshot', 'const x=[a];const old=x[0];x[0]=b;x[3]=a;return old===a&&x[0]===b&&x.length===4?a:b;'],
  ['managed-array-receiver', 'let n=0;let x=[a];const old=x;x[0]=(x=[b],n++,a);return old!==x&&old[0]===a&&x[0]===b&&n===1?a:b;'],
  ['managed-array-index-boundary', "const x=[];x['01']=a;x[4294967295]=b;x[4294967294]=a;return x.length===4294967295&&x['01']===a&&x[4294967295]===b?a:b;"],
  ['managed-array-truncate', 'const x=[a,b,a];const old=x[2];x.length=1;x.length=3;return x[0]===a&&x[1]===undefined&&x[2]===undefined&&old===a?a:b;'],
  ['managed-array-identity', 'let x=[a];const alias=x;const o={ref:x};const outer=[x];x=[b];return alias!==x&&o.ref===alias&&outer[0]===alias?a:b;'],
  ['managed-array-cycle', 'const x=[];x[0]=x;return x[0]===x?a:b;'],
  ['managed-array-value-kinds', "const o={value:a};const x=[o,'text',null,false,undefined];return x[0]===o&&x[1]==='text'&&x[2]===null&&x[3]===false&&x[4]===undefined?a:b;"],
  ['managed-array-utf16-keys', String.raw`const x=[];x['\ud800\0']=a;x['\ufffd\0']=b;return x['\ud800\0']===a&&x['\ufffd\0']===b&&x.length===0?a:b;`],
  ['managed-array-selected', 'const x=a?[a]:[b];return x&&x!==[]?a:b;'],
  ['managed-array-discard', 'const x=[];return a;'],
  ['managed-array-object-field', 'const o={x:[]};return a;'],
  ['managed-array-string-selection', "return (a?'x':[])===null?a:b;"],
];
export const managedArrays=cases.map(([id,body])=>[id,body,body]);
const sourceJS="function size(x){return x['length'];} export function run(a,b){return size([a,b]);}";
const sourceTS="function size(x:number):string{return x[('length' as string)];} export function run(a:boolean,b:object):void{return size([a,b]);}";
managedArrays.push(['managed-array-helper-erasure','','',undefined,undefined,{sourceJS,sourceTS}]);
