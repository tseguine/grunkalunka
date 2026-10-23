// Unlabelled break exits only the nearest loop. Counters bound mutation probes too.
const cases = [
  ['break-promoted-reference', 'while(1) { a=a+b; break; } return a;'],
  ['break-while-parameter', 'while(a) { break; } return b;'],
  ['break-for-unconditional', 'for(;;) { break; } return a;'],
  ['break-do-once', 'do { break; } while(0); return a;'],
  ['break-while-test-effects', 'let i=0; while(i++<4) { if(i===2) break; a+=b; } return a+i;'],
  ['break-for-skips-step', 'let i=0,x=0; for(;i<3;x+=10) { i++; if(i===2) break; x++; } return x+i;'],
  ['break-do-skips-test', 'let i=0,t=0; do { i++; if(i===2) break; } while((t++,i<3)); return i*10+t;'],
  ['break-mixed-return', 'do { if(a) break; else return b; } while(0); return a;'],
  ['break-return-suffix', 'do { if(a) break; return b; } while(0); return a;'],
  ['break-through-normal-suffix', 'let x=0; do { if(a) break; x++; if(b) break; x+=10; return x; } while(0); return x;'],
  ['break-both-arms', 'let x=0; do { if(a) { x=1; break; } else { x=2; break; } } while(0); return x;'],
  ['break-nested-nearest', 'let x=0; for(let i=0;i<3;i++) { for(let j=0;j<3;j++) { x++; break; } x+=10; } return x;'],
  ['break-restore-outer', 'let i=0,x=0; while(i++<3) { do { x++; break; } while(0); x+=10; break; } return x+i;'],
  ['break-mixed-nested-loops', 'let x=0; do { for(let i=0;i<3;i++) { let j=0; while(j++<3) { x++; break; } x+=10; break; } x+=100; break; } while(0); return x;'],
  ['break-inner-consumed-return', 'do { while(true) { break; } return a; } while(0);'],
  ['break-shadow-restoration', 'let x=a; for(let i=0;i<3;i++) { let x=b; x++; if(i===1) break; a=x; } return x+a;'],
  ['break-body-reinitialization', 'let x=0; for(let i=0;i<3;i++) { let y=0; do { y++; break; } while(0); x+=y; } return x;'],
  ['break-statement-boundary', `do { ${';'.repeat(252)} break; } while(0); return a;`],
  ['break-depth-boundary', `${'while(false) '.repeat(64)}break; return a;`],
];
export const numericBreak = cases.map(([id, body]) => [id, body, body]);
numericBreak.push(['ts-break-predicate',
  'for(let i=0;i<3;i++) { if(a) break; b+=i; } return b;',
  'for(let i:number=0;i<3;i++) { if(a as number) break; b+=i; } return b;']);

const missing = 'every structural path must return a numeric value';
const abrupt = 'statements after a guaranteed abrupt completion';
const expression = 'expression is outside the numeric experiment';
const negatives = [
  ['break-no-loop', 'break; return a;'],
  ['break-label', 'outer: while(1) { break outer; } return a;', 'statement is outside'],
  ['break-switch-remains', 'switch(a) { default: break; } return b;', 'statement is outside'],
  ['break-do-needs-return', 'do { break; } while(0);', missing],
  ['break-do-mixed-needs-return', 'do { if(a) break; else return b; } while(0);', missing],
  ['break-do-suffix-needs-return', 'do { if(a) break; a++; return b; } while(0);', missing],
  ['break-after-direct', 'while(1) { break; a=b; } return a;', abrupt],
  ['break-after-block', 'while(1) { { break; } return a; } return b;', abrupt],
  ['break-after-mixed', 'while(1) { if(a) break; else return b; a=b; } return a;', abrupt],
  ['break-after-both-arms', 'while(1) { if(a) break; else break; return a; } return b;', abrupt],
  ['break-unselected-arm', 'while(0) { if(a) break; else Math.random(); } return b;', expression],
  ['break-unselected-step', 'for(;1;Math.random()) { break; } return a;', expression],
  ['break-unselected-do-test', 'do { break; } while(Math.random()); return a;', expression],
  ['break-initializer-still-numeric', 'for(let i=(true);0;) { break; } return a;', expression],
  ['break-body-scope', 'do { let x=a; break; } while(0); return x;', 'reference is not an initialized'],
  ['break-statement-limit', `do { ${';'.repeat(253)} break; } while(0); return a;`, 'at most 256 statements'],
  ['break-depth-limit', `${'while(false) '.repeat(65)}break; return a;`, 'statement nesting exceeds 64'],
];
export const numericBreakRejected = negatives.map(([id, body, diagnostic]) =>
  [id, `export function run(a,b) { ${body} }`, 'js', diagnostic]);
