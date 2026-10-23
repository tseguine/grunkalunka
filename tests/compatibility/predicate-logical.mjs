// Logical conditions discard operand values but retain their short-circuit effects.
const cases = [
  ['predicate-and-comparisons', 'return (a<b && a>0) ? a : b;'],
  ['predicate-or-comparisons', 'return (a<b || a===b) ? a : b;'],
  ['predicate-and-true', 'return (true && a) ? a : b;'],
  ['predicate-and-false', 'return (false && a) ? a : b;'],
  ['predicate-or-true', 'return (true || a) ? a : b;'],
  ['predicate-or-false', 'return (false || a) ? a : b;'],
  ['predicate-number-and', 'return (a && b<0) ? a : b;'],
  ['predicate-number-or', 'return (a || b<0) ? a : b;'],
  ['predicate-not-and', 'return (!a && b) ? a : b;'],
  ['predicate-not-nested', 'return !(a && b) ? a : b;'],
  ['predicate-mixed-tree', 'return ((a<b || !a) && (b || false)) ? a : b;'],
  ['predicate-and-skips-effect', 'let x=0; if(false && x++) x+=10; return x;'],
  ['predicate-or-skips-effect', 'let x=0; if(true || x++) x+=10; return x;'],
  ['predicate-and-effects-once', 'let x=0; if(x++===0 && x++===1) return x; return -x;'],
  ['predicate-or-effects-once', 'let x=0; if(x++!==0 || x++===1) return x; return -x;'],
  ['predicate-final-comma', 'return (a=b, true && a) ? a : b;'],
  ['predicate-while', 'let i=0; while(i<3 && true) i++; return i;'],
  ['predicate-for', 'let x=0; for(let i=0;true && i<3;i++) x+=i; return x;'],
  ['predicate-do', 'let i=0; do i++; while(i<3 && !false); return i;'],
  ['predicate-logical-depth-boundary', `if(${Array(129).fill('a').join('&&')}) return a; return b;`],
];
export const predicateLogical = cases.map(([id, body]) => [id, body, body]);
predicateLogical.push(['ts-predicate-logical',
  'return (a<b && !false) ? a : b;',
  'return ((a<b as boolean) && !(false as boolean)) ? a : b;']);
