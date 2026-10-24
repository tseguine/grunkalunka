// Number shifts use the low five count bits and retain eager operand snapshots.
// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-leftShift
const cases = [
  ['shift-left-promoted-reference', 'return a << b;'],
  ['shift-right-promoted-reference', 'return a >> b;'],
  ['shift-unsigned-promoted-reference', 'return a >>> b;'],
  ['shift-bitwise-promoted-rejection', 'return (a<<b) | 1;'],
  ['shift-unsigned-fixed', 'return -1 >>> 0;'],
  ['shift-signed-fixed', 'return -1 >> 0;'],
  ['shift-left-fixed', 'return 1 << 31;'],
  ['shift-count-31', 'return (1<<31) + (-1>>>31) + (-1>>31);'],
  ['shift-count-32', 'return (1<<32) + (-1>>>32) + (-1>>32);'],
  ['shift-count-33', 'return (1<<33) + (-1>>>33) + (-1>>33);'],
  ['shift-count-negative', 'return (1<<-1) + (-1>>>-1) + (-1>>-1);'],
  ['shift-fractions', 'return (-5.9<<1.9) + (-5.9>>1.9) + (-5.9>>>1.9);'],
  ['shift-count-wrap', 'return (a<<4294967297) + (a>>-4294967295) + (a>>>9007199254740994);'],
  ['shift-nonfinite-count', 'return (a<<NaN) + (a>>Infinity) + (a>>>-Infinity);'],
  ['shift-nonfinite-left', 'return (NaN<<b) + (Infinity>>b) + (-Infinity>>>b);'],
  ['shift-wrapped-sign-bit', 'return (4294967295<<1) + (4294967295>>1) + (4294967295>>>1);'],
  ['shift-left-snapshot', 'return a << (a=b);'],
  ['shift-right-snapshot', 'return a >> (a=b);'],
  ['shift-unsigned-snapshot', 'return a >>> (a=b);'],
  ['shift-left-zero-eager-rhs', 'let x=0; const y=0 << (x=3); return x*10+y;'],
  ['shift-right-zero-eager-rhs', 'let x=0; const y=0 >> (x=3); return x*10+y;'],
  ['shift-unsigned-zero-eager-rhs', 'let x=0; const y=0 >>> (x=3); return x*10+y;'],
  ['shift-operand-order', 'let x=0; const y=(x=5) >> (x=1); return x*10+y;'],
  ['shift-nested-bitwise', 'return ((~a<<1) ^ (b>>>1)) | (a>>3);'],
  ['shift-predicate', 'return (a>>>b) ? -0 : b;'],
  ['shift-finite-while', 'let i=0; let x=a; while(i<3) { x=x>>>1; i++; } return x;'],
  ['shift-for-test', 'let x=a; for(let i=0;(i<<1)<6;i++) { x=x<<1; } return x;'],
  ['shift-finite-do', 'let i=0; let x=a; do { x=x>>1; i++; } while(i<3); return x;'],
  ['shift-depth-boundary', `return a${' >>> 0'.repeat(128)};`],
  ['shift-statement-boundary', `${'a >>> b;'.repeat(255)} return a;`],
  ['shift-local-boundary', `const ${Array.from({length:128},(_,i)=>`x${i}=a << b`).join(',')}; return x127;`],
];
export const numericShifts = cases.map(([id,body])=>[id,body,body]);
numericShifts.push(
  ['shift-ts-left', 'return a << b;', 'return <number>a << (b as unknown as string);'],
  ['shift-ts-right', 'return a >> b;', 'return (a! >> (b satisfies number)) as number;'],
  ['shift-ts-unsigned', 'return a >>> b;', 'return ((a as number) >>> <number>b)!;'],
);

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const unary = 'only unary +, - and ~ are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const negatives = [
  ...['<<','>>','>>>'].flatMap((op,i)=>[
    [`shift-boolean-left-${i}`, `return true ${op} a;`, expression],
    [`shift-boolean-right-${i}`, `return a ${op} false;`, expression],
  ]),
  ['shift-string-left', 'return "5" << a;', expression],
  ['shift-string-right', 'return a >> "3";', expression],
  ['shift-object', 'return a >>> ({});', expression],
  ['shift-bigint', 'return 1n << 3n;', expression],
  ['shift-null', 'return a >> null;', expression],
  ['shift-undefined', 'return a >>> undefined;', reference],
  ['shift-global', 'return clock << a;', reference],
  ['shift-call', 'return a >> Math.random();', expression],
  ['shift-unselected-call', 'return 0 ? (a << Math.random()) : a;', expression],
  ['shift-unselected-string', 'return 1 || (a >>> "4");', expression],
  ['shift-unselected-predicate', 'return 1 ? a : (a >> (a<b));', binary],
  ['shift-comparison', 'return (a<b) << b;', binary],
  ['shift-negation', 'return !a >> b;', unary],
  ['shift-asserted-boolean', 'return (true as number) >>> a;', expression, 'ts'],
  ['shift-const-write', 'const x=a; return x << (x=b);', 'assignment to a const binding is not admitted'],
  ['shift-const-update', 'const x=a; return x++ >> b;', 'update to a const binding is not admitted'],
  ['shift-self-tdz', 'let x=x >>> a; return x;', reference],
  ['shift-forward-tdz', 'const x=y << a; let y=b; return x;', reference],
  ['shift-depth-limit', `return a${' >>> 0'.repeat(129)};`, 'expression nesting exceeds'],
  ['shift-statement-limit', `${'a >>> b;'.repeat(256)} return a;`, 'at most 256 statements'],
  ['shift-local-limit', `const ${Array.from({length:129},(_,i)=>`x${i}=a << b`).join(',')}; return x128;`, 'at most 128 source locals'],
];
export const numericShiftsRejected = negatives.map(([id,body,diagnostic,extension='js'])=>
  [id,`export function run(a,b) { ${body} }`,extension,diagnostic]);
