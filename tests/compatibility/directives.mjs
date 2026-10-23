// Only Oxc-recognized prologues erase. Complete module text stays complete.
export const directivePrograms = [];
function add(id, modulePrefix, functionPrefix, body = 'return a+b;') {
  const sourceJS = `${modulePrefix}export function run(a,b) { ${functionPrefix}${body} }`;
  const sourceTS = `${modulePrefix}export function run(a:number,b:number):number { ${functionPrefix}${body} }`;
  directivePrograms.push([id, body, body, undefined, undefined, { sourceJS, sourceTS }]);
}
add('directive-promoted-module', '"use strict"; ', '');
add('directive-promoted-function', '', '"use strict"; ');
add('directive-ordinary-module', '"use strict"; ', '', 'return a;');
add('directive-ordinary-function', '', '"use strict"; ', 'return a;');
add('directive-module-unknown', '"module hint";\r\n', '');
add('directive-function-unknown', '', '"body hint"; ');
add('directive-multiple', '"one";\'two\';"use strict"; ', '"three";"use strict";"use strict"; ');
add('directive-asi', '"module hint"\n', '"body hint"\n"use strict"\n');
add('directive-comments', '/* prefix */ "hint"; // next\n', '"one";/* middle */"two"; // body\n');
add('directive-escaped', '"\\u0075se strict"; ', '"escaped\\ntext"; ');
add('directive-line-continuation', '"module' + '\\' + '\n' + 'hint"; ', '"body' + '\\' + '\n' + 'hint"; ');
add('directive-empty-unicode', '"";"λ"; ', '"";"π"; ');
add('directive-effects-follow', '"hint"; ', '"hint"; ', 'let x=a; x+=b; return x;');
add('directive-finite-loop', '"hint"; ', '"hint"; ', 'for(let i=0;i<3;i++) { a+=b; } return a;');
for (const [moduleCount, functionCount] of [[255, 0], [0, 255], [127, 128]]) {
  add(`directive-budget-${moduleCount}-${functionCount}`, '"hint";'.repeat(moduleCount),
    '"hint";'.repeat(functionCount), 'return a;');
}
add('directive-expression-depth', '', '"hint";', `return ${'('.repeat(128)}a${')'.repeat(128)};`);
add('directive-statement-depth', '', '"hint";', `${'{'.repeat(64)}return a;${'}'.repeat(64)}`);

const source = body => `export function run(a,b) { ${body} }`;
const expression = 'expression is outside the numeric experiment';
const top = 'exactly one exported function is admitted';
export const directiveRejected = [
  ['directive-module-budget-limit', '"hint";'.repeat(256) + source('return a;'), 'js', 'at most 256 statements'],
  ['directive-function-budget-limit', source('"hint";'.repeat(256) + 'return a;'), 'js', 'at most 256 statements'],
  ['directive-mixed-budget-limit', '"hint";'.repeat(128) + source('"hint";'.repeat(128) + 'return a;'), 'js', 'at most 256 statements'],
  ['directive-body-statement-budget', '"hint";' + source(';'.repeat(255) + 'return a;'), 'js', 'at most 256 statements'],
  ['directive-nonprologue', source('; "hint"; return a;'), 'js', expression],
  ['directive-parenthesized', source('("hint"); return a;'), 'js', expression],
  ['directive-nested-block', source('{ "hint"; } return a;'), 'js', expression],
  ['directive-template', source('`hint`; return a;'), 'js', expression],
  ['directive-ts-wrapped', 'export function run(a:number,b:number):number { "hint" as string; return a; }', 'ts', expression],
  ['directive-asi-add', source('"hint"\n+a; return a;'), 'js', expression],
  ['directive-asi-call', source('"hint"\n(a); return a;'), 'js', expression],
  ['directive-post-export', source('return a;') + '"hint";', 'js', top],
  ['directive-extra-export', '"hint";' + source('return a;') + 'export const x=1;', 'js', top],
  ['directive-call-remains', source('"hint"; Math.random(); return a;'), 'js', expression],
  ['directive-missing-return', source('"hint";'), 'js', 'every structural path must return'],
  ['directive-duplicate-parameter', '"hint"; export function run(a,a) { "hint"; return a; }'],
  ['directive-strict-binding', 'export function run(eval,b) { "hint"; return b; }'],
  ['directive-invalid-octal', '"\\1";' + source('return a;')],
  ['directive-malformed-string', '"unterminated\n' + source('return a;')],
  ['directive-unselected-call', source('"hint"; if(false) Math.random(); return a;'), 'js', expression],
  ['directive-depth-limit', source(`"hint"; return ${'('.repeat(129)}a${')'.repeat(129)};`), 'js', 'expression nesting exceeds'],
];
