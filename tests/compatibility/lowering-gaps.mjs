// Valid reference programs outside the current numeric admission.
// Their rejection checks must never count as executed lowering support.
const exponent = 'exponentiation requires a literal exponent of -1, 0, 1 or 2';
const unary = 'only unary +, - and ~ are admitted';
const statement = 'statement is outside the numeric experiment';
const reference = 'reference is not an initialized numeric parameter or local';
const topLevel = 'exactly one exported function is admitted';
const declaration = 'const or let local declarations are required';
const ordinaryParameters = 'at most two ordinary numeric parameters are admitted';
const plainFunction = 'expected a plain, synchronous function named run';

export const loweringGaps = [];
export const reducedEntryArityPromotion = {
  id: 'gap-one-parameter',
  source: 'export function run(a) { return a; }',
  sha256: 'd9c36f6ec2aa0c77801c770382cbc06d3246cc6a18110bab6d0fba2c7ab9afac',
};
export const numericArityPromotion = {
  id: 'arity',
  source: reducedEntryArityPromotion.source,
  sha256: reducedEntryArityPromotion.sha256,
};
export const reducedEntryArityPrograms = [
  ['gap-one-parameter', '', '', undefined, undefined, {
    sourceJS: reducedEntryArityPromotion.source,
    sourceTS: reducedEntryArityPromotion.source,
  }],
  ['arity', '', '', undefined, undefined, {
    sourceJS: numericArityPromotion.source,
    sourceTS: numericArityPromotion.source,
  }],
  ['entry-zero-parameters', '', '', undefined, undefined, {
    sourceJS: 'export function run(){return 4;}',
    sourceTS: 'export function run():number{return 4;}',
  }],
];
function add(id, category, body, expected, diagnostic, options = {}) {
  const source = options.source ?? `export function run(a,b) { ${body} }`;
  loweringGaps.push([`gap-${id}`, source, options.extension ?? 'js', diagnostic, {
    category, milestone: options.milestone ?? 'L4',
    oracleSource: options.oracleSource ?? source,
    args: options.args ?? [4, 2], entry: options.entry ?? 'run', expected,
    prerequisites: options.prerequisites ?? [],
  }]);
}

for (const [id, body, expected, diagnostic] of [
  ['power', 'return a ** b;', 16, exponent],
]) add(id, 'operators', body, expected, diagnostic);

export const bigintComparisonPrograms = [
  ['gap-bigint', 'export function run(a,b) { return 1n === 1n ? a : b; }'],
  ['bigint-large-neighbors', 'export function run(a,b){return 9007199254740993n > 9007199254740992n ? a : b;}'],
  ['bigint-radices', 'export function run(a,b){return 0xFF_FFn == 0b1111_1111_1111_1111n ? a : b;}'],
  ['bigint-negative', 'export function run(a,b){return -0o11n < -8n && -0n >= 0n ? a : b;}'],
  ['bigint-unequal', 'export function run(a,b){return 1n != 2n && 3n !== 4n && 5n <= 5n ? a : b;}'],
  ['bigint-typeof-original', "export function run(a,b){return typeof 1n==='bigint'?a:b;}"],
  ['bigint-typeof-negative', 'export function run(a,b){const kind=typeof (-0x20000000000001n);return kind.length+a;}'],
  ['bigint-truthy', 'export function run(a,b){if(-0n)return b;if(0x10000000000000000n)return a;return b;}'],
  ['bigint-not', 'export function run(a,b){return !0b0n && !(-0n) && !!(-42n) ? a : b;}'],
].map(([id, source]) => [id, '', '', undefined, undefined, {sourceJS: source, sourceTS: source}]);


add('nested-function', 'functions', 'function inc(x) { return x+1; } return inc(a)+b;', 7, statement, { milestone: 'L3', prerequisites: ['function calls'] });
add('recursion', 'functions', 'function fact(n) { return n<=1 ? 1 : n*fact(n-1); } return fact(a);', 24, statement,
  { milestone: 'L3', prerequisites: ['nested function declarations', 'calls'] });
add('module-import', 'modules', '', 7, topLevel, { milestone: 'L3',
  source: 'import {inc} from "data:text/javascript,export function inc(x){return x+1}"; export function run(a,b) { return inc(a)+b; }',
  prerequisites: ['multiple top-level statements', 'function calls'] });

add('defaults', 'bindings', '', 6, 'parameter modifiers, defaults and decorators are not admitted',
  { source: 'export function run(a=4,b=2) { return a+b; }', args: [] });
add('rest', 'bindings', '', 6, ordinaryParameters,
  { source: 'export function run(a,...rest) { return a+rest[0]; }', prerequisites: ['array values'] });

add('class-instance', 'classes', 'class Box { constructor(x) { this.x=x; } get() { return this.x; } } const box=new Box(a); return box.get()+b;', 6, statement,
  { prerequisites: ['objects', 'construction', 'method calls'] });
add('generator', 'generators-async', 'function* values() { yield a; return b; } const it=values(); return it.next().value+it.next().value;', 6, statement,
  { milestone: 'L5', prerequisites: ['nested functions', 'iterator objects', 'calls'] });
add('async-await', 'generators-async', '', 6, plainFunction, { milestone: 'L5',
  source: 'export async function run(a,b) { return a + await Promise.resolve(b); }', prerequisites: ['Promise built-in', 'calls'] });

add('export-name', 'input-shapes', '', 6, plainFunction,
  { source: 'export function sum(a,b) { return a+b; }', entry: 'sum', milestone: 'L3' });
add('extra-export', 'input-shapes', '', 6, topLevel,
  { source: 'export const n=2; export function run(a,b) { return a+n; }', milestone: 'L3' });
