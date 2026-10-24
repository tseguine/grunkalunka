// Primitive Boolean operands convert to 0/1 only for binary + - * / %.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-evaluatestringornumericbinaryexpression
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-applystringornumericbinaryoperator
const cases=[];
for(const [name,op] of [['add','+'],['subtract','-'],['multiply','*'],['divide','/'],['remainder','%']]) {
  cases.push(
    [`${name}-left`,`return true${op}a;`],
    [`${name}-right`,`return a${op}false;`],
    [`${name}-both`,`return true${op}false;`],
    [`${name}-comparison`,`return (a<b)${op}b;`],
    [`${name}-negation`,`return a${op}!b;`],
  );
}
cases.push(
  // Verbatim Rust-only former rejection bodies, now direct numeric witnesses.
  ['literal-add','return true+1;',undefined,2],
  ['complement-then-add','return ~(a<b)+true;','return ~((a<b) as number)+true;'],
  ['number-remainder-true','return a%true;'],
  ['order','let x=1; const r=(x++<2)+(x=10); return r*100+x;',undefined,1110],
  ['left-snapshot','let x=3; const r=x+((x=10)>0); return r*100+x;',undefined,410],
  ['both-effects','let x=0; const r=(x++<1)-(x++<2); return x*100+r;',undefined,200],
  ['rhs-once','let x=0; const r=10*(x++<1); return r*100+x;',undefined,1001],
  ['false-times-negative','return false*-1;',undefined,-0],
  ['false-divide-negative','return false/-1;',undefined,-0],
  ['negative-zero-remainder','return -0%true;',undefined,-0],
  ['zero-divide-zero','return false/false;',undefined,NaN],
  ['zero-remainder-zero','return false%false;',undefined,NaN],
  ['comparison-pair','return (a<b)+(a===b);'],
  ['nested','return ((a<b)+true)*((!b)-false);'],
  ['conditional-argument','return (a?b:2)+true;'],
  ['numeric-comma','return (a,b)+true;'],
  ['numeric-logical','return (a||b)*true;'],
  ['arithmetic-logical-result','return (a+true)||b;'],
  ['selected-effects','let x=0; const r=a?true+(x++):false*(x++); return x*100+r;'],
  ['skipped-logical-effects','let x=0; true||(x++<1)+false; false&&(x++<1)+true; return x;',undefined,0],
  ['as','return (a<b)+b;','return ((a<b) as number)+b;'],
  ['angle','return a+true;','return a+(<number>true);'],
  ['non-null','return (!a)*b;','return (!a)!*b;'],
  ['satisfies','return false/b;','return (false satisfies number)/b;'],
  ['annotation-not-proof','return a+b;','return (a as boolean)+(b as boolean);'],
  ['number-shadow','const trueValue=a; return trueValue+false;'],
  ['const-predicate-not','const flag=a<b; return !flag+a;'],
  ['finite-loop','let i=0,x=0; for(;i<3;i++) { x=x+(i<2); } return x;',undefined,2],
  ['remainder-loop','let i=0,x=0; while(i<3) { x=x+(i%true); i++; } return x;',undefined,0],
  ['depth-boundary',`return ${'('.repeat(127)}true${')'.repeat(127)}+a;`],
  ['ts-depth-boundary','return true+a;',`return true${'!'.repeat(127)}+a;`],
  ['local-boundary',`const ${Array.from({length:128},(_,i)=>`x${i}=a+true`).join(',')}; return x127;`],
  ['statement-boundary',`${'true+a;'.repeat(255)}return a;`],
);
export const numericBooleanArithmetic=cases.map(([id,js,ts=js,expected])=>[`boolean-arithmetic-${id}`,js,ts,expected]);
numericBooleanArithmetic.push(['boolean-arithmetic-helper','','',undefined,undefined,{
  sourceJS:'function f(x) { return (x<1)+x; } export function run(a,b) { let x=0; const r=f(x++); return r*100+x; }',
  sourceTS:'function f(x:number):number { return (x<1)+x; } export function run(a:number,b:number):number { let x=0; const r=f(x++); return r*100+x; }',
}]);
numericBooleanArithmetic.push(['boolean-arithmetic-remainder-conditional','','',undefined,undefined,{
  sourceJS:'function f(x) { return x?false%true:0; } export function run(a,b) { return f(a); }',
  sourceTS:'function f(x:number):number { return x?false%true:0; } export function run(a:number,b:number):number { return f(a); }',
}]);

// Six exact original source identities. TypeScript's JS oracle is hand-paired.
export const booleanArithmeticPromotions=[
  ['ts-erasure-boolean-arithmetic','export function run(a,b) { return (a < b) + b; }','export function run(a:number,b:number):number { return ((a < b) as boolean) + b; }','ts','5f87d094693f2f9e12fe4a6e4e406a0f53eae085ace924f15046d75d94fc99a6'],
  ['inequality-arithmetic','export function run(a,b) { return (a!=b)+a; }','export function run(a:number,b:number):number { return (a!=b)+a; }','js','678a01439b2b1a96fccc894b2d61493ca551541c1bba0eaed75d163e8edd5947'],
  ['discarded-number-arithmetic','export function run(a,b) { return true+a; }','export function run(a:number,b:number):number { return true+a; }','js','393decb102698f037696aa9f899e9026ecb7854ad8f8fa9602312628f22e65a2'],
  ['boolean-literal-arithmetic','export function run(a,b) { return true+b; }','export function run(a:number,b:number):number { return true+b; }','js','a08ce43017a57a56ec9466eef57afa9352e3e88049b0d38173569fa7c6cd632a'],
  ['not-arithmetic','export function run(a,b) { return (!a)+b; }','export function run(a:number,b:number):number { return (!a)+b; }','js','21e071dd23edb24e65665f17fcb0ce03610943362be27f5cde9ab05327d22d4a'],
  ['boolean-arithmetic','export function run(a,b) { return (a<b)+b; }','export function run(a:number,b:number):number { return (a<b)+b; }','js','acd423e440aed95754dddc8f40527d91748f886ff5ad5c09eb19b3abc318929e'],
];
for(const[id,sourceJS,sourceTS]of booleanArithmeticPromotions)numericBooleanArithmetic.push([id,'','',undefined,undefined,{sourceJS,sourceTS}]);

const expression='expression is outside the numeric experiment';
const binary='only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const numeric='Boolean bindings are not admitted in numeric value positions';
const reference='reference is not an initialized numeric parameter or local';
const negatives=[
  ['bitwise','return a&true;',expression],
  ['shift','return true<<a;',expression],
  ['string','return a+"1";',expression],
  ['object','return {}+true;',expression],
  ['bigint','return true+1n;',expression],
  ['unselected-call','return false?Math.random()+false:1;',expression],
  ['unselected-arithmetic-call','return a?1:Math.random()+true;',expression],
  ['dead-helper-call','if(false) { const x=true+Math.random(); } return a;',expression],
  ['const-write','const x=a; return true+(x=b);','assignment to a const binding'],
  ['tdz','return true+x; let x=a;',reference],
  ['self','const x=(x<1)+true; return a;',reference],
  ['depth-limit',`return ${'('.repeat(128)}true${')'.repeat(128)}+a;`,'expression nesting exceeds'],
  ['ts-depth-limit',`return true${'!'.repeat(128)}+a;`,'expression nesting exceeds','ts'],
  ['predicate-depth-limit',`return ${'!'.repeat(128)}a+true;`,'predicate nesting exceeds'],
  ['local-limit',`const ${Array.from({length:129},(_,i)=>`x${i}=a+true`).join(',')}; return a;`,'at most 128 source locals'],
  ['statement-limit',`${'true+a;'.repeat(256)}return a;`,'at most 256 statements'],
  ['asserted-string','return ("x" as boolean)+true;',expression,'ts'],
];
export const numericBooleanArithmeticRejected=negatives.map(([id,body,diagnostic,ext='js'])=>[
  `boolean-arithmetic-${id}`,`export function run(a,b) { ${body} }`,ext,diagnostic,
]);
