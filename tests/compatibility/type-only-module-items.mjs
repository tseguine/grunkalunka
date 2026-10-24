// Declaration-level TypeScript module items erase; explicit complete JS oracles.
export const typeOnlyModulePromotions = [
  ['type-declaration-excluded-import', 'import type { T } from "missing";export function run(a,b) { return a; }', '5d80a33a5f8b9b0eae7dbfa4ecc3465e25b72845f302c3e1efbdc2ef5086b68c'],
  ['type-declaration-excluded-export-list', 'type T=number; export type { T };export function run(a,b) { return a; }', '9ba6dfc1ef3b88a603367efb91001432f27af95e9bf21fbb4dfdd8079f489a8d'],
  ['type-declaration-excluded-re-export', 'export type { T } from "missing";export function run(a,b) { return a; }', 'b68b6c5112b11abd6ddc235c6dcbb421440f9efdfff8cdccee997ceee5e1b2fc'],
  ['type-declaration-excluded-export-star', 'export type * from "missing";export function run(a,b) { return a; }', 'cfcb48e3373a872b0e6e94f00569090ae3b0c1e907c8cd750d320139e4d9ccb4'],
];
export const typeOnlyModuleItems = [];
const identity = 'export function run(a,b) { return a; }';
function full(id, sourceJS, sourceTS) {
  typeOnlyModuleItems.push([id, '', '', undefined, undefined, { sourceJS, sourceTS }]);
}
for (const [id, ts] of typeOnlyModulePromotions) full(id, identity, ts);
for (const [id, declaration] of [
  ['default-import', 'import type T from "missing";'],
  ['namespace-import', 'import type * as Types from "missing";'],
  ['aliased-import', 'import type { T as Local, U } from "missing";'],
  ['export-aliases', 'type T=number;interface U {} export type { T as Local, U };'],
  ['export-empty', 'export type {};'],
  ['re-export-aliases', 'export type { T as Local, U } from "missing";'],
  ['export-namespace', 'export type * as Types from "missing";'],
]) {
  const js = 'function twice(x) { return x*2; } export function run(a,b) { return twice(a)+b; }';
  full(`type-only-module-${id}-prefix`, js, declaration + js);
  full(`type-only-module-${id}-suffix`, js, js + declaration);
}
for (const [id, js, ts] of [
  ['effects', 'export function run(a,b) { let x=a; const y=(x+=b); return y+(x=a); }',
    'import type { T } from "missing";export function run(a:T,b:T):T { let x:T=a; const y:T=(x+=b); return y+(x=a); }export type { T };'],
  ['loop', 'export function run(a,b) { let x=a; for(let i=0;i<3;i++) x+=b; return x; }',
    'export type * from "missing";export function run(a,b) { let x=a; for(let i=0;i<3;i++) x+=b; return x; }'],
  ['math-namespace', 'export function run(a,b) { return Math.abs(a)+b; }',
    'import type { Math } from "missing";export function run(a,b) { return Math.abs(a)+b; }'],
  ['number-intrinsics', 'export function run(a,b) { return Infinity+NaN; }',
    'import type { Infinity, NaN } from "missing";export function run(a,b) { return Infinity+NaN; }'],
  ['value-shadow', 'export function run(a,b) { const Math=a; return Math+b; }',
    'import type { Math } from "missing";export function run(a,b) { const Math=a; return Math+b; }'],
  ['parameter-shadow', 'export function run(T,b) { return T+b; }',
    'import type { T } from "missing";export function run(T,b) { return T+b; }'],
  ['wrapper-depth', identity,
    'import type { T } from "missing";' + `export function run(a,b) { return ${'('.repeat(128)}a${')'.repeat(128)}; }`],
]) full(`type-only-module-${id}`, js, ts);
export function typeOnlyItems(count, kind = 'import') {
  return Array.from({length:count}, (_,i) => kind === 'import' ? `import type { T as T${i} } from "missing";` :
    kind === 'list' ? 'export type {};' : kind === 'from' ? `export type { T${i} } from "missing";` :
      `export type * from "missing${i}";`).join('');
}
for (const kind of ['import','list','from','all']) full(`type-only-module-budget-${kind}`, identity, typeOnlyItems(255,kind)+identity);
const helper = 'function twice(x) { return x*2; } export function run(a,b) { return twice(a); }';
full('type-only-module-helper-budget', helper, typeOnlyItems(253)+helper);
full('type-only-module-shared-budget', identity,
  '"module hint";' + typeOnlyItems(127) + 'export function run(a,b) { ' +
  Array.from({length:127},(_,i)=>`type T${i}=number;`).join('') + 'return a; }');
function byteSource(extra = 0) {
  const prefix = 'import type { T } from "λ';
  const suffix = '";' + identity;
  return prefix + 'x'.repeat(16_384+extra-Buffer.byteLength(prefix+suffix)) + suffix;
}
full('type-only-module-byte-boundary', identity, byteSource());

export const typeOnlyModuleRejected = [];
export const typeOnlyModuleRejectionGroups = new Map();
function reject(id, source, diagnostic, group = 'type-module-forms', extension = 'ts') {
  id = `type-only-module-reject-${id}`;
  typeOnlyModuleRejected.push([id, source, extension, diagnostic]);
  typeOnlyModuleRejectionGroups.set(id, group);
}
for (const [id, prefix] of [
  ['import-value', 'import { T } from "missing";'], ['import-default', 'import T from "missing";'],
  ['import-namespace', 'import * as Types from "missing";'], ['import-side-effect', 'import "missing";'],
  ['import-empty', 'import {} from "missing";'], ['import-inline', 'import { type T } from "missing";'],
  ['import-mixed', 'import { type T, value } from "missing";'],
  ['export-empty', 'export {};'], ['export-inline', 'type T=number;export { type T };'],
  ['export-from', 'export { T } from "missing";'], ['export-from-inline', 'export { type T } from "missing";'],
  ['export-all', 'export * from "missing";'], ['export-namespace', 'export * as Types from "missing";'],
  ['import-equals', 'import T = require("missing");'], ['import-type-equals', 'import type T = require("missing");'],
]) reject(id, prefix+identity, 'exactly one exported function');
reject('unknown-type-value', 'import type { T } from "missing";export function run(a,b) { return T; }', 'reference is not an initialized numeric parameter or local');
reject('type-value-tdz', 'import type { T } from "missing";export function run(a,b) { const T=T; return T; }', 'reference is not an initialized numeric parameter or local');
reject('math-value-shadow', 'import type { Math } from "missing";export function run(a,b) { const Math=a; return Math.abs(a); }', 'expression is outside');
reject('dynamic-import', 'import type { T } from "missing";export function run(a,b) { return import("missing"); }', 'expression is outside');
reject('only-types', 'import type { T } from "missing";export type { T };', 'exactly one exported function');
reject('bare-entry', 'import type { T } from "missing";function run(a,b) { return a; }', "expected 'export function");
reject('type-export-entry', 'export type { run };function run(a,b) { return a; }', "expected 'export function");
reject('js-mode', 'import type { T } from "missing";'+identity, 'parse diagnostics', 'malformed-js', 'js');
reject('malformed-import', 'import type { T from "missing";'+identity, 'parse diagnostics', 'malformed-ts');
reject('duplicate-value', 'import type { T } from "missing";export function run(a,b) { const x=a;const x=b;return x; }', 'semantic diagnostics', 'duplicate-lexical-binding');
for (const kind of ['import','list','from','all']) reject(`budget-${kind}`, typeOnlyItems(256,kind)+identity, 'at most 256 statements', 'statement-count');
reject('directive-budget', '"module hint";'+typeOnlyItems(255)+identity, 'at most 256 statements', 'statement-count');
reject('helper-budget', typeOnlyItems(254)+helper, 'at most 256 statements', 'statement-count');
reject('byte-overflow', byteSource(1), '16 KiB', 'source-bytes');
reject('depth-overflow', 'import type { T } from "missing";'+`export function run(a,b) { return ${'('.repeat(129)}a${')'.repeat(129)}; }`, 'expression nesting exceeds', 'expression-depth');
