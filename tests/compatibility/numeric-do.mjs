// Finite numeric do-while loops; counters never depend on ABI input values.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-runtime-semantics-dowhileloopevaluation
const cases = [
  ['do-promoted-reference', 'let x=a; do { x=x+b; } while(0); return x;'],
  ['do-zero', 'do { a+=b; } while(0); return a;'],
  ['do-false', 'do { a+=b; } while(false); return a;'],
  ['do-negative-zero', 'do { a=b; } while(-0); return a;'],
  ['do-nan', 'do { a+=b; } while(0/0); return a;'],
  ['do-three', 'let i=0; do { a+=b; i++; } while(i<3); return a;'],
  ['do-terminal-test-effect', 'let i=0; let x=a; do { x+=b; } while((i=i+1)<3); return x+i;'],
  ['do-body-initialization', 'let i=0; let x=a; do { const y=b+i; x+=y; i++; } while(i<3); return x;'],
  ['do-nested-shadow', 'let i=a; let x=0; let k=0; do { let i=0; do { x+=b; i++; } while(i<2); k++; } while(k<2); return x+i;'],
  ['do-body-shadow-test', 'let i=0; let x=0; do { const i=b; x+=i; } while((i=i+1)<3); return x+i;'],
  ['do-conditional-return', 'let i=0; do { if(i===2) return a+b; i++; } while(i<3); return b;'],
  ['do-return-skips-test', 'do { return a; } while(a=b);'],
  ['do-both-branches-return', 'do { if(a<b) return a; else return b; } while(a=b);'],
  ['do-nested-return', 'do { do { return a; } while(0); } while(a=b);'],
  ['do-composed-test', 'let i=0; do { a+=b; } while((i++, (3-i)&&1)); return a;'],
  ['do-statement-budget', `do { ${'a=b;'.repeat(253)} } while(0); return a;`],
  ['do-source-depth-budget', `${'do '.repeat(64)} {} ${'while(0);'.repeat(64)} return a;`],
  ['do-local-budget', `do { ${Array.from({length:128},(_,i)=>`const x${i}=a;`).join(' ')} } while(0); return a;`],
  ['do-test-depth-budget', `do { a=b; } while(${'('.repeat(128)}0${')'.repeat(128)}); return a;`],
  ['do-expression-body', 'do a+=b; while(0); return a;'],
];
export const numericDoPrograms = cases.map(([id, body]) => [id, body, body]);
numericDoPrograms.push(['do-typescript-test',
  'let i=0; do { a+=b; i++; } while(i<3); return a;',
  'let i:number=0; do { a+=b; i++; } while((i<3) as boolean); return a;']);

const expression = 'expression is outside the numeric experiment';
const statement = 'statement is outside the numeric experiment';
const reference = 'reference is not an initialized numeric parameter or local';
const afterReturn = 'statements after a guaranteed return are not admitted';
const negatives = [
  ['do-unsupported-test-after-return', 'do { return a; } while(Math.random());', expression],
  ['do-string-test', 'do { a=b; } while("x"); return a;', expression],
  ['do-unsupported-body', 'do { a=Math.random(); } while(0); return a;', expression],
  ['do-body-local-test', 'do { const x=0; } while(x); return a;', reference],
  ['do-body-local-leak', 'do { const x=0; } while(0); return x;', reference],
  ['do-self-initializer', 'do { const x=x; } while(0); return a;', reference],
  ['do-const-body-write', 'const x=a; do { x=b; } while(0); return a;', 'assignment to a const binding is not admitted'],
  ['do-const-test-write', 'const x=a; do {} while(x=b); return a;', 'assignment to a const binding is not admitted'],
  ['do-label', 'outer: do { a=b; } while(0); return a;', statement],
  ['do-after-guaranteed-return', 'do { return a; } while(0); return b;', afterReturn],
  ['do-after-guaranteed-branches', 'do { if(a<b) return a; else return b; } while(0); a=b;', afterReturn],
  ['do-missing-return', 'do { a=b; } while(0);', 'every structural path must return a numeric value'],
  ['do-statement-limit', `do { ${'a=b;'.repeat(254)} } while(0); return a;`, 'at most 256 statements'],
  ['do-local-limit', `do { ${Array.from({length:129},(_,i)=>`const x${i}=a;`).join(' ')} } while(0); return a;`, 'at most 128 numeric locals'],
  ['do-source-depth-limit', `${'do '.repeat(65)} {} ${'while(0);'.repeat(65)} return a;`, 'statement nesting exceeds 64'],
  ['do-test-depth-limit', `do {} while(${'('.repeat(129)}0${')'.repeat(129)}); return a;`, 'predicate nesting exceeds'],
];
export const numericDoRejected = negatives.map(([id, body, diagnostic]) => [id,
  `export function run(a,b) { ${body} }`, 'js', diagnostic]);
