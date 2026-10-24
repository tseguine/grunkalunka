// GetValue still checks the original binding; discarded values expose no new ABI.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-expression-statement-runtime-semantics-evaluation
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-comma-operator-runtime-semantics-evaluation
export const discardedPredicateBindings=[];
const add=(id,js,ts=js,expected)=>discardedPredicateBindings.push([`discarded-predicate-binding-${id}`,js,ts,expected]);
const body=(context,operand)=>({
 statement:`${operand}; return a;`,
 'comma-prefix':`${operand},a; return a;`,
 'for-init':`let i=0; for(${operand};i<3;i++) {} return i;`,
 'for-update':`let i=0; for(;i<3;${operand}) { i++; } return i;`,
})[context];
for(const context of ['statement','comma-prefix','for-init','for-update']) {
 for(const value of [false,true])add(`${context}-${value}`,`const flag=${value}; `+body(context,'flag'));
 for(const [kind,wrap] of [['as',x=>`(${x} as number)`],['angle',x=>`<string>${x}`],['nonnull',x=>`${x}!`],['satisfies',x=>`(${x} satisfies number)`]])
  add(`${context}-${kind}`,'const flag=true; '+body(context,'flag'),'const flag:string=true; '+body(context,wrap('flag')));
 const count=context==='comma-prefix'?127:128;
 add(`${context}-depth128`,'const flag=true; '+body(context,'flag'),'const flag=true; '+body(context,'flag'+'!'.repeat(count)));
}
add('parenthesized','const flag=true; ((flag)); return a;');
add('alias','const flag=a<b; const copy=flag; copy; return +copy;');
add('initializer-once','let n=0; const flag=n++<1; flag; return n;',undefined,1);
add('capture','let n=0; const flag=n++<1; n=5; flag; return +flag*100+n;',undefined,105);
add('inline-effect-once','let n=0; const flag=true; n++<1; flag; return n;',undefined,1);
add('effect-order','let n=0; const flag=true; flag,n++,flag,n++; return n;',undefined,2);
add('numeric-shadow','const flag=true; { const flag=a; flag; } return a;');
add('predicate-shadow','const flag=a; { const flag=false; flag; } return a;');
add('numeric-annotation','const flag=a; flag; return a;','const flag:boolean=a; flag; return a;');
add('predicate-annotation','const flag=true; flag; return a;','const flag:number=true; flag; return a;');
add('intrinsic-names','const Infinity=true,NaN=false,Math=Infinity; Infinity; NaN; Math; return a;');
add('for-both','let n=0; const flag=true; for((flag,n++);n<3;(flag,n++)) {} return n;',undefined,3);
add('for-continue','let n=0; const flag=true; for(;n<3;flag) { n++; continue; } return n;',undefined,3);
add('body-reentry','let n=0; while(n<3) { const flag=(n=n+1)<3; flag; } return n;',undefined,3);
add('existing-logical','let n=0; const flag=false; false&&(n++<1); flag; return n;',undefined,0);
add('existing-mixed-join','let n=0; const flag=true; a?(n++<1):n++; flag; return n;',undefined,1);
add('predicate-comma','const flag=a<b; if((flag,a)) return 1; return 0;');
add('statement256',`const flag=true;flag;${'0;'.repeat(253)}return a;`);
add('local128',`const ${Array.from({length:128},(_,i)=>`f${i}=true`).join(',')}; f127; return a;`);
discardedPredicateBindings.push(['discarded-predicate-binding-helper','','',undefined,undefined,{
 sourceJS:'function f(x) { const flag=x<0; flag; return x; } export function run(a,b) { return f(a)+f(b); }',
 sourceTS:'function f(x:number):number { const flag:boolean=x<0; flag; return x; } export function run(a:number,b:number):number { return f(a)+f(b); }',
}]);
export const discardedPredicatePromotions=[
 ['const-boolean-statement','export function run(a,b) { const flag=true; flag; return a; }','41cdf1262f1a0f7b30ba299fb241a0d9c9239710c3fd361d075a3ccb7524e1b3'],
 ['unary-predicate-binding-reject-discard','export function run(a,b) { const flag=true; flag; return a; }','41cdf1262f1a0f7b30ba299fb241a0d9c9239710c3fd361d075a3ccb7524e1b3'],
 ['arithmetic-predicate-binding-reject-discard','export function run(a,b) { const flag=true; flag; return a; }','41cdf1262f1a0f7b30ba299fb241a0d9c9239710c3fd361d075a3ccb7524e1b3'],
 ['const-boolean-comma-prefix','export function run(a,b) { const flag=true; return (flag,a); }','6704f919e3e4fd74709f1d18023152f813fd9b13959b3022c4a8fa5ee5e29330'],
 ['const-boolean-for-update','export function run(a,b) { const flag=true; for(let i=0;i<1;flag) { i++; } return a; }','4a0c87c13b1536b9c873f1a3da6283639a9cffce68df1b38570b64c7e9967e40'],
];
for(const [id,sourceJS] of discardedPredicatePromotions)discardedPredicateBindings.push([id,'','',undefined,undefined,{
 sourceJS,sourceTS:id==='const-boolean-comma-prefix'?'export function run(a:number,b:number):number { const flag:boolean=true; return (flag,a); }':id==='const-boolean-for-update'?'export function run(a:number,b:number):number { const flag:boolean=true; for(let i:number=0;i<1;flag) { i++; } return a; }':'export function run(a:number,b:number):number { const flag:boolean=true; flag; return a; }',
}]);
const numeric='Boolean bindings are not admitted in numeric value positions';
const reference='reference is not an initialized numeric parameter or local';
const expression='expression is outside the numeric experiment';
const negatives=[
 ['root-conditional','const flag=true; a?flag:flag; return a;',numeric,'boolean-values'],
 ['root-sequence','const flag=true; a,flag; return a;',numeric,'boolean-values'],
 ['return','const flag=true; return flag;',numeric,'boolean-values'],
 ['call','const flag=true; return Math.abs(flag);',numeric,'boolean-values'],
 ['assignment','const flag=true; a=flag; return a;','General bindings require a primitive value position','boolean-values'],
 ['assignment-result','const flag=true; (a=flag); return a;','General bindings require a primitive value position','boolean-values'],
 ['compound','const flag=true; a+=flag; return a;',numeric,'boolean-values'],
 ['equality','const flag=true; return flag==1?1:0;',numeric,'coercive-equality'],
 ['bitwise','const flag=true; return flag|a;',numeric,'boolean-values'],
 ['shift','const flag=true; return flag<<a;',numeric,'boolean-values'],
 ['self','const flag=flag; flag; return a;',reference,'binding-errors'],
 ['tdz','flag; const flag=true; return a;',reference,'binding-errors'],
 ['shadow-tdz','const flag=true; { flag; const flag=false; } return a;',reference,'binding-errors'],
 ['outscope','{const flag=true;} flag; return a;',reference,'binding-errors'],
 ['unresolved','clock; return a;',reference,'binding-errors'],
 ['const-write','const flag=true; flag=false; return a;','assignment to a const binding','const-errors'],
 ['for-update-dead-call','const flag=true; for(;false;(flag,Math.random())) {} return a;',expression,'function-calls'],
 ['string','("x" as boolean); return a;',expression,'string-values','ts'],
 ['local129',`const ${Array.from({length:129},(_,i)=>`f${i}=true`).join(',')}; f128; return a;`,'at most 128 source locals','local-count'],
 ['statement257',`const flag=true;flag;${'0;'.repeat(254)}return a;`,'at most 256 statements','statement-count'],
];
for(const context of ['statement','comma-prefix','for-init','for-update']) {
 const count=context==='comma-prefix'?128:129;
 negatives.push([`${context}-depth129`,'const flag=true; '+body(context,'flag'+'!'.repeat(count)),'expression nesting exceeds','expression-depth','ts']);
}
export const discardedPredicateBindingsRejected=negatives.map(([id,source,diagnostic,,extension='js'])=>[`discarded-predicate-binding-reject-${id}`,`export function run(a,b) { ${source} }`,extension,diagnostic]);
export const discardedPredicateRejectionGroups=new Map(negatives.map(([id,,,group])=>[`discarded-predicate-binding-reject-${id}`,group]));
