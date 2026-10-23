// Empty statements fall through without allocating IR values or source cells.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-empty-statement-runtime-semantics-evaluation
const cases = [
  ['empty-promoted-statement', '; return a;'],
  ['empty-multiple', '; ; return a;'],
  ['empty-nested-blocks', '{ ; { ; } ; } return a;'],
  ['empty-if-else', 'let x=a; if(a<b); else x=b; return x;'],
  ['empty-both-branches', 'if(a<b); else; return a;'],
  ['empty-for-body', 'let i=0; for(;i++<3;); return a+i;'],
  ['empty-while-body', 'let i=0; while(i++<3); return a+i;'],
  ['empty-do-body', 'let i=0; do; while(++i<3); return a+i;'],
  ['empty-false-for', 'for(;0;); return a;'],
  ['empty-false-while', 'while(0); return a;'],
  ['empty-statement-budget', `${';'.repeat(255)} return a;`],
  ['empty-depth-budget', `${'if(false)'.repeat(64)}; return a;`],
];
export const emptyStatementPrograms = cases.map(([id, body]) => [id, body, body]);

const returns = 'every structural path must return a numeric value';
const negatives = [
  ['empty-statement-limit', `${';'.repeat(256)} return a;`, 'at most 256 statements'],
  ['empty-depth-limit', `${'if(false)'.repeat(65)}; return a;`, 'statement nesting exceeds 64'],
  ['empty-only-missing-return', '; ;', returns],
  ['empty-after-return', 'return a; ;', 'statements after a guaranteed return are not admitted'],
  ['empty-return-without-value', 'return;', 'return must have a numeric expression'],
  ['empty-loop-missing-return', 'for(;;);', returns],
  ['empty-if-missing-return', 'if(a<b);', returns],
];
export const emptyStatementRejected = negatives.map(([id, body, diagnostic]) => [id,
  `export function run(a,b) { ${body} }`, 'js', diagnostic]);
emptyStatementRejected.push(['empty-module-prefix', '; export function run(a,b) { return a; }', 'js',
  'exactly one exported function and no directives/hashbang are admitted']);
