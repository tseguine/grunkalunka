// Finite numeric for loops; counters never depend on ABI input values.
// Evaluation order: https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-forbodyevaluation
const cases = [
  ['for-zero', 'for (;0;) { a=b; } return a;'],
  ['for-three', 'let x=a; for(let i=0;i<3;i++) { x+=b; } return x;'],
  ['for-step-after-body', 'let x=0; for(let i=0;i<3;i++) { x=x*10+i; } return x;'],
  ['for-return-skips-step', 'for(;;a=b) { return a; } return b;'],
  ['for-omitted-init', 'let i=0; let x=a; for(;i<3;i++) { x+=b; } return x;'],
  ['for-omitted-step', 'let i=0; let x=a; for(;i<3;) { x+=b; i++; } return x;'],
  ['for-omitted-test', 'let x=a; for(let i=0;;i++) { if(i===3) return x; x+=b; } return b;'],
  ['for-all-omitted', 'for(;;) { return a; } return b;'],
  ['for-initializer-once', 'let i=0; let x=0; for(x++;i<3;i++) { a+=b; } return x;'],
  ['for-terminal-test-effect', 'let i=0; let x=a; for(;(i=i+1)<4;b=b+1) { x+=b; } return x+i+b;'],
  ['for-body-initialization', 'let x=a; for(let i=0;i<3;i++) { const y=b+i; x+=y; } return x;'],
  ['for-nested-shadow', 'let i=a; let x=0; for(let i=0;i<2;i++) { for(let j=0;j<2;j++) { const i=b; x+=i; } } return x+i;'],
  ['for-body-shadow-step', 'let x=0; for(let i=0;i<3;i++) { let i=9; x+=i; } return x;'],
  ['for-const-header', 'let i=0; let x=0; for(const y=a;i<3;i++) { x+=y; } return x;'],
  ['for-value-headers', 'for(a+b;0;a*b) {} return a;'],
  ['for-value-step', 'let i=0; let x=a; for(0;i<3;a+b) { x+=b; i++; } return x;'],
  ['for-promoted-reference', 'let x=a; for(let i=0;i<2;i=i+1) x=x+b; return x;'],
  ['for-header-statement-budget', `for(let i=0;0;i++) { ${'a=b;'.repeat(252)} } return a;`],
  ['for-expression-statement-budget', `for(a=0;0;b++) { ${'a=b;'.repeat(253)} } return a;`],
  ['for-header-local-budget', `${Array.from({length:127},(_,i)=>`const x${i}=a;`).join(' ')} for(let i=0;0;i++) {} return a;`],
  ['for-source-depth-budget', `${Array.from({length:64},(_,i)=>`for(let i${i}=0;0;i${i}++)`).join('')} {} return a;`],
  ['for-header-expression-depth', `for(${'('.repeat(128)}a${')'.repeat(128)};${'('.repeat(128)}0${')'.repeat(128)};${'('.repeat(128)}b${')'.repeat(128)}) {} return a;`],
];
export const numericForPrograms = cases.map(([id, body]) => [id, body, body]);
numericForPrograms.push(['for-typescript-headers',
  'let x=a; for(let i=0;i<3;i++) { x+=b; } return x;',
  'let x=a; for(let i:number=0;(i<3) as boolean;(<number>(i++))) { x+=b; } return x;']);

numericForPrograms.push(['for-composed-headers',
  'let x=a; for(let i=0;(3-i)&&1;(i++,x+=b)) {} return x;',
  'let x:number=a; for(let i:number=0;(3-i)&&1;(i++,x+=b)) {} return x;']);

const statement = 'statement is outside the numeric experiment';
const expression = 'expression is outside the numeric experiment';
const reference = 'reference is not an initialized numeric parameter or local';
const declaration = 'one initialized const or let identifier per declaration is admitted';
const assignmentConst = 'assignment to a const binding is not admitted';
const returns = 'every structural path must return a numeric value';
const negatives = [
  ['for-var', 'for(var i=0;0;i++) {} return a;', declaration],
  ['for-multiple-declarations', 'for(let i=0,j=0;0;i++) {} return a;', declaration],
  ['for-destructure', 'for(const [i]=a;0;) {} return a;', 'local destructuring is not admitted'],
  ['for-uninitialized', 'for(let i;0;i++) {} return a;', 'numeric local must have an initializer'],
  ['for-self-initializer', 'for(let i=i;0;i++) {} return a;', reference],
  ['for-self-write', 'for(let i=(i=0);0;i++) {} return a;', reference],
  ['for-shadow-self-initializer', 'let i=a; for(let i=i;0;i++) {} return a;', reference],
  ['for-header-leak', 'for(let i=0;0;i++) {} return i;', reference],
  ['for-body-local-step', 'for(let i=0;i<1;x++) { let x=0; i++; } return a;', reference],
  ['for-const-step-update', 'for(const i=0;i<1;i++) {} return a;', 'update to a const binding is not admitted'],
  ['for-const-step-write', 'for(const i=0;0;i=1) {} return a;', assignmentConst],
  ['for-const-test-write', 'for(const i=0;i=1;) {} return a;', assignmentConst],
  ['for-unselected-body', 'for(;0;) { a=Math.random(); } return a;', expression],
  ['for-unselected-step', 'for(;1;Math.random()) { return a; } return b;', expression],
  ['for-test-call', 'for(;Math.random();) {} return a;', expression],
  ['for-init-call', 'for(Math.random();0;) {} return a;', expression],
  ['for-missing-return', 'for(let i=0;i<1;i++) { return a; }', returns],
  ['for-omitted-test-return-policy', 'for(;;) { return a; }', returns],
  ['for-in', 'for(const i in {}) {} return a;', statement],
  ['for-of', 'for(const i of [a]) { return i; } return b;', statement],
  ['for-break', 'for(;;) { break; } return a;', statement],
  ['for-continue', 'for(let i=0;i<1;i++) { continue; } return a;', statement],
  ['for-label', 'outer: for(;;) { return a; } return b;', statement],
  ['for-boolean-test', 'for(;true;) { return a; } return b;', expression],
  ['for-header-statement-limit', `for(let i=0;0;i++) { ${'a=b;'.repeat(253)} } return a;`, 'at most 256 statements'],
  ['for-header-local-limit', `${Array.from({length:128},(_,i)=>`const x${i}=a;`).join(' ')} for(let i=0;0;i++) {} return a;`, 'at most 128 numeric locals'],
  ['for-source-depth-limit', `${Array.from({length:65},(_,i)=>`for(let i${i}=0;0;i${i}++)`).join('')} {} return a;`, 'statement nesting exceeds 64'],
  ['for-init-depth-limit', `for(${'('.repeat(129)}a${')'.repeat(129)};0;) {} return a;`, 'expression nesting exceeds'],
  ['for-test-depth-limit', `for(;${'('.repeat(129)}0${')'.repeat(129)};) {} return a;`, 'predicate nesting exceeds'],
  ['for-step-depth-limit', `for(;0;${'('.repeat(129)}b${')'.repeat(129)}) {} return a;`, 'expression nesting exceeds'],
];
export const numericForRejected = negatives.map(([id, body, diagnostic]) => [id,
  `export function run(a,b) { ${body} }`, 'js', diagnostic]);
