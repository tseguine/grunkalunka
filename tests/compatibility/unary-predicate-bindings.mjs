// Unary bridges use runtime Predicate slot evidence, never type-annotation spelling.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-plus-operator
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-minus-operator
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-bitwise-not-operator
export const unaryPredicateBindings=[];
const add=(id,js,ts=js,expected)=>unaryPredicateBindings.push([`unary-predicate-binding-${id}`,js,ts,expected]);
for(const [op,name] of [['+','plus'],['-','minus'],['~','complement']]) {
  for(const value of [false,true]) add(`${name}-${value}`,`const flag=${value}; return ${op}flag;`,undefined,op==='+'?Number(value):op==='-'?-Number(value):~Number(value));
  add(`${name}-alias`,`const first=a<b; const second=first; return ${op}second;`);
  for(const [kind,wrap] of [['as',x=>`(${x} as number)`],['angle',x=>`<string>${x}`],['nonnull',x=>`${x}!`],['satisfies',x=>`(${x} satisfies number)`]])
    add(`${name}-${kind}`,`const flag=true; return ${op}flag;`,`const flag:number=true; return ${op}(${wrap('flag')});`);
  add(`${name}-depth128`,`const flag=true; return ${op}flag;`,`const flag=true; return ${op}flag${'!'.repeat(127)};`);
}
add('former-bit-not-guard','const x=true; return ~x;',undefined,-2);
add('initializer-once','let n=0; const flag=n++<1; const result=+flag; return result*100+n;',undefined,101);
add('captured','let n=0; const flag=n++<1; n=5; return ~flag*10+n;',undefined,-15);
add('effectful-not-once','let n=0; const flag=true; const result=+!n++; return result*100+n;',undefined,101);
add('numeric-shadow','const flag=true; { const flag=a; return +flag; }');
add('predicate-shadow','const flag=a; { const flag=false; return -flag; }',undefined,-0);
add('numeric-annotation','const flag=a; return ~flag;','const flag:boolean=a; return ~flag;');
add('predicate-annotation','const flag=true; return +flag;','const flag:string=true; return +flag;');
add('intrinsic-names','const Infinity=true,NaN=false,Math=Infinity; return +Math+-NaN+~Infinity;',undefined,-1);
add('intrinsic-number','return +Infinity;');
add('number-identity','const flag=a; return -flag;');
add('loop','let n=0; for(let i=0;i<3;i++) { const flag=i<2; n=n+ +flag; } return n;',undefined,2);
add('negated-binding','const flag=a<b; return -!flag;');
add('selected','const flag=a<b; return a?+flag:~flag;');
add('statement256',`const flag=true;${'0;'.repeat(254)}return +flag;`,undefined,1);
unaryPredicateBindings.push(['unary-predicate-binding-helper','','',undefined,undefined,{
 sourceJS:'function f(x) { const flag=x<0; return ~flag; } export function run(a,b) { return f(a); }',
 sourceTS:'function f(x:number):number { const flag:boolean=x<0; return ~flag; } export function run(a:number,b:number):number { return f(a); }',
}]);
export const unaryPredicatePromotions=[
 ['const-boolean-complement','export function run(a,b) { const flag=true; return ~flag; }','15bb5a3d49e746839f343f7ff6c1acf81214d1556e1be1524a761426cf61676a'],
 ['const-boolean-plus','export function run(a,b) { const flag=true; return +flag; }','ee414b1a2c5e0d24c34277492afbfbbeb9eebe621fd9d4624400943d5f75da17'],
 ['boolean-numeric-unary-reject-identifier','export function run(a,b) { const flag=true; return +flag; }','ee414b1a2c5e0d24c34277492afbfbbeb9eebe621fd9d4624400943d5f75da17'],
];
for(const [id,sourceJS] of unaryPredicatePromotions) {
 const sourceTS=id==='const-boolean-complement'?'export function run(a:number,b:number):number { const flag:boolean=true; return ~flag; }':'export function run(a:number,b:number):number { const flag:boolean=true; return +flag; }';
 unaryPredicateBindings.push([id,'','',undefined,undefined,{sourceJS,sourceTS}]);
}
const numeric='Boolean bindings are not admitted in numeric value positions';
const expression='expression is outside the numeric experiment';
const reference='reference is not an initialized numeric parameter or local';
const negatives=[
 ['return','const flag=true; return flag;',numeric,'boolean-values'],
 ['call','const flag=true; return Math.abs(flag);',numeric,'boolean-values'],
 ['assignment','const flag=true; a=flag; return a;',numeric,'boolean-values'],
 ['equality','const flag=true; return flag==1?1:0;',numeric,'coercive-equality'],
 ['bitwise','const flag=true; return flag|a;',numeric,'boolean-values'],
 ['shift','const flag=true; return flag<<a;',numeric,'boolean-values'],
 ['compound','const flag=true; a+=flag; return a;',numeric,'boolean-values'],
 ['self','const flag=+flag; return a;',reference,'binding-errors'],
 ['forward','const first=+flag; const flag=true; return a;',reference,'binding-errors'],
 ['tdz','return +flag; const flag=true;',reference,'binding-errors'],
 ['outscope','{const flag=true;} return +flag;',reference,'binding-errors'],
 ['unresolved','return +clock;',reference,'binding-errors'],
 ['const-write','const flag=true; return +(flag=false);','assignment to a const binding is not admitted','const-errors'],
 ['skipped-call','const flag=true; return +!(0&&Math.random());',expression,'function-calls'],
 ['local129',`const ${Array.from({length:129},(_,i)=>`f${i}=true`).join(',')}; return +f128;`,'at most 128 source locals','local-count'],
 ['statement257',`const flag=true;${'0;'.repeat(255)}return +flag;`,'at most 256 statements','statement-count'],
];
for(const [op,name] of [['+','plus'],['-','minus'],['~','complement']]) negatives.push([`depth129-${name}`,`const flag=true; return ${op}flag${'!'.repeat(128)};`,'expression nesting exceeds','expression-depth','ts']);
export const unaryPredicateBindingsRejected=negatives.map(([id,body,diagnostic,,extension='js'])=>[
 `unary-predicate-binding-reject-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic,
]);
export const unaryPredicateRejectionGroups=new Map(negatives.map(([id,,,group])=>[`unary-predicate-binding-reject-${id}`,group]));
