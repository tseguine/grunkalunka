// Discarded Number results still evaluate in source order, preserving effects.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-expression-statement-runtime-semantics-evaluation
const cases = [
  ['statement-primitive-expressions', '1; a; a+b; return a;'],
  ['statement-promoted-arithmetic', 'a+(a=b); return a;'],
  ['statement-promoted-parentheses', '((a+(a=b))); return a;'],
  ['statement-promoted-comma', '(a=b,b=a+1); return a+b;'],
  ['statement-comma-updates', '(a++,b+=a,a+b); return a+b;'],
  ['statement-logical-and', 'a&&(b=a); return b;'],
  ['statement-logical-or', 'a||(b=a); return b;'],
  ['statement-nested-logical', 'let x=a; (a&&(x=b)) || (x=a); return x;'],
  ['statement-conditional', 'a?(b=a):(a=b); return a+b;'],
  ['statement-block', 'let x=a; { x+(x=b); } return x;'],
  ['statement-for-body', 'let x=a; for(let i=0;i<3;i++) { (x+=b,i); } return x;'],
  ['statement-do-body', 'let i=0; do { a+b; (a+=b,i++); } while(i<3); return a;'],
  ['statement-expression-depth-budget', `${'('.repeat(128)}a${')'.repeat(128)}; return a;`],
  ['statement-count-budget', `${'a;'.repeat(255)} return a;`],
  ['statement-special-values', 'a/b; 0/0; -0; return a;'],
];
export const numericStatementPrograms = cases.map(([id, body]) => [id, body, body]);
numericStatementPrograms.push(
  ['ts-erasure-as-statement', 'a=b; return a;', '(a = b) as number; return a;'],
  ['ts-erasure-angle-statement', 'a=b; return a;', '<number>(a = b); return a;'],
  ['ts-erasure-non-null-statement', 'a=b; return a;', '(a = b)!; return a;'],
  ['ts-erasure-satisfies-statement', 'a=b; return a;', '(a = b) satisfies number; return a;'],
  ['statement-typescript-no-conversion', 'a=b; return a;', '(a=b) as unknown as string; return a;'],
);

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const unary = 'only unary +, - and ~ are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const negatives = [
  ['statement-string', '("x"); return a;', expression],
  ['statement-call', 'Math.random(); return a;', expression],
  ['statement-global', 'clock; return a;', reference],
  ['statement-unselected-conditional-call', '0?Math.random():a; return a;', expression],
  ['statement-unselected-logical-call', '0&&Math.random(); return a;', expression],
  ['statement-const-write', 'const x=a; x+(x=b); return a;', 'assignment to a const binding is not admitted'],
  ['statement-forward-reference', 'x+1; let x=a; return x;', reference],
  ['statement-after-return', 'return a; a+b;', 'statements after a guaranteed return are not admitted'],
  ['statement-expression-depth-limit', `${'('.repeat(129)}a${')'.repeat(129)}; return a;`, 'expression nesting exceeds'],
  ['statement-count-limit', `${'a;'.repeat(256)} return a;`, 'at most 256 statements'],
  ['statement-asserted-string', '("x") satisfies number; return a;', expression, 'ts'],
];
export const numericStatementRejected = negatives.map(([id, body, diagnostic, extension='js']) => [id,
  `export function run(a,b) { ${body} }`, extension, diagnostic]);
