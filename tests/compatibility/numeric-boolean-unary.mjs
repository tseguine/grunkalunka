// ECMA-262 unary + / unary - and ToNumber(Boolean); no general coercion profile.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-plus-operator
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-minus-operator
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
export const numericBooleanUnary = [];
const add=(id,js,ts=js,expected)=>numericBooleanUnary.push([`boolean-numeric-unary-${id}`,js,ts,expected]);
add('plus-false','return +false;',undefined,0);
add('minus-false','return -false;',undefined,-0);
add('plus-true','return +true;',undefined,1);
add('minus-true','return -true;',undefined,-1);
for(const [i,operator] of ['<','<=','>','>=','==','!=','===','!=='].entries())
 add(`compare-${i}`,`return ${i%2?'-':'+'}(a${operator}b);`);
add('not','return +!a;');
add('double-not','return -!!a;');
add('not-conditional-domain','return +!(a?true:false);');
add('not-logical-domain','return -!(a&&b);');
add('once-plus','let x=0; const y=+(x++<1); return x*100+y;',undefined,101);
add('once-minus','let x=0; const y=-(x++<1); return x*100-y;',undefined,101);
add('capture','let x=0; const y=+(x++===0); const z=-(x++===1); return (y-z)*10+x*5;',undefined,30);
add('number-plus','return +a;');
add('number-minus','return -a;');
add('number-plus-assertion','return +a;','return +(a as boolean);');
add('number-minus-assertion','return -a;','return -(a as boolean);');
for(const [name,wrap] of [['as',x=>`(${x} as number)`],['angle',x=>`<number>${x}`],
 ['nonnull',x=>`(${x}!)`],['satisfies',x=>`(${x} satisfies number)`]])
 for(const sign of ['+','-']) add(`${name}-${sign==='+'?'plus':'minus'}`,`return ${sign}true;`,`return ${sign}(${wrap('true')});`);
for(const sign of ['+','-']) {
 const name=sign==='+'?'plus':'minus';
 add(`depth128-${name}`,`return ${sign}true;`,`return ${sign}true${'!'.repeat(127)};`);
 add(`paren-depth128-${name}`,`return ${sign}true;`,`return ${sign}(true${'!'.repeat(126)});`);
}
add('loop-plus','let x=0; for(let i=0;i<3;i++) x+=+(i<2); return x;',undefined,2);
add('loop-minus','let x=0; for(let i=0;i<3;i++) x+=-(i<1); return x;',undefined,-1);
add('shadow','let Infinity=0; return -!Infinity;',undefined,-1);
add('selected','let x=0; const y=+!(a?(x=1):(x=2)); return x*10+y;');
add('statement256',`${'0;'.repeat(255)}return +true;`,undefined,1);
numericBooleanUnary.push(['boolean-numeric-unary-helper','','',undefined,undefined,{
 sourceJS:'function f(x) { return -!x; } export function run(a,b) { return f(a); }',
 sourceTS:'function f(x:number):number { return -(!x as number); } export function run(a:number,b:number):number { return f(a); }',
}]);

const expression='expression is outside the numeric experiment';
const reference='reference is not an initialized numeric parameter or local';
const negatives=[
 ['root-logical','return +(true&&false);',expression,'boolean-values'],
 ['root-conditional','return -(a?true:false);',expression,'boolean-values'],
 ['root-sequence','return +(0,true);',expression,'boolean-values'],
 ['bare-not','return !a;','only unary +, - and ~ are admitted','boolean-values'],
 ['string','return +"1";',expression,'string-values'],
 ['bigint','return -1n;',expression,'bigint-values'],
 ['object','return +({});',expression,'object-values'],
 ['null','return +null;',expression,'nullish-values'],
 ['unselected-call','return +!(0&&Math.random());',expression,'function-calls'],
 ['const-write','const x=0; if(0) return +!(x=1); return 0;','assignment to a const binding is not admitted','const-errors'],
 ['tdz','return -!x; const x=0;',reference,'binding-errors'],
 ['statement257',`${'0;'.repeat(256)}return -false;`,'at most 256 statements','statement-count'],
];
for(const sign of ['+','-']) {
 const name=sign==='+'?'plus':'minus';
 negatives.push([`depth129-${name}`,`return ${sign}true${'!'.repeat(128)};`,'expression nesting exceeds','expression-depth','ts']);
 negatives.push([`paren-depth129-${name}`,`return ${sign}(true${'!'.repeat(127)});`,'expression nesting exceeds','expression-depth','ts']);
}
export const numericBooleanUnaryRejected=negatives.map(([id,body,diagnostic,,extension='js'])=>
 [`boolean-numeric-unary-reject-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic]);
export const numericBooleanUnaryRejectionGroups=new Map(negatives.map(([id,,,group])=>[`boolean-numeric-unary-reject-${id}`,group]));
