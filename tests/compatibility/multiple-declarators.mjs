// Lexical binding lists evaluate and initialize declarators left to right.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-runtime-semantics-bindingevaluation
const cases = [
  ['declarators-promoted-reference', 'const x=a,y=b; return x+y;'],
  ['declarators-dependent', 'let x=a,y=x+b; return x+y;'],
  ['declarators-three', 'const x=a,y=x+b,z=y-x; return z;'],
  ['declarators-order', 'let x=(a=1),y=(a=2); return 10*x+y+a;'],
  ['declarators-mutate-earlier', 'let x=a,y=(x=b),z=x+y; return z+x;'],
  ['declarators-snapshots', 'let x=a,y=(a=b),z=x; return x+y+z;'],
  ['declarators-inner-shadow', 'let x=a,y=b; { let x=b,y=x+a; a=x+y; } return a+x+y;'],
  ['declarators-branch', 'let x=a,y=b; if(a<b) { const z=x+y,w=z-x; return w; } return x;'],
  ['declarators-body-reentry', 'let i=0,x=a; while(i<3) { const y=b+i,z=y+1; x+=z; i++; } return x;'],
  ['declarators-for-promoted', 'for(let i=0,j=0;0;i++) {} return a;'],
  ['declarators-for-dependent', 'let x=a; for(let i=0,j=i+2;i<3;i++) x+=j+b; return x;'],
  ['declarators-for-scope', 'let i=a,j=b; for(let i=0,j=1;i<3;i++) j+=i; return i+j;'],
  ['declarators-for-const', 'for(const x=a,y=x+b;0;) {} return a;'],
  ['declarators-for-init-effects', 'for(let i=(a=1),j=(a=2);0;) {} return a;'],
  ['declarators-local-budget', `const ${Array.from({length:128},(_,i)=>`x${i}=a`).join(',')}; return a;`],
  ['declarators-statement-budget', `let x=a,y=b; ${'a;'.repeat(254)} return x+y;`],
  ['declarators-header-budget', `for(let i=0,j=0;0;) { ${'a;'.repeat(252)} } return a;`],
];
export const multipleDeclarators = cases.map(([id, body]) => [id, body, body]);
multipleDeclarators.push(['ts-multiple-declarators',
  'let x=a,y=x+b; return x+y;', 'let x:number=a,y:number=(x+b) as number; return x+y;']);

const reference = 'reference is not an initialized numeric parameter or local';
const expression = 'expression is outside the numeric experiment';
const negatives = [
  ['declarators-forward', 'let x=y,y=a; return x;', reference],
  ['declarators-self-later', 'let x=a,y=y; return x;', reference],
  ['declarators-shadow-forward', 'let x=a; { let y=x,x=b; return y; }', reference],
  ['declarators-shadow-self', 'let y=a; { let x=a,y=y; return x; }', reference],
  ['declarators-later-write-before-init', 'let x=(y=a),y=b; return x;', reference],
  ['declarators-write-earlier-const', 'const x=a,y=(x=b); return y;', 'assignment to a const binding is not admitted'],
  ['declarators-later-destructure', 'let x=a,[y]=b; return x;', 'local destructuring is not admitted'],
  ['declarators-later-string', 'let x=a,y="x"; return x;', expression],
  ['declarators-later-call', 'let x=a,y=Math.random(); return x;', expression],
  ['declarators-later-definite', 'let x:number=a,y!:number; return x;', 'definite-assignment assertions are not admitted', 'ts'],
  ['declarators-for-forward', 'for(let i=j,j=0;0;) {} return a;', reference],
  ['declarators-for-leak', 'for(let i=0,j=1;0;) {} return j;', reference],
  ['declarators-for-body-step-leak', 'for(let i=0,j=1;i<1;k++) { let k=0; i++; } return a;', reference],
  ['declarators-for-later-const-write', 'for(const i=0,j=(i=1);0;) {} return a;', 'assignment to a const binding is not admitted'],
  ['declarators-local-limit', `let ${Array.from({length:129},(_,i)=>`x${i}=a`).join(',')}; return a;`, 'at most 128 source locals'],
  ['declarators-cumulative-local-limit', `let ${Array.from({length:128},(_,i)=>`x${i}=a`).join(',')}; { let y=b; } return a;`, 'at most 128 source locals'],
  ['declarators-statement-limit', `let x=a,y=b; ${'a;'.repeat(255)} return x+y;`, 'at most 256 statements'],
  ['declarators-header-statement-limit', `for(let i=0,j=0;0;) { ${'a;'.repeat(253)} } return a;`, 'at most 256 statements'],
];
export const multipleDeclaratorsRejected = negatives.map(([id,body,diagnostic,extension='js']) =>
  [id, `export function run(a,b) { ${body} }`, extension, diagnostic]);
