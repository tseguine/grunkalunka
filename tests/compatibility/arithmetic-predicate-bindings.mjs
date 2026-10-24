// Primitive conversion is explicit; semantic binding/slot evidence supplies the kind.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-additive-operators
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-multiplicative-operators
export const arithmeticPredicateBindings=[];
const add=(id,js,ts=js,expected)=>arithmeticPredicateBindings.push([`arithmetic-predicate-binding-${id}`,js,ts,expected]);
for(const [op,name] of [['+','add'],['-','subtract'],['*','multiply'],['/','divide'],['%','remainder']]) {
 for(const value of [false,true]) {
  add(`${name}-left-${value}`,`const flag=${value}; return flag${op}a;`);
  add(`${name}-right-${value}`,`const flag=${value}; return a${op}flag;`);
  for(const right of [false,true]) add(`${name}-both-${value}-${right}`,`const left=${value},right=${right}; return left${op}right;`);
 }
 for(const [kind,wrap] of [['as',x=>`(${x} as string)`],['angle',x=>`<number>${x}`],['nonnull',x=>`${x}!`],['satisfies',x=>`(${x} satisfies number)`]])
  add(`${name}-${kind}`,`const flag=true; return flag${op}b;`,`const flag:string=true; return (${wrap('flag')})${op}b;`);
 add(`${name}-depth128`,`const flag=true; return flag${op}b;`,`const flag=true; return flag${'!'.repeat(127)}${op}b;`);
}
add('alias','const flag=a<b; const copy=flag; return copy+b;');
add('initializer-once','let n=0; const flag=n++<1; return (flag+0)*100+n;',undefined,101);
add('capture','let n=0; const flag=n++<1; n=5; return flag*10+n;',undefined,15);
add('numeric-shadow','const flag=true; { const flag=a; return flag+b; }');
add('predicate-shadow','const flag=a; { const flag=false; return flag/b; }');
add('numeric-annotation','const flag=a; return flag+b;','const flag:boolean=a; return flag+b;');
add('predicate-annotation','const flag=true; return flag+b;','const flag:string=true; return flag+b;');
add('left-snapshot','let x=3; const flag=true; const r=x+(flag?(x=10):0); return r*100+x;',undefined,1310);
add('effect-order','let n=0; const r=(n++<1)-(n++<1); return r*100+n;',undefined,102);
add('effectful-left-once','let n=0; const flag=true; const r=(!n++)+flag; return r*100+n;',undefined,201);
add('effectful-right-once','let n=0; const flag=true; const r=flag+(!n++); return r*100+n;',undefined,201);
add('loop','let n=0; for(let i=0;i<3;i++) { const flag=i<2; n=n+flag; } return n;',undefined,2);
add('negative-zero','const flag=false; return flag*(-1);',undefined,-0);
add('false-divisor','const flag=false; return a/flag;');
add('false-remainder','const flag=false; return a%flag;');
add('intrinsic-names','const Infinity=true,NaN=false,Math=Infinity; return Math+NaN;',undefined,1);
add('statement256',`const flag=true;${'0;'.repeat(254)}return flag+a;`);
add('local128',`const ${Array.from({length:128},(_,i)=>`f${i}=true`).join(',')}; return f127+a;`);
arithmeticPredicateBindings.push(['arithmetic-predicate-binding-helper','','',undefined,undefined,{
 sourceJS:'function f(x) { const flag=x<0; return flag+x; } export function run(a,b) { return f(a)+f(b); }',
 sourceTS:'function f(x:number):number { const flag:boolean=x<0; return flag+x; } export function run(a:number,b:number):number { return f(a)+f(b); }',
}]);
export const arithmeticPredicatePromotions=[
 ['const-boolean-arithmetic','export function run(a,b) { const flag=true; return flag+a; }','53b259230950d900a2ffb7d284ab3f07417cd6a78107ed59f2981753586107fa'],
 ['boolean-arithmetic-bare-const','export function run(a,b) { const flag=true; return flag+a; }','53b259230950d900a2ffb7d284ab3f07417cd6a78107ed59f2981753586107fa'],
 ['unary-predicate-binding-reject-arithmetic','export function run(a,b) { const flag=true; return flag+a; }','53b259230950d900a2ffb7d284ab3f07417cd6a78107ed59f2981753586107fa'],
 ['boolean-arithmetic-bare-const-right','export function run(a,b) { const flag=true; return a*flag; }','d7e0407983ce6b01056c2c9de1343657c7bc976382b6b0ccbe66ab55c85059d7'],
];
for(const [id,sourceJS] of arithmeticPredicatePromotions) arithmeticPredicateBindings.push([id,'','',undefined,undefined,{
 sourceJS,sourceTS:id.endsWith('-right')?'export function run(a:number,b:number):number { const flag:boolean=true; return a*flag; }':'export function run(a:number,b:number):number { const flag:boolean=true; return flag+a; }',
}]);
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
 ['mutable','let flag=true; return flag+a;','General bindings require a primitive value position','boolean-values'],
 ['root-conditional','const flag=true; return (a?flag:false)+b;',numeric,'boolean-values'],
 ['root-logical','const flag=true; return (flag&&true)+b;',numeric,'boolean-values'],
 ['root-sequence','const flag=true; return (0,flag)+b;',numeric,'boolean-values'],
 ['self','const flag=flag+a; return a;',reference,'binding-errors'],
 ['forward','const first=flag+a; const flag=true; return a;',reference,'binding-errors'],
 ['tdz','return flag+a; const flag=true;',reference,'binding-errors'],
 ['outscope','{const flag=true;} return flag+a;',reference,'binding-errors'],
 ['unresolved','return clock+a;',reference,'binding-errors'],
 ['const-write','const flag=true; return (flag=false)+a;','assignment to a const binding is not admitted','const-errors'],
 ['unselected-call','const flag=true; return a?flag+b:Math.random()+flag;',expression,'function-calls'],
 ['string','return a+"1";',expression,'string-values'],
 ['local129',`const ${Array.from({length:129},(_,i)=>`f${i}=true`).join(',')}; return f128+a;`,'at most 128 source locals','local-count'],
 ['statement257',`const flag=true;${'0;'.repeat(255)}return flag+a;`,'at most 256 statements','statement-count'],
];
for(const [op,name] of [['+','add'],['-','subtract'],['*','multiply'],['/','divide'],['%','remainder']]) negatives.push([`depth129-${name}`,`const flag=true; return flag${'!'.repeat(128)}${op}b;`,'expression nesting exceeds','expression-depth','ts']);
export const arithmeticPredicateBindingsRejected=negatives.map(([id,body,diagnostic,,extension='js'])=>[`arithmetic-predicate-binding-reject-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic]);
export const arithmeticPredicateRejectionGroups=new Map(negatives.map(([id,,,group])=>[`arithmetic-predicate-binding-reject-${id}`,group]));
