// Explicit aliases/interfaces stay on the original AST and emit no runtime code.
export const typeDeclarationPrograms = [];
function full(id, sourceJS, sourceTS) {
  typeDeclarationPrograms.push([id, '', '', undefined, undefined, { sourceJS, sourceTS }]);
}
function add(id, bodyJS, bodyTS = bodyJS, prefix = '', suffix = '') {
  full(id, `export function run(a,b) { ${bodyJS} }`,
    `${prefix}export function run(a:unknown,b:unknown):unknown { ${bodyTS} }${suffix}`);
}
full('type-declaration-promoted-reference', 'export function run(a,b) { return a; }',
  'type Scalar = number; export function run(a:Scalar,b:Scalar):Scalar { return a; }');
full('type-declaration-promoted-alias', 'export function run(a,b) { return a; }',
  'type Scalar=number; export function run(a:Scalar,b:Scalar):Scalar { return a; }');
full('type-declaration-promoted-interface', 'export function run(a,b) { return a; }',
  'interface Input { x:number } export function run(a:Input,b:Input):Input { return a; }');
full('type-declaration-compound-shift-snapshot',
  'export function run(a,b) { let x=a; const y=(x >>>= (x=b)); return y+x; }',
  'type Word=number; export function run(a:Word,b:Word):Word { let x:Word=a; const y:Word=(x >>>= (x=b)); return y+x; type Done=Word; }');
for (const [name, declaration] of [
  ['alias', 'type T=number;'], ['interface', 'interface T { x:number }'],
  ['declared-alias', 'declare type T=number;'], ['declared-interface', 'declare interface T { x:number }'],
  ['export-alias', 'export type T=number;'], ['export-interface', 'export interface T { x:number }'],
  ['export-declared-alias', 'export declare type T=number;'],
  ['export-declared-interface', 'export declare interface T { x:number }'],
  ['default-interface', 'export default interface T { x:number }'],
  ['generic-recursive', 'type T<X=number>={ value:X,next?:T<X> }; interface I<X> extends T<X> { items:readonly X[] }'],
]) add(`type-declaration-module-${name}`, 'return a+b;', 'return a+b;', declaration);
add('type-declaration-module-suffix', 'return a+b;', 'return a+b;', 'type Before=number;', 'interface After {}');
add('type-declaration-function', 'const x=a; return x+b;', 'type T=number; interface I { x:T } const x:T=a; return x+b;');
add('type-declaration-block', '{ const x=a; a=x+b; } return a;', '{ type T=number; interface I {} const x:T=a; a=x+b; } return a;');
add('type-declaration-same-name-value', 'const T=a; let I=b; return T+I;',
  'type T=number; const T=a; interface I {} let I=b; return T+I;');
add('type-declaration-type-only-intrinsics', 'return Infinity+NaN;', 'type Infinity=number; interface NaN {} return Infinity+NaN;');
add('type-declaration-runtime-intrinsic-shadow', 'const Infinity=a; let NaN=b; return Infinity+NaN;',
  'type Infinity=number; const Infinity=a; interface NaN {} let NaN=b; return Infinity+NaN;');
add('type-declaration-after-return', 'return a;', 'return a; type T=number; interface I {}');
add('type-declaration-after-break', 'do { break; } while(0); return a;', 'do { break; type T=number; interface I {} } while(0); return a;');
add('type-declaration-after-continue', 'for(let i=0;i<2;i++) { a+=b; continue; } return a;',
  'for(let i=0;i<2;i++) { a+=b; continue; type T=number; interface I {} } return a;');
add('type-declaration-after-mixed-return-break', 'do { if(a) return b; else break; } while(0); return a;',
  'do { if(a) return b; else break; type T=number; interface I {} } while(0); return a;');
add('type-declaration-after-mixed-return-continue', 'do { if(a) return b; else continue; } while(0); return a;',
  'do { if(a) return b; else continue; type T=number; interface I {} } while(0); return a;');
add('type-declaration-after-mixed-break-continue', 'for(let i=0;i<2;i++) { if(a) break; else continue; } return b;',
  'for(let i=0;i<2;i++) { if(a) break; else continue; type T=number; interface I {} } return b;');
add('type-declaration-after-both-returns', 'if(a) return a; else return b;',
  'if(a) return a; else return b; type T=number; interface I {}');
add('type-declaration-preserve-normal-alternative', 'if(a) return b; return a;', 'if(a) return b; type T=number; return a;');
add('type-declaration-depth-boundary', `${'{'.repeat(64)}return a;${'}'.repeat(64)}`,
  `${'{'.repeat(64)}return a; type T=number;${'}'.repeat(64)}`);
const aliases = (count, exported = '') => Array.from({length:count},(_,i)=>`${exported}type T${i}=number;`).join('');
add('type-declaration-module-budget', 'return a;', 'return a;', aliases(255));
add('type-declaration-export-budget', 'return a;', 'return a;', aliases(255, 'export '));
add('type-declaration-body-budget', 'return a;', 'return a;' + aliases(255));
add('type-declaration-shared-directive-budget', 'return a;', '"body hint";' + aliases(126) + 'return a;',
  '"module hint";' + aliases(127));
add('type-declaration-default-wrapper-budget', ';'.repeat(254) + 'return a;', ';'.repeat(254) + 'return a;',
  'export default interface T {}');
const locals = Array.from({length:128},(_,i)=>`x${i}=a`).join(',');
add('type-declaration-members-are-not-locals', `const ${locals}; return a;`, `const ${locals}; return a;`,
  `interface T { ${Array.from({length:129},(_,i)=>`p${i}:number;`).join('')} }`);
function byteLimitSource(extra = 0) {
  const prefix = 'type T={ "λ';
  const suffix = '":number }; export function run(a,b) { return a; }';
  return prefix + 'x'.repeat(16_384 + extra - Buffer.byteLength(prefix + suffix)) + suffix;
}
full('type-declaration-original-byte-limit', 'export function run(a,b) { return a; }', byteLimitSource());

const run = 'export function run(a,b) { return a; }';
const source = body => `export function run(a,b) { ${body} }`;
const top = 'exactly one exported function is admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const abrupt = 'statements after a guaranteed';
export const typeDeclarationRejected = [
  ...[
    ['import', 'import type { T } from "missing";'], ['export-list', 'type T=number; export type { T };'],
    ['re-export', 'export type { T } from "missing";'], ['export-star', 'export type * from "missing";'],
    ['ambient-value', 'declare const x:number;'], ['ambient-function', 'declare function other():number;'],
    ['namespace', 'namespace N { export type T=number; }'], ['ambient-namespace', 'declare namespace N { interface T {} }'],
    ['global-augmentation', 'declare global { interface T {} }'], ['module-augmentation', 'declare module "missing" { interface T {} }'],
    ['enum', 'enum E { A }'], ['const-enum', 'const enum E { A }'], ['ambient-enum', 'declare enum E { A }'],
    ['runtime-default', 'export default 1;'], ['runtime-export', 'export const x=1;'],
  ].map(([id,prefix]) => [`type-declaration-excluded-${id}`, prefix + run, 'ts', top]),
  ['type-declaration-only-types', 'type T=number; export interface I {}', 'ts', top],
  ['type-declaration-bare-entry', 'type T=number; function run(a,b) { return a; }', 'ts', "expected 'export function"],
  ['type-declaration-type-not-value', source('type Only=number; return Only;'), 'ts', reference],
  ['type-declaration-type-tdz', source('type x=number; const x=x; return x;'), 'ts', reference],
  ['type-declaration-intrinsic-tdz', source('type Infinity=number; const Infinity=Infinity; return Infinity;'), 'ts', reference],
  ...[
    ['return', 'return a; type T=number; a=b;'],
    ['break', 'do { break; interface I {} a=b; } while(0); return a;'],
    ['continue', 'for(let i=0;i<1;i++) { continue; type T=number; a=b; } return a;'],
    ['mixed', 'do { if(a) return a; else break; type T=number; a=b; } while(0); return b;'],
    ['empty', 'return a; type T=number; ;'],
  ].map(([id,body]) => [`type-declaration-runtime-after-${id}`, source(body), 'ts', abrupt]),
  ['type-declaration-mixed-break-fallback', source('do { if(a) return a; else break; type T=number; } while(0);'), 'ts', 'every structural path'],
  ['type-declaration-mixed-continue-fallback', source('do { if(a) return a; else continue; interface I {} } while(0);'), 'ts', 'every structural path'],
  ['type-declaration-module-limit', aliases(256) + run, 'ts', 'at most 256 statements'],
  ['type-declaration-body-limit', source('return a;' + aliases(256)), 'ts', 'at most 256 statements'],
  ['type-declaration-export-limit', aliases(256, 'export ') + run, 'ts', 'at most 256 statements'],
  ['type-declaration-directive-limit', '"hint";' + aliases(255) + run, 'ts', 'at most 256 statements'],
  ['type-declaration-default-wrapper-limit', 'export default interface T {}' + source(';'.repeat(255) + 'return a;'), 'ts', 'at most 256 statements'],
  ['type-declaration-depth-limit', source(`${'{'.repeat(65)}type T=number;${'}'.repeat(65)}return a;`), 'ts', 'statement nesting exceeds'],
  ['type-declaration-byte-overflow', byteLimitSource(1), 'ts', '16 KiB'],
  ['type-declaration-malformed-alias', 'type T=;' + run, 'ts', 'parse diagnostics'],
  ['type-declaration-malformed-interface', 'interface T { x: }' + run, 'ts', 'parse diagnostics'],
  ['type-declaration-decorator', '@dec interface T {}' + run, 'ts', 'parse diagnostics'],
  ['type-declaration-duplicate-runtime-binding', 'type T=number;' + source('const x=a; const x=b; return x;'), 'ts', 'semantic diagnostics'],
  ['type-declaration-runtime-boolean', source('type T=number; return true;'), 'ts', 'expression is outside'],
  ['type-declaration-unselected-call', source('type T=number; if(false) Math.random(); return a;'), 'ts', 'expression is outside'],
];
