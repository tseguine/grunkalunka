// Pristine Number-only intrinsics, not mutable external-realm builtin objects.
// https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.sqrt
// https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.floor
// https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.ceil
// https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.trunc
export const mathUnaryMethods=['sqrt','floor','ceil','trunc'];
export const numericMathUnary=[];
function add(id,body,typed=body){numericMathUnary.push([`math-unary-${id}`,body,typed]);}
function full(id,sourceJS,sourceTS=sourceJS){numericMathUnary.push([`math-unary-${id}`,'','',undefined,undefined,{sourceJS,sourceTS}]);}
const witnesses={sqrt:'4',floor:'-1.5',ceil:'1.5',trunc:'-1.5'};
for(const method of mathUnaryMethods){
 add(`${method}-input`,`return Math.${method}(a);`);
 full(`${method}-extra`,`export function run(a,b) { return Math.${method}(a,b); }`);
 full(`${method}-missing`,`export function run(a,b) { return Math.${method}(); }`);
 add(`${method}-witness`,`return Math.${method}(${witnesses[method]});`);
 add(`${method}-negative-zero`,`return Math.${method}(-0);`);
 add(`${method}-postfix-once`,`return Math.${method}(a++)+a;`);
 add(`${method}-snapshot`,`return Math.${method}(a)+(a=b);`);
 add(`${method}-argument-write`,`return Math.${method}(a=b)+a;`);
 add(`${method}-selected`,`return a ? Math.${method}(a=b) : b;`);
 add(`${method}-finite-loop`,`let x=a; for(let i=0;i<3;i++) { x=Math.${method}(x)+b; } return x;`);
 add(`${method}-wrappers`,`return Math.${method}(a);`,`return (Math.${method}((a as string)!) satisfies number);`);
 full(`${method}-helper`,`function apply(x) { return Math.${method}(x); } export function run(a,b) { return apply(a)+b; }`);
}
add('nested-family','return Math.trunc(Math.ceil(Math.floor(Math.sqrt(Math.abs(a)))));');
add('scope-restoration','{ let Math=a; Math++; a=Math; } return Math.floor(a);');
add('remainder-composition','return Math.trunc(a%b);');
add('predicate','return Math.sqrt(a) ? Math.ceil(b) : Math.floor(b);');
add('skipped-effect','return 1 ? a : Math.sqrt(a=b);');
add('depth-boundary',`return ${'Math.trunc('.repeat(128)}a${')'.repeat(128)};`);
full('type-only-math','export function run(a,b) { return Math.floor(a); }','type Math=number; export function run(a:Math,b:Math) { return Math.floor(a); }');
full('interface-only-math','export function run(a,b) { return Math.ceil(a); }','interface Math {} export function run(a,b) { return Math.ceil(a); }');
export const promotedMathUnaryReference={id:'math-abs-other-method',source:'export function run(a,b) { return Math.floor(a); }',extension:'js',sourceMode:'module-strict',previousGroup:'function-calls'};
numericMathUnary.push([promotedMathUnaryReference.id,'','',undefined,undefined,{sourceJS:promotedMathUnaryReference.source,sourceTS:promotedMathUnaryReference.source}]);
const expression='expression is outside the numeric experiment';
const reference='reference is not an initialized numeric parameter or local';
const binary='only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const bad=[];
function reject(id,body,diagnostic=expression,extension='js'){bad.push([`math-unary-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic]);}
for(const method of mathUnaryMethods){
 reject(`${method}-local-shadow`,`let Math=a; return Math.${method}(b);`);
 bad.push([`math-unary-${method}-parameter-shadow`,`export function run(Math,b) { return Math.${method}(b); }`,'js',expression]);
 reject(`${method}-forward-tdz`,`const x=Math.${method}(a); let Math=b; return x;`,expression);
 reject(`${method}-unselected-unsupported`,`return 1?a:Math.${method}(Math.random());`);
 reject(`${method}-spread`,`return Math.${method}(...a);`,'spread call arguments');
 reject(`${method}-generic`,`return Math.${method}<number>(a);`,expression,'ts');
 reject(`${method}-computed`,`return Math["${method}"](a);`);
 reject(`${method}-optional`,`return Math.${method}?.(a);`);
 reject(`${method}-wrapped-callee`,`return (Math.${method})(a);`);
 reject(`${method}-boolean`,`return Math.${method}(true);`);
 reject(`${method}-predicate`,`return Math.${method}(a<b);`,binary);
 reject(`${method}-string`,`return Math.${method}("3");`);
 reject(`${method}-const-write`,`const x=a; return Math.${method}(x=b);`,'assignment to a const');
 reject(`${method}-dead-patch`,`if(false) Math.${method}=a; return Math.${method}(b);`,'assignment target must be an initialized identifier');
}
reject('undefined','return Math.trunc(undefined);',reference);
reject('negative-object','return Math.sqrt({});');
reject('bigint','return Math.floor(1n);');
reject('global-write','Math=a; return Math.ceil(b);',reference);
bad.push(['math-unary-unused-helper-patch','function unused(x) { Math.trunc=x; return x; } export function run(a,b) { return Math.trunc(a); }','js','assignment target must be an initialized identifier']);
reject('depth-limit',`return ${'Math.trunc('.repeat(129)}a${')'.repeat(129)};`,'expression nesting exceeds');
export const numericMathUnaryRejected=bad;
