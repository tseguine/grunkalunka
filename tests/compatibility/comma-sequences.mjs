// Numeric comma operands run in order; the final operand supplies the value.
const cases = [
  ['comma-last', 'return (a,b);'],
  ['comma-ordered', 'return (a=b,b=a+1,a+b);'],
  ['comma-final-predicate', 'return (a=b,a<b) ? a : -a;'],
  ['comma-loop-test-effects', 'let i=0; while((i=i+1,i<3)) { a+=b; } return a+i;'],
  ['comma-snapshot', 'return (a=b,a)+(a=3);'],
  ['comma-nested', 'return (a=b,(b=a+1,a+b));'],
  ['comma-updates', 'return (a++,b++,a+b);'],
  ['comma-compound', 'return (a=b,a+=1,a);'],
  ['comma-once', 'let x=0; const y=(x++,x++,x); return x+y;'],
  ['comma-signed-zero', 'return (a=b,-0);'],
  ['comma-nan-result', 'return (a=b,a/b);'],
  ['comma-numeric-predicate', 'return (a=b,a) ? -0 : b;'],
  ['comma-if-predicate', 'if((a=b,b=a,a===b)) return a; return b;'],
  ['comma-depth-boundary', `return ${'('.repeat(127)}a,b${')'.repeat(127)};`],
  ['comma-predicate-depth-boundary', `return ${'('.repeat(125)}a,a<b${')'.repeat(125)} ? a : b;`],
];

export const commaSequences = cases.map(([id, body]) => [id, body, body]);
commaSequences.push(
  ['ts-comma-value', 'return (a=b,a+1);', 'return (a=b,a+1) as number;'],
  ['ts-comma-predicate', 'return (a=b,a<b) ? -0 : b;', 'return ((a=b,a<b) as boolean) ? -0 : b;'],
);
