// Number-only pristine intrinsic profile; no external realm or coercive operands.
// https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.abs
const cases = [
 ['zero','return Math.abs(0);'], ['negative-zero','return Math.abs(-0);'],
 ['positive','return Math.abs(3);'], ['negative','return Math.abs(-3);'],
 ['input','return Math.abs(a);'], ['nonfinite','return Math.abs(-Infinity)+Math.abs(NaN);'],
 ['subnormal','return Math.abs(-5e-324);'], ['maximum','return Math.abs(-1.7976931348623157e308);'],
 ['nested','return Math.abs(-Math.abs(a));'],
 ['postfix-once','return Math.abs(a++)+a;'],
 ['snapshot','return Math.abs(a)+(a=b);'],
 ['argument-write','return Math.abs(a=b)+a;'],
 ['discarded','Math.abs(a=b); return a;'],
 ['selected','return a ? Math.abs(a=b) : b;'],
 ['skipped','return 1 ? a : Math.abs(a=b);'],
 ['logical','return a || Math.abs(a=b);'],
 ['predicate','return Math.abs(a) ? b : -0;'],
 ['loop','let x=a; for(let i=0;i<3;i++) { x=Math.abs(x)+b; } return x;'],
 ['local-math-mutation','let Math=a; Math++; return Math+b;'],
 ['scope-restoration','{ let Math=a; Math+=b; a=Math; } return Math.abs(a);'],
 ['depth-boundary',`return ${'Math.abs('.repeat(128)}a${')'.repeat(128)};`],
 ['statement-boundary',`${'Math.abs(a);'.repeat(255)} return a;`],
];
export const numericMathAbs=cases.map(([id,body])=>[`math-abs-${id}`,body,body]);
function full(id,sourceJS,sourceTS=sourceJS){numericMathAbs.push([`math-abs-${id}`,'','',undefined,undefined,{sourceJS,sourceTS}]);}
full('extra','export function run(a,b) { return Math.abs(a,b); }');
full('missing','export function run(a,b) { return Math.abs(); }');
numericMathAbs.push(['math-abs-missing-shadowed-undefined','const undefined=a;return Math.abs();','const undefined=a;return Math.abs();']);
numericMathAbs.push(['math-abs-extra-effects','let x=a;return Math.abs(x,x=b)+x;','let x=a;return Math.abs(x,x=b)+x;']);
const promoted='export function run(a,b) { return Math.abs(a); }';
// Preserve the promoted curated requirement's original source/oracle and provenance.
export const promotedMathAbsReference={id:'gap-math-call',source:promoted,extension:'js',sourceMode:'module-strict',category:'globals-builtins',milestone:'L4',prerequisites:['property access','calls'],oracleSource:promoted,args:[-4,2],entry:'run',expected:4};
for(const id of ['call','gap-math-call']) numericMathAbs.push([id,'','',undefined,undefined,{sourceJS:promoted,sourceTS:promoted}]);
full('helper','function magnitude(x) { return Math.abs(x); } export function run(a,b) { return magnitude(a)+b; }');
full('parameter-math-mutation','export function run(Math,b) { Math++; return Math+b; }');
full('type-only-math','export function run(a,b) { return Math.abs(a); }','type Math=number; export function run(a:Math,b:Math) { return Math.abs(a); }');
full('interface-only-math','export function run(a,b) { return Math.abs(a); }','interface Math {} export function run(a,b) { return Math.abs(a); }');
full('wrappers','export function run(a,b) { return Math.abs(a); }','export function run(a,b) { return (Math.abs((a as string)!) satisfies number); }');
const expression='expression is outside the numeric experiment';
const reference='reference is not an initialized numeric parameter or local';
const assignment='assignment target must be an initialized identifier';
const bad=[];
function reject(id,body,diagnostic=expression,extension='js'){bad.push([`math-abs-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic]);}
for(const [id,body] of [
 ['local-shadow','let Math=a; return Math.abs(b);'],
 ['self-tdz','let Math=Math.abs(a); return Math;'],
 ['forward-tdz','const x=Math.abs(a); let Math=b; return x;'],
 ['unselected-shadow','let Math=a; return 1?a:Math.abs(b);'],
 ['computed','return Math["abs"](a);'], ['optional-member','return Math?.abs(a);'],
 ['optional-call','return Math.abs?.(a);'], ['wrapped-callee','return (Math.abs)(a);'],
 ['constructor','return new Math.abs(a);'], ['first-class','return Math.abs;'],
 ['alias','const f=Math.abs; return f(a);'],
 ['boolean','return Math.abs(true);'], ['string','return Math.abs("3");'],
 ['object','return Math.abs({});'], ['bigint','return Math.abs(1n);'],
 ['null','return Math.abs(null);'], ['dead-unsupported','return 1?a:Math.abs(Math.random());'],
 ]) reject(id,body);
reject('spread','return Math.abs(...a);','spread call arguments');
reject('generic','return Math.abs<number>(a);',expression,'ts');
reject('undefined','return Math.abs(undefined);',reference);
reject('global-write','Math=a; return Math.abs(b);',reference);
reject('global-update','Math++; return Math.abs(a);',reference);
reject('property-write','Math.abs=a; return Math.abs(b);',assignment);
reject('dead-property-write','if(false) Math.abs=a; return Math.abs(b);',assignment);
reject('dead-global-write','if(false) Math=a; return Math.abs(b);',reference);
reject('delete','delete Math.abs; return Math.abs(a);','only unary');
reject('const-write','const x=a; return Math.abs(x=b);','assignment to a const');
reject('depth-limit',`return ${'Math.abs('.repeat(129)}a${')'.repeat(129)};`,'expression nesting exceeds');
reject('statement-limit',`${'Math.abs(a);'.repeat(256)} return a;`,'at most 256 statements');
for(const [id,source] of [
 ['parameter-shadow','export function run(Math,b) { return Math.abs(b); }'],
 ['module-helper-shadow','function Math(x) { return x; } export function run(a,b) { return Math.abs(a); }'],
 ['unused-helper-patch','function unused(x) { Math.abs=x; return x; } export function run(a,b) { return Math.abs(a); }'],
 ]) bad.push([`math-abs-${id}`,source,'js',id==='unused-helper-patch'?assignment:expression]);
export const numericMathAbsRejected=bad;
