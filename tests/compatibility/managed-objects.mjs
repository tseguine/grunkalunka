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

// Static own-property membership was admitted by the property-presence path.
// Keep the former gap source and its independent reference observation exact.
const membership = 'export function run(a,b) { return "x" in {x:a} ? 1 : 0; }';
export const promotedPropertyInGap = {
  id: 'gap-property-in', sourceJS: membership, sourceTS: membership, oracleSource: membership,
  args: [4, 2], entry: 'run', expected: 1,
  sourceSha256: '7ed37657a852b28a30a9aab0afc073a099b5e536cbeea62d397d951e2204f7aa',
};
managedObjects.push(['gap-property-in', '', '', 1, undefined,
  {sourceJS: membership, sourceTS: membership, oracleSource: membership}]);
