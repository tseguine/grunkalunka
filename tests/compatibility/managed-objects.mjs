// Source-compiled WasmGC data objects; observations return through numeric ABI.
const cases = [
  ['managed-object-alias', 'const o={x:a};const alias=o;return o===alias?a:b;'],
  ['managed-object-snapshot', 'const o={x:a};const old=o.x;o.x=b;return old===a&&o.x===b?a:b;'],
  ['managed-object-receiver-order', 'let o={x:a};const old=o;o.x=(o={x:b},b);return old.x===b&&o.x===b?a:b;'],
  ['managed-object-selected', 'const o=a?{x:a}:{x:b};return o.x===a?a:b;'],
  ['managed-object-truthiness', 'const o={x:a,y:{}};return o.x&&o.y?a:b;'],
  ['managed-object-utf16', String.raw`const o={"\ud800\0":a,"\ufffd\0":b};return o["\ud800\0"]===a&&o["\ufffd\0"]===b?a:b;`],
];
export const managedObjects = cases.map(([id, body]) => [id, body, body]);
// Preserve the exact formerly rejected source, including its wrapping/spacing.
const original = 'export function run(a,b) { return ({x:a},a); }';
managedObjects.push(['managed-object-discard-original', 'return ({x:a},a);', 'return ({x:a},a);',
  undefined, undefined, {sourceJS: original, sourceTS: original}]);
const discarded = 'export function run(a,b) { ({}); return a; }';
managedObjects.push(['managed-object-statement-original', '({}); return a;', '({}); return a;',
  undefined, undefined, {sourceJS: discarded, sourceTS: discarded}]);
