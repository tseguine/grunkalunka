// Boolean literals are admitted as predicates; source slots and returns stay numeric.
const cases = [
  ['boolean-true-if', 'if(true) return a; return b;'],
  ['boolean-false-if', 'if(false) return a; return b;'],
  ['boolean-true-ternary', 'return true ? a : b;'],
  ['boolean-false-ternary', 'return false ? a : b;'],
  ['boolean-nested-numeric', 'return a ? (true ? a : b) : (false ? b : a);'],
  ['boolean-true-write', 'if(true) { a+=b; } else { a-=b; } return a;'],
  ['boolean-false-write', 'if(false) { a+=b; } else { a-=b; } return a;'],
  ['boolean-while-false', 'while(false) { return a; } return b;'],
  ['boolean-while-true-return', 'while(true) { return a; } return b;'],
  ['boolean-for-true-return', 'for(;true;) { return a; } return b;'],
  ['boolean-for-false', 'for(;false;) { return a; } return b;'],
  ['boolean-comma-true', 'return (a=b,true) ? a : b;'],
  ['boolean-comma-false', 'return (a=b,false) ? a : -b;'],
  ['boolean-depth-boundary', `if(${'('.repeat(128)}true${')'.repeat(128)}) return a; return b;`],
];

export const booleanLiterals = cases.map(([id, body]) => [id, body, body]);
booleanLiterals.push(
  ['ts-boolean-as', 'if(true) return a; return b;', 'if(true as boolean) return a; return b;'],
  ['ts-boolean-angle', 'return false ? a : b;', 'return <boolean>false ? a : b;'],
);
