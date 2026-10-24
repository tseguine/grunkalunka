// Same-kind strict Boolean equality; no mixed-kind or loose conversion policy.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-isstrictlyequal
export const strictBooleanEquality=[];
const add=(id,js,ts=js,expected)=>strictBooleanEquality.push([`strict-boolean-equality-${id}`,js,ts,expected]);
for(const [op,name] of [['===','equal'],['!==','not-equal']]) {
 for(const left of [false,true]) for(const right of [false,true])
  add(`${name}-${left}-${right}`,`return ${left}${op}${right}?1:0;`,undefined,op==='==='?Number(left===right):Number(left!==right));
 add(`${name}-binding-left`,`const flag=a<b; return flag${op}true?a:b;`);
 add(`${name}-binding-right`,`const flag=a<b; return false${op}flag?a:b;`);
 add(`${name}-aliases`,`const flag=a<b,copy=flag; return flag${op}copy?1:0;`);
 add(`${name}-comparisons`,`return (a<b)${op}(a>=b)?1:0;`);
 add(`${name}-negation`,`return (!a)${op}(!b)?1:0;`);
 add(`${name}-nested`,`return ((a<b)===true)${op}((a>b)!==false)?1:0;`);
 for(const [kind,wrapped] of [['as','(flag as number)'],['angle','(<string>flag)'],['nonnull','flag!'],['satisfies','(flag satisfies string)']])
  add(`${name}-${kind}`,`const flag=true; return flag${op}false?1:0;`,`const flag:number=true; return ${wrapped} ${op}false?1:0;`,op==='==='?0:1);
 add(`${name}-depth128`,`return true${op}false?a:b;`,`return true${'!'.repeat(126)} ${op}false?a:b;`);
}
add('left-snapshot','let x=0; return ((!x)===(!(x=1))?10:20)+x*100;',undefined,120);
add('effect-order','let x=0; const flag=((x=1)<2)===((x=2)<2); return x*100+(flag?1:0);',undefined,200);
add('both-once','let x=0,y=0; const flag=(x++===0)===(y++===0); return x*100+y*10+(flag?1:0);',undefined,111);
add('eager-right','let x=0; return (false===!(x=1)?10:20)+x*100;',undefined,110);
add('numeric-shadow','const flag=true; { const flag=a; if(flag===b) return 1; } return flag===false?2:3;');
add('predicate-shadow','const flag=a; { const flag=true; if(flag!==false) return 1; } return flag;');
add('numeric-annotation','const flag=a; return flag===b?1:0;','const flag:boolean=a; return flag===b?1:0;');
add('predicate-annotation','const flag=true; return flag===false?1:0;','const flag:string=true; return flag===false?1:0;',0);
add('intrinsic-shadow','const Infinity=true,NaN=false,Math=Infinity; return Math!==NaN?1:0;',undefined,1);
add('unary-composition','const flag=a<b; return +(flag===true);');
add('arithmetic-composition','const flag=a<b; return (flag!==false)+b;');
add('discarded-comparison','let x=0; (!x++)===true; return x;',undefined,1);
add('loop','let n=0; for(let i=0;i<3;i++) { const flag=i<2; if(flag===true) n++; } return n;',undefined,2);
add('js-depth128',`return ${'('.repeat(126)}true===false${')'.repeat(126)}?a:b;`);
strictBooleanEquality.push(['strict-boolean-equality-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y) { const flag=x<y; return flag===true?x:y; } export function run(a,b) { return f(a,b); }',
 sourceTS:'function f(x:number,y:number):number { const flag:boolean=x<y; return flag===true?x:y; } export function run(a:number,b:number):number { return f(a,b); }',
}]);
export const strictBooleanEqualityPromotions=[
 ['boolean-equality-operands','export function run(a,b) { if(true===false) return a; return b; }','export function run(a:number,b:number):number { if(true===false) return a; return b; }','a76683e2472b7ba44bce8292a1061a6a8a1bccefef39072661fe3b7fb696546e'],
 ['not-equality-operand','export function run(a,b) { if(!a===!b) return a; return b; }','export function run(a:number,b:number):number { if(!a===!b) return a; return b; }','1dbab9a6fff5c3f740b1113ae2f375c4ec9954bb52d5f8c3df03932e759c2a8b'],
];
for(const[id,sourceJS,sourceTS]of strictBooleanEqualityPromotions) strictBooleanEquality.push([id,'','',undefined,undefined,{sourceJS,sourceTS}]);
const expression='expression is outside the numeric experiment',numeric='Boolean bindings are not admitted in numeric value positions',binary='only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted',reference='reference is not an initialized numeric parameter or local';
const negatives=[
 ['loose-equal','return true==false?1:0;',expression,'coercive-equality'],
 ['loose-not-equal','return true!=false?1:0;',expression,'coercive-equality'],
 ['relational','return true<false?1:0;',expression,'boolean-values'],
 ['direct-return','return true===false;',binary,'boolean-values'],
 ['self','const flag=flag===true; return a;',reference,'binding-errors'],
 ['forward','const first=flag===true; const flag=true; return a;',reference,'binding-errors'],
 ['tdz','return flag===true?1:0; const flag=true;',reference,'binding-errors'],
 ['shadow-tdz','const flag=true; { const copy=flag===true; const flag=false; } return a;',reference,'binding-errors'],
 ['scope','{const flag=true;} return flag===true?1:0;',reference,'binding-errors'],
 ['unselected-call','return false?(true===!Math.random()?1:0):a;',expression,'function-calls'],
 ['const-write','const flag=true; return (!flag++)===false?1:0;','update to a const binding is not admitted','const-errors'],
 ['asserted-string','return ("x" as boolean)===true?1:0;',expression,'string-values','ts'],
 ['js-depth129',`return ${'('.repeat(127)}true===false${')'.repeat(127)}?a:b;`,'expression nesting exceeds','expression-depth'],
 ['ts-depth129',`return true${'!'.repeat(127)} ===false?a:b;`,'expression nesting exceeds','expression-depth','ts'],
];
export const strictBooleanEqualityRejected=negatives.map(([id,body,diagnostic,,extension='js'])=>[`strict-boolean-equality-reject-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic]);
export const strictBooleanEqualityRejectionGroups=new Map(negatives.map(([id,,,group])=>[`strict-boolean-equality-reject-${id}`,group]));
