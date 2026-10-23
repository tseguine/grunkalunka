// Conditional predicates convert only the selected arm to truthiness.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-conditional-operator-runtime-semantics-evaluation
const cases = [
  ['predicate-conditional-boolean', 'if(a?true:false) return a; return b;'],
  ['predicate-conditional-comparison', 'return (a ? a<b : a==b) ? a : b;'],
  ['predicate-conditional-mixed', 'return (a ? true : b) ? a : b;'],
  ['predicate-conditional-mixed-reverse', 'return (a ? b : false) ? a : b;'],
  ['predicate-conditional-numeric', 'return (a ? a : b) ? a : b;'],
  ['predicate-conditional-boolean-test', 'return (true ? a<b : false) ? a : b;'],
  ['predicate-conditional-not', 'return !(a ? true : b<0) ? a : b;'],
  ['predicate-conditional-logical', 'return ((a ? true : b<0) && !false) ? a : b;'],
  ['predicate-conditional-logical-test', 'return ((a<b && b) ? false : true) ? a : b;'],
  ['predicate-conditional-nested', 'return (a ? b ? true : false : a<b ? false : true) ? a : b;'],
  ['predicate-conditional-test-once', 'let x=0; if(x++ ? true : false) x+=10; return x;'],
  ['predicate-conditional-true-effect', 'let x=0; if(true ? x++===0 : x++===1) x+=10; return x;'],
  ['predicate-conditional-false-effect', 'let x=0; if(false ? x++===0 : x++===1) x+=10; return x;'],
  ['predicate-conditional-comma', 'return (a=b, a ? true : false) ? a : b;'],
  ['predicate-conditional-while', 'let i=0; while(i<3 ? true : false) i++; return i;'],
  ['predicate-conditional-for', 'let i=0; for(;i<3 ? true : false;i++) { a+=b; } return a;'],
  ['predicate-conditional-do', 'let i=0; do i++; while(i<3 ? true : false); return i;'],
  ['predicate-conditional-depth-boundary', `if(${'a?true:'.repeat(128)}false) return a; return b;`],
];
export const predicateConditional = cases.map(([id, body]) => [id, body, body]);
predicateConditional.push(['ts-predicate-conditional',
  'return (a ? true : b<0) ? a : b;',
  'return ((a ? (true as boolean) : ((b<0) as boolean)) as boolean) ? a : b;']);
