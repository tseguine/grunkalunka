// Logical operators return numeric operand snapshots, with short-circuit effects.
const cases = [
  ['logical-and-value', 'return a && b;'],
  ['logical-and-effect', 'let x=0; const y=a && (x=x+1); return x+y;'],
  ['logical-or-value', 'return a || b;'],
  ['logical-or-effect', 'let x=0; const y=a || (x=x+1); return x+y;'],
  ['logical-and-left-once', 'let x=0; const y=(x=x+1) && (x=x+1); return x+y;'],
  ['logical-or-left-once', 'let x=0; const y=(x=x+1) || (x=x+1); return x+y;'],
  ['logical-and-state', 'let x=a; const y=x && (x=b); return y+x;'],
  ['logical-or-state', 'let x=a; const y=x || (x=b); return y+x;'],
  ['logical-or-snapshot', 'return (a || (a=b)) + (a=3);'],
  ['logical-and-nested', 'return a && (b || (a=b));'],
  ['logical-joined-value', 'return (a && b) || a;'],
  ['logical-precedence', 'return a || b && -a;'],
  ['logical-and-condition', 'return (a && b) ? -0 : b;'],
  ['logical-or-condition', 'return (a || b) ? a : -0;'],
  ['logical-comma-value', 'return (a=b,a) || (b=a+1,b);'],
  ['logical-and-loop', 'let i=3; while(i && (i=i-1)) { a+=b; } return a+i;'],
  ['logical-or-loop', 'let i=3; let x=0; while((i=i-1) || (x=x+1)-1) { a+=b; } return a+x;'],
];

export const numericLogical = cases.map(([id, body]) => [id, body, body]);
numericLogical.push(
  ['ts-logical-value', 'return a && b;', 'return (a && b) as number;'],
  ['ts-logical-predicate', 'return (a || b) ? a : b;', 'return ((a || b) as number) ? a : b;'],
);
