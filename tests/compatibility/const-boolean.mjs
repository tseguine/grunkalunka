// Immutable Boolean cells have predicate-position reads, not general Boolean values.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations
// Runtime initializer shape, never a TypeScript annotation, determines the cell kind.
const cases = [
  ['false', 'const flag=false; return flag?a:b;'],
  ['negated-alias', 'const flag=a<b; const other=!flag; return other?a:b;'],
  ['alias-chain', 'const one=a<b, two=one, three=two; return three?1:0;'],
  ['once', 'let x=0; const flag=x++<1; const copy=flag; return copy?x:-1;'],
  ['negation-once', 'let x=0; const flag=!(x++); return flag?x:-1;'],
  ['captured', 'const flag=a>(a=b); return flag?1:0;'],
  ['operand-order', 'let x=0; const flag=(x=3)<(x=4); return flag?x:-x;'],
  ['mixed-declarators', 'const x=a, flag=x<b, copy=flag, y=b; return copy?x:y;'],
  ['number-annotation', 'const flag=a; return flag;', 'const flag:boolean=a; return flag;'],
  ['boolean-annotation', 'const flag=true; return flag?a:b;', 'const flag:number=true; return flag?a:b;'],
  ['as', 'const flag=a<b; const copy=flag; return copy?a:b;', 'const flag=(a<b) as number; const copy=flag as string; return copy?a:b;'],
  ['angle', 'const flag=!a; return flag?a:b;', 'const flag=<number>!a; return flag?a:b;'],
  ['non-null', 'const flag=true; const copy=flag; return copy?a:b;', 'const flag=true!; const copy=flag!; return copy?a:b;'],
  ['satisfies', 'const flag=a<b; return flag?a:b;', 'const flag=(a<b) satisfies number; return flag?a:b;'],
  ['numeric-shadow', 'const flag=true; { const flag=a; if(flag) a=b; } return flag?a:b;'],
  ['boolean-shadow', 'const flag=a; { const flag=false; if(flag) a=b; } return flag;'],
  ['intrinsic-shadow', 'const Infinity=true, NaN=false, Math=Infinity; return Math&&!NaN?a:b;'],
  ['logical-condition', 'const flag=a<b; if(flag&&b) return a; return b;'],
  ['conditional-condition', 'const flag=a<b; if(a?flag:!flag) return a; return b;'],
  ['predicate-comma', 'const flag=a<b; if((a=b,flag)) return 1; return 0;'],
  ['loop-entry', 'let i=0,n=0; while(i<3) { const flag=i++<2; if(flag) n++; } return n;'],
  ['for-header', 'let n=0; for(const flag=true;n<3&&flag;n++) {} return n;'],
  ['do-condition', 'let n=0; const flag=false; do { n++; } while(flag); return n;'],
  ['unused', 'const flag=true; return a;'],
  ['not-conditional', 'const flag=!(a?true:false); return flag?a:b;'],
  ['not-logical', 'const flag=!(a&&b); return flag?a:b;'],
  ['depth-boundary', `const flag=${'('.repeat(128)}true${')'.repeat(128)}; return flag?a:b;`],
  ['not-depth-boundary', `const flag=${'!'.repeat(128)}a; return flag?a:b;`],
  ['ts-depth-boundary', 'const flag=true; return flag?a:b;', `const flag=true${'!'.repeat(128)}; return flag?a:b;`],
  ['mixed-local-boundary', `const ${Array.from({length:128},(_,i)=>`x${i}=${i%2?'true':'a'}`).join(',')}; return x127?x126:b;`],
];
for (const operator of ['<','<=','>','>=','==','!=','===','!==']) {
  cases.push([`comparison-${['<','<=','>','>=','==','!=','===','!=='].indexOf(operator)}`,
    `const flag=a${operator}b; return flag?1:0;`]);
}
export const constBooleanPrograms = cases.map(([id,js,ts=js])=>[`const-boolean-${id}`,js,ts]);
constBooleanPrograms.push(['const-boolean-helper','','',undefined,undefined,{
  sourceJS:'function choose(x,y) { const flag=x<y; return flag?x:y; } export function run(a,b) { return choose(a,b); }',
  sourceTS:'function choose(x:number,y:number):number { const flag:boolean=x<y; return flag?x:y; } export function run(a:number,b:number):number { return choose(a,b); }',
}]);
constBooleanPrograms.push(['const-boolean-module-local-boundary','','',undefined,undefined,{
  sourceJS:`function f(x) { const ${Array.from({length:64},(_,i)=>`h${i}=true`).join(',')}; return h63?x:0; } export function run(a,b) { const ${Array.from({length:64},(_,i)=>`r${i}=a`).join(',')}; return f(r63); }`,
  sourceTS:`function f(x:number):number { const ${Array.from({length:64},(_,i)=>`h${i}=true`).join(',')}; return h63?x:0; } export function run(a:number,b:number):number { const ${Array.from({length:64},(_,i)=>`r${i}=a`).join(',')}; return f(r63); }`,
}]);

// Original sources/IDs survive promotion. JS oracle text is written independently.
for (const [id,sourceJS,sourceTS] of [
  ['gap-boolean-value','export function run(a,b) { const less = a < b; return less ? 1 : 0; }','export function run(a:number,b:number):number { const less = a < b; return less ? 1 : 0; }'],
  ['boolean-literal-local','export function run(a,b) { const x=true; return x?a:b; }','export function run(a:number,b:number):number { const x=true; return x?a:b; }'],
  ['not-local','export function run(a,b) { const x=!a; return x?a:b; }','export function run(a:number,b:number):number { const x=!a; return x?a:b; }'],
  ['equality-numeric-local','export function run(a,b) { const x=a==b; return a; }','export function run(a:number,b:number):number { const x=a==b; return a; }'],
]) constBooleanPrograms.push([id,'','',undefined,undefined,{sourceJS,sourceTS}]);
// Preserve the former curated reference's independent observation contract.
export const promotedBooleanGap = {
  id:'gap-boolean-value', category:'values', milestone:'L4', prerequisites:[],
  oracleSource:'export function run(a,b) { const less = a < b; return less ? 1 : 0; }',
  args:[4,2], entry:'run', expected:0,
};

const numeric='Boolean bindings are not admitted in numeric value positions';
const reference='reference is not an initialized numeric parameter or local';
const expression='expression is outside the numeric experiment';
const negatives=[
  ['return','const flag=true; return flag;',numeric],
  ['assignment-rhs','const flag=true; a=flag; return a;','General bindings require a primitive value position'],
  ['const-write','const flag=true; flag=a; return a;','assignment to a const binding'],
  ['const-update','const flag=true; flag++; return a;','update to a const binding'],
  ['self','const flag=flag; return a;',reference],
  ['forward','const flag=next; const next=true; return a;',reference],
  ['shadow-self','const flag=true; { const flag=flag; } return a;',reference],
  ['shadow-forward','const flag=true; { const copy=flag; const flag=false; } return a;',reference],
  ['intrinsic-self','const Infinity=Infinity; return a;',reference],
  ['scope-leak','{ const flag=true; } return flag?a:b;',reference],
  ['dead-unsupported','if(false) { const flag=!Math.random(); } return a;',expression],
  ['unselected-unsupported','const flag=!(true?true:Math.random()); return a;',expression],
  ['asserted-string','const flag=("x" as boolean); return a;',expression,'ts'],
  ['depth-limit',`const flag=${'('.repeat(129)}true${')'.repeat(129)}; return a;`,'expression nesting exceeds'],
  ['not-depth-limit',`const flag=${'!'.repeat(129)}a; return a;`,'predicate nesting exceeds'],
  ['ts-depth-limit',`const flag=true${'!'.repeat(129)}; return a;`,'expression nesting exceeds','ts'],
  ['alias-depth-limit',`const flag=true; const copy=${'('.repeat(129)}flag${')'.repeat(129)}; return a;`,'expression nesting exceeds'],
  ['mixed-local-limit',`const ${Array.from({length:129},(_,i)=>`x${i}=${i%2?'true':'a'}`).join(',')}; return a;`,'at most 128 source locals'],
  ['statement-limit',`const x0=true;${'if(false) ;'.repeat(128)}return a;`,'at most 256 statements'],
];
export const constBooleanRejected=negatives.map(([id,body,diagnostic,ext='js'])=>[
  `const-boolean-${id}`,`export function run(a,b) { ${body} }`,ext,diagnostic,
]);
constBooleanRejected.push(['const-boolean-unused-helper',
  'function f(x) { const flag=true; return flag; } export function run(a,b) { return a; }','js',numeric]);
constBooleanRejected.push(['const-boolean-module-local-limit',
  `function f(x) { const ${Array.from({length:65},(_,i)=>`h${i}=true`).join(',')}; return x; } export function run(a,b) { const ${Array.from({length:64},(_,i)=>`r${i}=a`).join(',')}; return a; }`,
  'js','at most 128 source locals']);
