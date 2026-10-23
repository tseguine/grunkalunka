// Boolean operands for numeric ~ only; original full sources/IDs are preserved.
// ECMA-262 Number::bitwiseNOT and ToNumber(Boolean):
// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-bitwiseNOT
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
export const numericBooleanComplement = [];
numericBooleanComplement.push(["bit-not-boolean",'','',undefined,undefined,{"sourceJS": "export function run(a,b) { return ~true; }", "sourceTS": "export function run(a,b) { return ~true; }"}]);
numericBooleanComplement.push(["bit-not-comparison",'','',undefined,undefined,{"sourceJS": "export function run(a,b) { return ~(a<b); }", "sourceTS": "export function run(a,b) { return ~(a<b); }"}]);
numericBooleanComplement.push(["bit-not-negation",'','',undefined,undefined,{"sourceJS": "export function run(a,b) { return ~(!a); }", "sourceTS": "export function run(a,b) { return ~(!a); }"}]);
numericBooleanComplement.push(["bit-not-asserted-boolean",'','',undefined,undefined,{"sourceJS": "export function run(a,b) { return ~true; }", "sourceTS": "export function run(a,b) { return ~(true as number); }"}]);

const add=(id,js,ts=js,expected)=>numericBooleanComplement.push([id,js,ts,expected]);
add('boolean-complement-false','return ~false;',undefined,-1);
add('boolean-complement-true','return ~true;',undefined,-2);
for(const [i,operator]of ['<','<=','>','>=','==','!=','===','!=='].entries())
 add(`boolean-complement-compare-${i}`,`return ~(a${operator}b);`);
add('boolean-complement-not-number','return ~!a;');
add('boolean-complement-not-compare','return ~!(a<b);');
add('boolean-complement-double-not','return ~!!a;');
add('boolean-complement-once','let x=0; const y=~(x++<1); return x*100+y;',undefined,98);
add('boolean-complement-negation-once','let x=0; const y=~!(x++); return x*100+y;',undefined,98);
add('boolean-complement-ordered','let x=a; const y=~((x=b)<(x=a)); return y+x;');
add('boolean-complement-number','return ~2;',undefined,-3);
add('boolean-complement-infinity','return ~Infinity;',undefined,-1);
add('boolean-complement-nan','return ~NaN;',undefined,-1);
add('boolean-complement-minuszero','return ~-0;',undefined,-1);
add('boolean-complement-number-assertion','return ~a;','return ~(a as boolean);');
add('boolean-complement-literal-number-assertion','return ~2;','return ~(2 as boolean);',-3);
add('boolean-complement-shadow','let Infinity=2; return ~Infinity;',undefined,-3);
for(const [name,wrap]of [['as',x=>`(${x} as number)`],['angle',x=>`<number>${x}`],
 ['nonnull',x=>`(${x}!)`],['satisfies',x=>`(${x} satisfies number)`]])
 add(`boolean-complement-wrapper-${name}`,'return ~true;',`return ~(${wrap('true')});`,-2);
add('boolean-complement-wrapped-compare','return ~(a<b);','return ~(((a as number)<b) as number);');
add('boolean-complement-not-predicate-domain','return ~!(a?true:false);');
add('boolean-complement-not-logical-domain','return ~!(a&&b);');
add('boolean-complement-number-logical','return ~(a&&b);');
add('boolean-complement-number-ternary','return ~(a?b:2);');
add('boolean-complement-depth128','return ~true;',`return ~true${'!'.repeat(127)};`,-2);
add('boolean-complement-inner-paren-depth128','return ~true;',`return ~(true${'!'.repeat(126)});`,-2);
add('boolean-complement-loop','let x=0; for(let i=0;i<2;i++) x+=~(i<1); return x;',undefined,-3);
numericBooleanComplement.push(['boolean-complement-helper','','',undefined,undefined,{
 sourceJS:'function f(x) { return ~!x; } export function run(a,b) { return f(a); }',
 sourceTS:'function f(x:number):number { return ~(!x as number); } export function run(a:number,b:number):number { return f(a); }',
}]);
