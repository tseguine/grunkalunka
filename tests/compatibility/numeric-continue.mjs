// Unlabelled continue reaches the nearest loop's test, or its step then test.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-forbodyevaluation
const cases = [
  ['continue-promoted-reference', 'let i=0; while(i<2) { i=i+1; if(i===1) continue; a=a+b; } return a;'],
  ['continue-promoted-for', 'for(let i=0;i<1;i++) { continue; } return a;'],
  ['continue-promoted-do', 'do { continue; } while(0); return a;'],
  ['continue-promoted-break-boundary', 'for(let i=0;i<3;i++) { continue; } return a;'],
  ['continue-while-terminal-test', 'let i=0,x=a; while(i++<3) { if(i<3) continue; x+=b; } return x+i;'],
  ['continue-for-step-probe', 'let g=0,s=0; for(;g++<3;s++) { continue; } return 10*g+s;'],
  ['continue-do-test-probe', 'let i=0,t=0; do { i++; if(i<3) continue; } while((t++,0)); return i*10+t;'],
  ['continue-do-three', 'let i=0,x=a; do { i++; if(i<3) continue; x+=b; } while(i<3); return x+i;'],
  ['continue-for-no-step', 'let g=0; for(;g++<3;) { continue; } return a+g;'],
  ['continue-for-no-test', 'let i=0; for(;;) { if(i++<2) continue; return a+i; } return b;'],
  ['continue-both-arms', 'let i=0,x=0; while(i++<3) { if(a) { x++; continue; } else { x+=10; continue; } } return x+i;'],
  ['continue-nested-nearest', 'let x=0; for(let i=0;i<3;i++) { let j=0; while(j++<3) { x++; continue; } x+=10; } return x;'],
  ['continue-restore-outer', 'let i=0,x=0; while(i++<3) { do { x++; continue; } while(0); if(i<3) continue; x+=10; } return x+i;'],
  ['continue-inner-outer-break', 'let x=0; for(let i=0;i<3;i++) { let j=0; while(j++<3) { x++; continue; } x+=10; break; } return x;'],
  ['continue-after-inner-break', 'let g=0,s=0,x=0; for(;g++<3;s++) { do { x++; break; } while(0); continue; } return 100*x+10*g+s;'],
  ['continue-mixed-return-break', 'for(let i=0;i<3;i++) { if(a) continue; if(b) break; return i; } return a+b;'],
  ['continue-do-mixed-return', 'do { if(a) continue; else return b; } while(0); return a;'],
  ['continue-do-return-suffix', 'do { if(a) continue; return b; } while(0); return a;'],
  ['continue-do-all-alternatives', 'let x=0; do { if(a) continue; x++; if(b) break; x+=10; return x; } while(0); return x;'],
  ['continue-body-reinitialization', 'let i=0,x=0; while(i++<3) { let y=0; y++; x+=y; continue; } return x;'],
  ['continue-header-body-shadow', 'let i=a,x=b; for(let i=0,j=0;i<3;i++,j++) { const x=i+j; if(i<2) continue; a=x; } return i+a+x;'],
  ['continue-inner-consumed-return', 'do { let i=0; while(i++<3) { continue; } return a; } while(0);'],
  ['continue-statement-boundary', `do { ${';'.repeat(252)} continue; } while(0); return a;`],
  ['continue-header-statement-boundary', `for(let i=0;0;) { ${';'.repeat(251)} continue; } return a;`],
  ['continue-depth-boundary', `${'while(false) '.repeat(64)}continue; return a;`],
  ['continue-local-boundary', `for(let i=0;0;) { const ${Array.from({length:127},(_,i)=>`x${i}=a`).join(',')}; continue; } return b;`],
];
export const numericContinue = cases.map(([id, body]) => [id, body, body]);
numericContinue.push(['ts-continue-predicate',
  'let g=0; for(;g++<3;) { if(a) continue; b+=g; } return b;',
  'let g:number=0; for(;g++<3;) { if(a as number) continue; b+=g; } return b;']);

const missing = 'every structural path must return a numeric value';
const abrupt = 'statements after a guaranteed abrupt completion';
const expression = 'expression is outside the numeric experiment';
const reference = 'reference is not an initialized';
const negatives = [
  ['continue-no-loop', 'continue; return a;'],
  ['continue-label', 'outer: while(0) { continue outer; } return a;', 'statement is outside'],
  ['continue-do-needs-return', 'do { continue; } while(0);', missing],
  ['continue-do-mixed-needs-return', 'do { if(a) continue; else return b; } while(0);', missing],
  ['continue-do-suffix-needs-return', 'do { if(a) continue; a++; return b; } while(0);', missing],
  ['continue-while-needs-return', 'while(0) { continue; }', missing],
  ['continue-for-needs-return', 'for(;0;) { continue; }', missing],
  ['continue-after-direct', 'while(0) { continue; a=b; } return a;', abrupt],
  ['continue-after-block', 'while(0) { { continue; } return a; } return b;', abrupt],
  ['continue-after-both-arms', 'while(0) { if(a) continue; else continue; return a; } return b;', abrupt],
  ['continue-after-mixed-break', 'while(0) { if(a) continue; else break; a=b; } return a;', abrupt],
  ['continue-after-mixed-return', 'while(0) { if(a) continue; else return b; a=b; } return a;', abrupt],
  ['continue-unselected-arm', 'while(0) { if(a) continue; else Math.random(); } return b;', expression],
  ['continue-unselected-step', 'for(;0;Math.random()) { continue; } return a;', expression],
  ['continue-unselected-do-test', 'do { continue; } while(Math.random()); return a;', expression],
  ['continue-for-body-step-scope', 'for(let i=0;i<1;j++) { let j=0; continue; } return a;', reference],
  ['continue-do-body-test-scope', 'do { let x=0; continue; } while(x); return a;', reference],
  ['continue-header-scope', 'for(let i=0;i<1;i++) { continue; } return i;', reference],
  ['continue-const-step', 'for(const i=0;0;i++) { continue; } return a;', 'update to a const binding is not admitted'],
  ['continue-switch-remains', 'while(0) { switch(a) { default: continue; } } return b;', 'statement is outside'],
  ['continue-finally-remains', 'while(0) { try { continue; } finally { a=b; } } return b;', 'statement is outside'],
  ['continue-statement-limit', `do { ${';'.repeat(253)} continue; } while(0); return a;`, 'at most 256 statements'],
  ['continue-header-statement-limit', `for(let i=0;0;) { ${';'.repeat(252)} continue; } return a;`, 'at most 256 statements'],
  ['continue-depth-limit', `${'while(false) '.repeat(65)}continue; return a;`, 'statement nesting exceeds 64'],
  ['continue-local-limit', `for(let i=0;0;) { const ${Array.from({length:128},(_,i)=>`x${i}=a`).join(',')}; continue; } return b;`, 'at most 128 source locals'],
];
export const numericContinueRejected = negatives.map(([id,body,diagnostic]) =>
  [id, `export function run(a,b) { ${body} }`, 'js', diagnostic]);
