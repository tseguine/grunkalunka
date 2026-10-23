// Compound assignment captures the target before RHS effects, stores and yields the result.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
const cases = [
  ['compound-bit-and-promoted', 'return a &= b;'],
  ['compound-bit-or-promoted', 'return a |= b;'],
  ['compound-bit-xor-promoted', 'return a ^= b;'],
  ['compound-shift-left-promoted', 'return a <<= b;'],
  ['compound-shift-right-promoted', 'return a >>= b;'],
  ['compound-shift-unsigned-promoted', 'return a >>>= b;'],
  ['compound-bitwise-composition-promoted', 'return a & (b|=1);'],
  ['compound-shift-left-composition-promoted', 'return a << (b<<=1);'],
  ['compound-shift-right-composition-promoted', 'return a >> (b>>=1);'],
  ['compound-shift-unsigned-composition-promoted', 'return a >>> (b>>>=1);'],
  ...[['and','&='],['or','|='],['xor','^='],['left','<<='],['right','>>='],['unsigned','>>>=']]
    .map(([name,op])=>[`compound-${name}-target-snapshot`, `return a ${op} (a=b);`]),
  ['compound-right-fixed-snapshot', 'let x=-8; return x >>= (x=1);'],
  ['compound-unsigned-zero-count', 'let x=-1; return x >>>= 0;'],
  ['compound-unsigned-wrapped-count', 'let x=-1; return x >>>= 32;'],
  ...[['and','&=',5,3],['or','|=',5,3],['xor','^=',5,3],['left','<<=',5,3],
    ['right','>>=',-8,1],['unsigned','>>>=',-8,0]]
    .map(([name,op,left,right])=>[`compound-${name}-store-yield`, `let x=${left}; const y=x ${op} ${right}; return x*10+y;`]),
  ['compound-yield-before-later-write', 'let x=5; return (x&=3)-(x=9);'],
  ['bitwise-compound-rhs-once', 'let n=0; let x=5; const y=x|=(n++,3); return n*100+x*10+y;'],
  ['compound-zero-left-eager-rhs', 'let n=0; let x=0; const y=x&=(n++,3); return n*10+x+y;'],
  ['compound-nested', 'let x=a; let y=b; return x^=(y|=3);'],
  ['compound-predicate', 'let x=a; return (x&=b) ? -0 : b;'],
  ['compound-finite-for', 'let x=a; for(let i=0;i<3;i++) { x^=b; } return x;'],
  ['compound-finite-while-header', 'let i=8; let x=a; while(i>>=1) { x|=b; } return x;'],
  ['compound-finite-do', 'let i=0; let x=a; do { x<<=1; i++; } while(i<3); return x;'],
  ['compound-statement-boundary', `${'a&=b;'.repeat(255)} return a;`],
  ['compound-local-boundary', `const ${Array.from({length:128},(_,i)=>`x${i}=(a&=b)`).join(',')}; return x127;`],
  ['compound-depth-boundary', `return ${'a&='.repeat(128)}b;`],
];
export const numericBitwiseCompounds = cases.map(([id,body])=>[id,body,body]);
numericBitwiseCompounds.push(
  ['compound-ts-whole-wrapper', 'return a |= b;', 'return ((a|=b) as number)!;'],
  ['compound-ts-rhs-wrapper', 'return a <<= b;', 'return (a<<=(b as unknown as string)) satisfies number;'],
  ['compound-ts-local-wrapper', 'let x=a; return x >>>= b;', 'let x:number=a; return <number>(x>>>=b);'],
);

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const assignment = 'only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>= and >>>= assignments are admitted';
const target = 'assignment target must be an initialized identifier';
const negatives = [
  ...['&=','|=','^=','<<=','>>=','>>>='].map((op,i)=>
    [`compound-immutable-${i}`, `const x=a; return x ${op} b;`, 'assignment to a const binding is not admitted']),
  ['compound-self-tdz', 'let x=(x&=a); return x;', reference],
  ['compound-forward-tdz', 'const x=(y|=a); let y=b; return x;', reference],
  ['compound-unselected-tdz', 'const x=1?a:(y^=a); let y=b; return x;', reference],
  ['compound-scope-leak', '{ let x=a; } return x<<=b;', reference],
  ['compound-global-target', 'return clock>>=a;', reference],
  ['compound-infinity-target', 'return Infinity>>>=a;', reference],
  ['compound-nan-target', 'return NaN&=a;', reference],
  ['compound-property-target', 'return a.x|=b;', target],
  ['compound-destructuring-target', 'return [a]^=b;', 'parse diagnostics'],
  ['compound-ts-wrapped-target', 'return (a as number)<<=b;', target, 'ts'],
  ['compound-ts-nonnull-target', 'return a!>>=b;', target, 'ts'],
  ['compound-boolean-rhs', 'return a&=true;', expression],
  ['compound-string-rhs', 'return a|="3";', expression],
  ['compound-object-rhs', 'return a^=({});', expression],
  ['compound-bigint-rhs', 'return a<<=1n;', expression],
  ['compound-null-rhs', 'return a>>=null;', expression],
  ['compound-call-rhs', 'return a>>>=Math.random();', expression],
  ['compound-unselected-call', 'return 1?a:(a&=Math.random());', expression],
  ['compound-predicate-rhs', 'return a|=(a<b);', binary],
  ['compound-depth-limit', `return ${'a&='.repeat(129)}b;`, 'expression nesting exceeds'],
  ['compound-statement-limit', `${'a&=b;'.repeat(256)} return a;`, 'at most 256 statements'],
  ['compound-local-limit', `const ${Array.from({length:129},(_,i)=>`x${i}=(a&=b)`).join(',')}; return x128;`, 'at most 128 numeric locals'],
];
export const numericBitwiseCompoundsRejected = negatives.map(([id,body,diagnostic,extension='js'])=>
  [id,`export function run(a,b) { ${body} }`,extension,diagnostic]);
