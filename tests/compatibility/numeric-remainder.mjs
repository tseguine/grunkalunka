// Number-only remainder; originals are deliberately promoted with their exact source IDs/bytes.
export const numericRemainderPrograms = [];
numericRemainderPrograms.push(["gap-remainder", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return a % b; }","sourceTS":"export function run(a,b) { return a % b; }","oracleSource":"export function run(a,b) { return a % b; }"}]);
numericRemainderPrograms.push(["remainder", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return a % b; }","sourceTS":"export function run(a,b) { return a % b; }"}]);
numericRemainderPrograms.push(["bit-not-remainder-remains", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return ~(a%b); }","sourceTS":"export function run(a,b) { return ~(a%b); }"}]);
numericRemainderPrograms.push(["bitwise-remainder-remains", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return (a%b) ^ 1; }","sourceTS":"export function run(a,b) { return (a%b) ^ 1; }"}]);
numericRemainderPrograms.push(["shift-remainder-remains", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return (a%b) >>> 1; }","sourceTS":"export function run(a,b) { return (a%b) >>> 1; }"}]);
numericRemainderPrograms.push(["unsupported-compound-0", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return a %= b; }","sourceTS":"export function run(a,b) { return a %= b; }"}]);
numericRemainderPrograms.push(["compound-unsupported-unselected", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return a?a:(b%=a); }","sourceTS":"export function run(a,b) { return a?a:(b%=a); }"}]);
numericRemainderPrograms.push(["compound-remainder-remains", '', '', undefined, undefined, {"sourceJS":"export function run(a,b) { return a&=(b%=a); }","sourceTS":"export function run(a,b) { return a&=(b%=a); }"}]);
export const remainderReference = {
  "id": "gap-remainder",
  "category": "operators",
  "milestone": "L4",
  "oracleSource": "export function run(a,b) { return a % b; }",
  "args": [
    4,
    2
  ],
  "entry": "run",
  "expected": 0,
  "prerequisites": []
};

const cases = [
 ['remainder-left-snapshot', 'return a % (a=b);'],
 ['remainder-left-write', 'return (a=b) % a;'],
 ['remainder-right-after-left', 'let x=a; return (x=b) % (x=a);'],
 ['remainder-assign-target-snapshot', 'let x=5; const y=(x%=(x=3)); return x*10+y;',22],
 ['remainder-assign-rhs-once', 'let n=0; let x=5; const y=x%=(n++,3); return n*100+x*10+y;',122],
 ['remainder-yield-snapshot', 'let x=a; return (x%=b)-(x=7);'],
 ['remainder-nested-fixed', 'return (17%5)%3;',2],
 ['remainder-selected', 'return a ? b%3 : a%5;'],
 ['remainder-unselected-zero', 'return 1 ? a : a%0;'],
 ['remainder-predicate', 'if(a%3) return a; return b;'],
 ['remainder-finite-for', 'let x=a; for(let i=0;i<2;i++) { x%=3; } return x;'],
 ['remainder-while-test', 'let i=3; while(i%=2) { i--; } return a+i;'],
 ['remainder-do-test', 'let i=3; do { i--; } while(i%=2); return a+i;'],
 ['remainder-for-step', 'for(let i=0;i<2;i=(i+1)%3) { a+=b; } return a;'],
 ['remainder-header-initializer', 'for(let i=5%2;i<2;i++) { a+=b; } return a;'],
 ['remainder-statement', 'a%b; return a;'],
 ['remainder-negative-zero', 'return -4%2;',-0],
 ['remainder-real-quotient', 'return 1%0.1;',0.09999999999999995],
 ['remainder-source-name', 'const number_remainder=a; return number_remainder%b;'],
];
numericRemainderPrograms.push(...cases.map(([id,body,expected])=>[id,body,body,expected]));
numericRemainderPrograms.push(['remainder-ts-wrappers','return a%b;','return ((a as number)%(b as number))!;']);
numericRemainderPrograms.push(['remainder-type-declarations','','',undefined,undefined,{
 sourceJS:'export function run(a,b) { let x=a; return x%=b; }',
 sourceTS:'type Word=number; export function run(a:Word,b:Word):Word { let x:Word=a; return x%=b; type Done=Word; }',
}]);
for (const [id,sourceJS,sourceTS] of [
  ['remainder-direct-helper',
    'function mod(x,y) { return x%y; } export function run(a,b) { return mod(a,b); }',
    'function mod(x:number,y:number):number { return x%y; } export function run(a:number,b:number):number { return mod(a,b); }'],
  ['remainder-recursive-helper',
    'function fold(n,x) { if(n<=0) return x; return fold(n-1,x%3); } export function run(a,b) { return fold(2,a); }',
    'function fold(n:number,x:number):number { if(n<=0) return x; return fold(n-1,x%3); } export function run(a:number,b:number):number { return fold(2,a); }'],
  ['remainder-unused-source-helper',
    'function unused(x,y) { return x%y; } export function run(a,b) { return a+b; }',
    'function unused(x:number,y:number):number { return x%y; } export function run(a:number,b:number):number { return a+b; }'],
]) numericRemainderPrograms.push([id,'','',undefined,undefined,{sourceJS,sourceTS}]);
