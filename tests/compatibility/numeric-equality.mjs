// Equality predicates admit two primitive Number operands, with no coercion.
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal
const cases = [
  ['equality-inputs', 'return a==b?-0:0;'],
  ['inequality-inputs', 'return a!=b?-0:0;'],
  ['equality-self-nan', 'return a==a?b:-0;'],
  ['inequality-self-nan', 'return a!=a?-0:b;'],
  ['equality-zero', 'return a==0?-0:b;'],
  ['equality-left-snapshot', 'return a==(a=b)?-0:0;'],
  ['inequality-left-snapshot', 'return a!=(a=b)?-0:0;'],
  ['equality-operand-order', 'let x=0; return (x=1)==(x=x+1)?a:x;'],
  ['equality-rhs-once', 'let x=0; let y=a; if((y=b)==(x=x+1)) y=x; return x+y;'],
  ['inequality-counter-test', 'let i=0; let x=a; while((i=i+1)!=4) { x+=b; } return x+i;'],
  ['inequality-for-test', 'let x=a; for(let i=0;i!=3;i++) { x+=b; } return x;'],
  ['inequality-do-test', 'let i=0; do { a+=b; i++; } while(i!=3); return a;'],
  ['equality-comma-test', 'return (a=b,a==b)?-0:0;'],
  ['equality-logical-operands', 'return (a||b)==b?1:0;'],
  ['equality-negated-predicate', 'return !(a==b)?-0:0;'],
  ['inequality-promoted-reference', 'return a!=b?1:0;'],
  ['equality-promoted-rejection', 'return a==b?a:b;'],
  ['equality-depth-budget', `return ${'('.repeat(126)}a==b${')'.repeat(126)}?-0:0;`],
];
export const numericEqualityPrograms = cases.map(([id, body]) => [id, body, body]);
numericEqualityPrograms.push(['equality-typescript-wrappers',
  'return a==b?-0:0;',
  'return (((a as number)! == (<number>b)) as boolean)?-0:0;']);

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, &, |, ^, <<, >> and >>> are admitted';
const negatives = [
  ['equality-boolean-left', 'return true==1?a:b;', expression],
  ['equality-boolean-right', 'return 1==true?a:b;', expression],
  ['inequality-boolean-left', 'return true!=1?a:b;', expression],
  ['inequality-boolean-right', 'return 1!=true?a:b;', expression],
  ['equality-string-left', 'return "4"==a?1:0;', expression],
  ['inequality-string-right', 'return a!="4"?1:0;', expression],
  ['equality-null-left', 'return null==a?1:0;', expression],
  ['inequality-null-right', 'return a!=null?1:0;', expression],
  ['equality-object', 'return a==({})?1:0;', expression],
  ['inequality-bigint', 'return a!=1n?1:0;', expression],
  ['equality-comparison-operand', 'return (a<b)==0?1:0;', binary],
  ['equality-nested-operand', 'return (a==b)==0?1:0;', binary],
  ['equality-numeric-return', 'return a==b;', binary],
  ['equality-numeric-local', 'const x=a==b; return a;', binary],
  ['inequality-arithmetic', 'return (a!=b)+a;', binary],
  ['equality-unselected-call', 'return 0?(a==Math.random()?1:0):b;', expression],
  ['equality-asserted-string', 'return ((a==("4" as number)) as boolean)?1:0;', expression, 'ts'],
  ['equality-expression-statement', 'a==b; return a;', binary],
  ['equality-predicate-depth-limit', `return ${'('.repeat(127)}a==b${')'.repeat(127)}?a:b;`, 'expression nesting exceeds'],
  ['inequality-operand-depth-limit', `return ${'('.repeat(127)}a${')'.repeat(127)}!=b?a:b;`, 'expression nesting exceeds'],
];
export const numericEqualityRejected = negatives.map(([id, body, diagnostic, extension='js']) => [id,
  `export function run(a,b) { ${body} }`, extension, diagnostic]);
