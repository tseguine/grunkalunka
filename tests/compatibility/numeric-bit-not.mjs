// Primitive Number bitwise complement first truncates and wraps to signed 32 bits.
// https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-bitwiseNOT
const cases = [
  ['bit-not-promoted-reference', 'return ~a;'],
  ['bit-not-zero', 'return ~0;'],
  ['bit-not-positive-zero-result', 'return ~(-1);'],
  ['bit-not-double', 'return ~~a;'],
  ['bit-not-negative-fraction', 'return ~(-1.9);'],
  ['bit-not-positive-fraction', 'return ~1.9;'],
  ['bit-not-wrap-high', 'return ~4294967295;'],
  ['bit-not-wrap-positive', 'return ~4294967297;'],
  ['bit-not-wrap-negative', 'return ~(-4294967295);'],
  ['bit-not-large-integer', 'return ~9007199254740994;'],
  ['bit-not-nonfinite', 'return ~NaN + ~Infinity + ~(-Infinity);'],
  ['bit-not-once-postfix', 'return ~a++ + a;'],
  ['bit-not-snapshot-before-write', 'return ~a + (a=b);'],
  ['bit-not-predicate', 'return ~a ? b : -0;'],
  ['bit-not-finite-loop', 'let x=a; for(let i=0;i<3;i++) { x=~x; } return x;'],
  ['bit-not-discarded-write', '~(a=b); return a;'],
  ['bit-not-depth-boundary', `return ${'~'.repeat(128)}a;`],
  ['bit-not-statement-boundary', `${'~a;'.repeat(255)} return a;`],
  ['bit-not-local-boundary', `const ${Array.from({length:128},(_,i)=>`x${i}=~a`).join(',')}; return x127;`],
];
export const numericBitNot = cases.map(([id,body]) => [id,body,body]);
numericBitNot.push(['ts-bit-not-wrappers', 'return ~a;',
  'return ~(<number>((a as string)! satisfies number));']);
numericBitNot.push(['bit-not-directive-postfix', 'return ~a++ + a;', 'return ~a++ + a;',
  undefined, undefined, {
    sourceJS: '"module hint"; export function run(a,b) { "use strict"; return ~a++ + a; }',
    sourceTS: '"module hint"; export function run(a:number,b:number):number { "use strict"; return ~a++ + a; }',
  }]);

const expression = 'expression is outside the numeric experiment';
const unary = 'only unary +, - and ~ are admitted';
const binary = 'only binary +, -, * and / are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const negatives = [
  ['bit-not-boolean', 'return ~true;', expression],
  ['bit-not-comparison', 'return ~(a<b);', binary],
  ['bit-not-negation', 'return ~(!a);', unary],
  ['bit-not-string', 'return ~"4";', expression],
  ['bit-not-object', 'return ~({});', expression],
  ['bit-not-bigint', 'return ~1n;', expression],
  ['bit-not-null', 'return ~null;', expression],
  ['bit-not-undefined', 'return ~undefined;', reference],
  ['bit-not-global', 'return ~clock;', reference],
  ['bit-not-call', 'return ~Math.random();', expression],
  ['bit-not-unselected-call', 'return 0?~Math.random():a;', expression],
  ['bit-not-const-write', 'const x=a; return ~(x=b);', 'assignment to a const binding is not admitted'],
  ['bit-not-const-update', 'const x=a; return ~x++;', 'update to a const binding is not admitted'],
  ['bit-not-self-tdz', 'let x=~x; return x;', reference],
  ['bit-not-forward-tdz', 'const x=~y; let y=a; return x;', reference],
  ['bit-not-remainder-remains', 'return ~(a%b);', binary],
  ['bit-not-depth-limit', `return ${'~'.repeat(129)}a;`, 'expression nesting exceeds'],
  ['bit-not-statement-limit', `${'~a;'.repeat(256)} return a;`, 'at most 256 statements'],
  ['bit-not-local-limit', `const ${Array.from({length:129},(_,i)=>`x${i}=~a`).join(',')}; return x128;`, 'at most 128 numeric locals'],
  ['bit-not-asserted-boolean', 'return ~(true as number);', expression, 'ts'],
];
export const numericBitNotRejected = negatives.map(([id,body,diagnostic,extension='js']) =>
  [id,`export function run(a,b) { ${body} }`,extension,diagnostic]);
