// Primitive Number bitwise operations eagerly capture both operands in source order.
// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numberbitwiseop
const cases = [
  ['bitwise-and-promoted-reference', 'return a & b;'],
  ['bitwise-or-promoted-reference', 'return a | b;'],
  ['bitwise-xor-promoted-reference', 'return a ^ b;'],
  ['bitwise-or-promoted-rejection', 'return a | b;'],
  ['bitwise-and-fixed', 'return 5 & 3;'],
  ['bitwise-or-fixed', 'return 5 | 3;'],
  ['bitwise-xor-fixed', 'return 5 ^ 3;'],
  ['bitwise-and-left-snapshot', 'return a & (a=b);'],
  ['bitwise-or-left-snapshot', 'return a | (a=b);'],
  ['bitwise-xor-left-snapshot', 'return a ^ (a=b);'],
  ['bitwise-and-zero-eager-rhs', 'let x=0; const y=0 & (x=5); return x+y;'],
  ['bitwise-or-nonzero-eager-rhs', 'let x=0; const y=1 | (x=2); return x*10+y;'],
  ['bitwise-xor-operand-order', 'let x=0; const y=(x=5) ^ (x=3); return x*10+y;'],
  ['bitwise-fractions', 'return (-5.9 & 3.9) + (5.9 | -3.9) + (-5.9 ^ -3.9);'],
  ['bitwise-wrap', 'return (4294967297 & 3) + (4294967295 | 0) + (-4294967295 ^ 3);'],
  ['bitwise-signed-result', 'return (2147483648 | 0) + (2147483648 & -1) + (2147483648 ^ 1);'],
  ['bitwise-nonfinite', 'return (NaN & -1) + (Infinity | 3) + (-Infinity ^ 5);'],
  ['bitwise-nested-complement', 'return ~(a & b) ^ (a | ~b);'],
  ['bitwise-predicate', 'return (a & b) ? -0 : b;'],
  ['bitwise-while-test', 'let i=3; let x=a; while(i & 3) { x=x ^ b; i--; } return x;'],
  ['bitwise-for-body', 'let x=a; for(let i=0;i<3;i++) { x=(x | i) ^ b; } return x;'],
  ['bitwise-do-test', 'let i=0; let x=a; do { x=x | b; i++; } while(i<3 && (1 | 0)); return x;'],
  ['bitwise-discarded-effects', 'a & (b=a); return b;'],
  ['bitwise-depth-boundary', `return a${' & 1'.repeat(128)};`],
  ['bitwise-statement-boundary', `${'a & b;'.repeat(255)} return a;`],
  ['bitwise-local-boundary', `const ${Array.from({length:128},(_,i)=>`x${i}=a & b`).join(',')}; return x127;`],
];
export const numericBitwiseBinary = cases.map(([id,body]) => [id,body,body]);
numericBitwiseBinary.push(
  ['bitwise-ts-and', 'return a & b;', 'return ((a as string) & <number>b)!;'],
  ['bitwise-ts-or', 'return a | b;', 'return (a! | (b satisfies number)) as number;'],
  ['bitwise-ts-xor', 'return a ^ b;', 'return (<number>a ^ (b as number)) satisfies number;'],
  ['bitwise-hashbang-directive-snapshot', 'return a & (a=b);', 'return a & (a=b);',
    undefined, undefined, {
      sourceJS: '#!/usr/bin/env node\n"module hint"; export function run(a,b) { "use strict"; return a & (a=b); }',
      sourceTS: '#!/usr/bin/env node\n"module hint"; export function run(a:number,b:number):number { "use strict"; return a & (a=b); }',
    }],
);

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, &, | and ^ are admitted';
const unary = 'only unary +, - and ~ are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const negatives = [
  ...['&','|','^'].flatMap((op,i)=>[
    [`bitwise-boolean-left-${i}`, `return true ${op} a;`, expression],
    [`bitwise-boolean-right-${i}`, `return a ${op} false;`, expression],
  ]),
  ['bitwise-string-left', 'return "5" & a;', expression],
  ['bitwise-string-right', 'return a | "3";', expression],
  ['bitwise-object', 'return a ^ ({});', expression],
  ['bitwise-bigint', 'return 1n & 3n;', expression],
  ['bitwise-null', 'return a | null;', expression],
  ['bitwise-undefined', 'return a ^ undefined;', reference],
  ['bitwise-global', 'return clock & a;', reference],
  ['bitwise-call', 'return a | Math.random();', expression],
  ['bitwise-unselected-call', 'return 0 ? (a & Math.random()) : a;', expression],
  ['bitwise-unselected-string', 'return 1 || (a ^ "4");', expression],
  ['bitwise-unselected-predicate', 'return 1 ? a : (a | (a<b));', binary],
  ['bitwise-comparison', 'return (a<b) & b;', binary],
  ['bitwise-negation', 'return !a | b;', unary],
  ['bitwise-asserted-boolean', 'return (true as number) ^ a;', expression, 'ts'],
  ['bitwise-const-write', 'const x=a; return x & (x=b);', 'assignment to a const binding is not admitted'],
  ['bitwise-const-update', 'const x=a; return x++ | b;', 'update to a const binding is not admitted'],
  ['bitwise-self-tdz', 'let x=x ^ a; return x;', reference],
  ['bitwise-forward-tdz', 'const x=y & a; let y=b; return x;', reference],
  ['bitwise-shift-remains', 'return (a<<b) | 1;', binary],
  ['bitwise-compound-remains', 'return a & (b|=1);', 'only =, +=, -=, *= and /= assignments are admitted'],
  ['bitwise-remainder-remains', 'return (a%b) ^ 1;', binary],
  ['bitwise-depth-limit', `return a${' & 1'.repeat(129)};`, 'expression nesting exceeds'],
  ['bitwise-statement-limit', `${'a & b;'.repeat(256)} return a;`, 'at most 256 statements'],
  ['bitwise-local-limit', `const ${Array.from({length:129},(_,i)=>`x${i}=a & b`).join(',')}; return x128;`, 'at most 128 numeric locals'],
];
export const numericBitwiseBinaryRejected = negatives.map(([id,body,diagnostic,extension='js'])=>
  [id,`export function run(a,b) { ${body} }`,extension,diagnostic]);
