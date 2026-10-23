// Logical negation produces predicates; source locals and returns remain numeric.
const cases = [
  ['not-number', 'return !a ? 1 : 0;'],
  ['not-twice', 'return !!a ? a : b;'],
  ['not-three-times', 'return !!!a ? a : b;'],
  ['not-true', 'if(!true) return a; return b;'],
  ['not-false', 'if(!false) return a; return b;'],
  ['not-less', 'return !(a<b) ? a : b;'],
  ['not-equal', 'return !(a===b) ? a : b;'],
  ['not-assignment-once', 'let x=0; if(!(x+=1)) x+=10; return x;'],
  ['not-postfix-once', 'let x=0; if(!x++) x+=10; return x;'],
  ['not-comma', 'return !(a=b,a) ? a : b;'],
  ['not-while', 'let i=0; while(!(i>=3)) i++; return i;'],
  ['not-for', 'let x=0; for(let i=0;!(i>=3);i++) x+=i; return x;'],
  ['not-depth-boundary', `if(${'!'.repeat(128)}a) return a; return b;`],
];
export const predicateNot = cases.map(([id, body]) => [id, body, body]);
predicateNot.push(
  ['ts-not-as', 'return !(a<b) ? a : b;', 'return !((a<b) as boolean) ? a : b;'],
  ['ts-not-nonnull', 'if(!false) return a; return b;', 'if(!false!) return a; return b;'],
);
