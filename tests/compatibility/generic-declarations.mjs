// Generic declarations retain unvalidated type metadata; runtime values stay numeric.
export const genericDeclarationPromotions = [
  [
    "annotation-generic-function",
    "export function run(a,b) { return a; }",
    "export function run<T>(a:T,b:T):T { return a; }",
    "165496aea0ed234ff49210bd53f782c61b9ffa92ef98c1490e27c651d44f0a9b"
  ],
  [
    "direct-call-helper-generic",
    "function helper(x) { return x; } export function run(a,b) { return helper(a); }",
    "function helper<T>(x:T) { return x; } export function run(a,b) { return helper(a); }",
    "6553c86bb3829759bf1bae932e2cdfe1d446c40e85128ae89d79997222931035"
  ],
  [
    "type-param",
    "export function run(a,b) { return a; }",
    "export function run<T>(a:number,b:number):number { return a; }",
    "f6843951571f95863861cc61504b8885dee49fd0b48a02507551e76fa8a4e758"
  ]
];
export const genericDeclarations = genericDeclarationPromotions.map(([id,sourceJS,sourceTS]) => [id,'','',undefined,undefined,{sourceJS,sourceTS}]);
genericDeclarations.push(["generic-declaration-reject-helper-call",'', '',undefined,undefined,{sourceJS:"function id(x) { return x; }export function run(a,b) { return id(a); }",sourceTS:"function id<T>(x:T):T { return x; }export function run(a,b) { return id<number>(a); }"}]);
function add(id,js,ts){genericDeclarations.push([`generic-declaration-${id}`,'','',undefined,undefined,{sourceJS:js,sourceTS:ts}]);}
const identity='export function run(a,b) { return a; }';
const parameters=['<T>','<T,U>','<T extends number>','<T=number>','<const T>','<T extends number=number,U=T>'];
for(const [i,p] of parameters.entries()){
 add(`entry-${i}`,'export function run(a,b) { return a+b; }',`export function run${p}(a:T,b:T):T { return a+b; }`);
 add(`helper-${i}`,'function id(x) { return x; }export function run(a,b) { return id(a)+b; }',`function id${p}(x:T):T { return x; }export function run<A>(a:A,b:A):A { return id(a)+b; }`);
}
for(const [id,js,ts] of [
 ['capture','export function run(a,b) { let x=a; return add(x,x=b)+x; }function add(x,y) { return x+y; }','export function run<T>(a:T,b:T):T { let x=a; return add(x,x=b)+x; }function add<T,U>(x:T,y:U):T { return x+y; }'],
 ['recursion','function sum(x) { if(x<=0) return 0; return x+sum(x-1); }export function run(a,b) { return sum(3)+b; }','function sum<T>(x:T):T { if(x<=0) return 0; return x+sum(x-1); }export function run<T>(a:T,b:T):T { return sum(3)+b; }'],
 ['math-namespace','export function run(a,b) { return Math.abs(a)+b; }','export function run<Math,T>(a:T,b:T):T { return Math.abs(a)+b; }'],
 ['misleading','export function run(a,b) { return a-b; }','export function run<T=string>(a:T,b:T):T { return a-b; }'],
 ['type-alias','export function run(a,b) { return a*b; }','type Input=number;export function run<T=Input>(a:T,b:T):T { type Local=T;interface Shape { x:T } return a*b; }'],
 ['type-module',identity,'import type { T } from "missing";export function run<U=T>(a:U,b:U):U { return a; }export type { T };'],
])add(id,js,ts);
add('depth',identity,`export function run<T>(a:T,b:T):T { return ${'('.repeat(128)}a${')'.repeat(128)}; }`);
add('statements','export function run(a,b) { '+ 'a=a;'.repeat(255)+'return a; }','export function run<T>(a:T,b:T):T { '+'a=a;'.repeat(255)+'return a; }');
function bytes(extra=0){const prefix='export function run<T extends "λ',suffix='">(a:T,b:T):T { return a; }';return prefix+'x'.repeat(16384+extra-Buffer.byteLength(prefix+suffix))+suffix;}
add('bytes',identity,bytes());
export const genericDeclarationsRejected=[];
export const genericDeclarationGroups=new Map();
function bad(id,source,diagnostic,group){id=`generic-declaration-reject-${id}`;genericDeclarationsRejected.push([id,source,'ts',diagnostic]);genericDeclarationGroups.set(id,group);}
for(const [id,source,diagnostic,group] of [
 ['async','export async function run<T>(a:T,b:T):T { return a; }','plain, synchronous function','async-functions'],
 ['generator','export function* run<T>(a:T,b:T):T { return a; }','plain, synchronous function','generators'],
 ['declare','export declare function run<T>(a:T,b:T):T;','plain, synchronous function','typescript-runtime-forms'],
 ['this','export function run<T>(this:T,a:T,b:T):T { return a; }','plain, synchronous function','parameters'],
 ['arity','export function run<T>(a:T):T { return a; }','exactly two ordinary','parameters'],
 ['default','export function run<T>(a:T,b:T=1):T { return a; }','default','parameters'],
 ['optional','export function run<T>(a?:T,b?:T):T { return a; }','parameter modifiers','parameters'],
 ['rest','export function run<T>(a:T,...b:T[]):T { return a; }','exactly two ordinary','parameters'],
 ['type-value','export function run<T>(a:T,b:T):T { return T; }','reference is not an initialized','binding-errors'],
 ['type-tdz','export function run<T>(a:T,b:T):T { const T=T;return T; }','reference is not an initialized','binding-errors'],
 ['const-write','export function run<T>(a:T,b:T):T { const x=a;x=b;return x; }','assignment to a const binding is not admitted','const-errors'],
 ['unused-helper','function unused<T>(x:T):T { if(false) return Math.random();return x; }'+identity,'expression is outside','function-calls'],
 ['parse','export function run<T extends >(a:T,b:T) { return a; }','parse diagnostics','malformed-ts'],
 ['duplicate','export function run<T>(a:T,b:T) { const x=a;const x=b;return x; }','semantic diagnostics','duplicate-lexical-binding'],
 ['statements','export function run<T>(a:T,b:T):T { '+'a=a;'.repeat(256)+'return a; }','256 statements','statement-count'],
 ['bytes',bytes(1),'16 KiB','source-bytes'],
 ['depth',`export function run<T>(a:T,b:T):T { return ${'('.repeat(129)}a${')'.repeat(129)}; }`,'expression nesting exceeds','expression-depth'],
])bad(id,source,diagnostic,group);
