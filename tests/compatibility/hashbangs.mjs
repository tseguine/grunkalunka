// Complete original module text: hashbangs are comments, never body wrappers.
export const hashbangPrograms = [];
function add(id, prefix, body = 'return a+b;', modulePrefix = '', functionPrefix = '') {
  const sourceJS = `${prefix}${modulePrefix}export function run(a,b) { ${functionPrefix}${body} }`;
  const sourceTS = `${prefix}${modulePrefix}export function run(a:number,b:number):number { ${functionPrefix}${body} }`;
  hashbangPrograms.push([id, body, body, undefined, undefined, { sourceJS, sourceTS }]);
}
add('hashbang-promoted-reference', '#!/usr/bin/env node\n');
add('hashbang-promoted-ordinary', '#!/usr/bin/env node\n', 'return a;');
add('hashbang-promoted-directive', '#!/usr/bin/env node\n', 'return a;', '"hint";');
for (const [name, ending] of [['crlf', '\r\n'], ['cr', '\r'], ['ls', '\u2028'], ['ps', '\u2029']]) {
  add(`hashbang-${name}`, `#!node${ending}`);
}
add('hashbang-empty-comment', '#!\n');
add('hashbang-unicode-comment', '#!λ 🚀 \uFEFF \u0085\n');
add('hashbang-bom-after-comment', '#!node\n\uFEFF');
add('hashbang-directives', '#!node\n', 'return a+b;', '"module hint"\n', '"body hint";"use strict";');
add('hashbang-effects', '#!node\n', 'let x=a; for(let i=0;i<3;i++) { x+=b; } return ~(x++);');
for (const [moduleCount, bodyCount] of [[255, 0], [0, 255], [127, 128]]) {
  add(`hashbang-budget-${moduleCount}-${bodyCount}`, '#!node\n', ';'.repeat(bodyCount) + 'return a;',
    '"hint";'.repeat(moduleCount));
}
add('hashbang-expression-depth', '#!node\n', `return ${'('.repeat(128)}a${')'.repeat(128)};`);
add('hashbang-statement-depth', '#!node\n', `${'{'.repeat(64)}return a;${'}'.repeat(64)}`);

function byteLimitSource(typescript, extra = 0) {
  const prefix = '#!λ🚀';
  const suffix = typescript
    ? '\nexport function run(a:number,b:number):number { return a; }'
    : '\nexport function run(a,b) { return a; }';
  return prefix + ' '.repeat(16_384 + extra - Buffer.byteLength(prefix + suffix)) + suffix;
}
hashbangPrograms.push(['hashbang-original-byte-limit', 'return a;', 'return a;', undefined, undefined,
  { sourceJS: byteLimitSource(false), sourceTS: byteLimitSource(true) }]);

const source = 'export function run(a,b) { return a; }';
const top = 'exactly one exported function is admitted';
const parse = 'parse diagnostics';
export const hashbangRejected = [
  ...[['bom', '\uFEFF'], ['two-boms', '\uFEFF\uFEFF'], ['space', ' '], ['tab', '\t'],
    ['newline', '\n'], ['comment', '/* prefix */']].map(([id, prefix]) =>
    [`hashbang-prefix-${id}`, `${prefix}#!node\n${source}`, 'js', parse]),
  ['hashbang-ts-prefix-bom', '\uFEFF#!node\nexport function run(a:number,b:number):number { return a; }', 'ts', parse],
  ['hashbang-duplicate', `#!node\n#!node\n${source}`, 'js', parse],
  ['hashbang-nested', 'export function run(a,b) {\n#!node\nreturn a; }', 'js', parse],
  ['hashbang-split-marker', `# !node\n${source}`, 'js', parse],
  ['hashbang-empty-module', '#!', 'js', top],
  ['hashbang-comment-only', '#!node', 'js', top],
  ['hashbang-swallowed-export', `#!${source}`, 'js', top],
  ['hashbang-extra-export', `#!node\n${source}export const x=1;`, 'js', top],
  ['hashbang-module-empty-prefix', `#!node\n;${source}`, 'js', top],
  ['hashbang-wrong-entry', '#!node\nexport function other(a,b) { return a; }', 'js', 'expected a plain'],
  ['hashbang-unsupported-body', '#!node\nexport function run(a,b) { if(false) Math.random(); return a; }', 'js',
    'expression is outside the numeric experiment'],
  ['hashbang-strict-binding', '#!node\nexport function run(eval,b) { return b; }', 'js', 'semantic diagnostics'],
  ['hashbang-statement-limit', '#!node\nexport function run(a,b) { ' + ';'.repeat(256) + 'return a; }', 'js',
    'at most 256 statements'],
  ['hashbang-directive-limit', '#!node\n' + '"hint";'.repeat(256) + source, 'js', 'at most 256 statements'],
  ['hashbang-original-byte-overflow', byteLimitSource(false, 1), 'js', '16 KiB'],
  ['hashbang-ts-original-byte-overflow', byteLimitSource(true, 1), 'ts', '16 KiB'],
];
