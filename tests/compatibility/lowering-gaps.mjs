// Valid reference programs outside the current numeric admission.
// Their rejection checks must never count as executed lowering support.
const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const unary = 'only unary +, - and ~ are admitted';
const statement = 'statement is outside the numeric experiment';
const reference = 'reference is not an initialized numeric parameter or local';
const topLevel = 'exactly one exported function is admitted';
const declaration = 'initialized const or let identifiers are required';
const ordinaryParameters = 'exactly two ordinary numeric parameters are admitted';
const plainFunction = 'expected a plain, synchronous, non-generic function named run';

export const loweringGaps = [];
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
  ['power', 'return a ** b;', 16, binary],
  ['loose-equality-coercion', 'return a == "4" ? 1 : 0;', 1, expression],
  ['nullish', 'return (a === 4 ? null : a) ?? b;', 2, expression],
  ['typeof', 'return typeof a === "number" ? 1 : 0;', 1, unary],
  ['void', 'return void a === undefined ? 1 : 0;', 1, unary],
  ['delete', 'return delete a.x ? 1 : 0;', 1, unary],
  ['property-in', 'return "x" in {x:a} ? 1 : 0;', 1, binary],
  ['instanceof', 'return [] instanceof Array ? 1 : 0;', 1, binary],
]) add(id, 'operators', body, expected, diagnostic);

for (const [id, body, expected, diagnostic] of [
  ['boolean-value', 'const less = a < b; return less ? 1 : 0;', 0, binary],
  ['string-coercion', 'return a + "2" === "42" ? 1 : 0;', 1, expression],
  ['object-property', 'return ({x:a}).x + b;', 6, expression],
  ['array-value', 'const items = [a,b]; return items[1];', 2, expression],
  ['bigint', 'return 1n === 1n ? a : b;', 4, expression],
  ['null', 'return null === null ? a : b;', 4, expression],
  ['undefined', 'return undefined === undefined ? a : b;', 4, reference],
]) add(id, 'values', body, expected, diagnostic);

for (const [id, body, expected] of [
  ['switch', 'switch(a) { case 4: return b; default: return a; }', 2],
  ['label', 'outer: while(1) { a=a+b; break outer; } return a;', 6],
]) add(id, 'control-flow', body, expected, statement, { milestone: 'L2' });

add('call', 'functions', 'return (x => x + b)(a);', 6, expression, { milestone: 'L3', prerequisites: ['arrow function values'] });
add('nested-function', 'functions', 'function inc(x) { return x+1; } return inc(a)+b;', 7, statement, { milestone: 'L3', prerequisites: ['function calls'] });
add('recursion', 'functions', 'function fact(n) { return n<=1 ? 1 : n*fact(n-1); } return fact(a);', 24, statement,
  { milestone: 'L3', prerequisites: ['nested function declarations', 'calls'] });
add('module-import', 'modules', '', 7, topLevel, { milestone: 'L3',
  source: 'import {inc} from "data:text/javascript,export function inc(x){return x+1}"; export function run(a,b) { return inc(a)+b; }',
  prerequisites: ['multiple top-level statements', 'function calls'] });

add('var', 'bindings', 'var x=a; return x+b;', 6, declaration);
add('var-hoisting', 'bindings', 'const before = x === undefined ? 1 : 0; var x=a; return before;', 1, reference,
  { prerequisites: ['undefined values', 'var bindings'] });
add('uninitialized-let', 'bindings', 'let x; x=a; return x;', 4, 'numeric local must have an initializer');
add('local-destructure', 'bindings', 'const {x}={x:a}; return x+b;', 6, 'local destructuring is not admitted', { prerequisites: ['object values'] });
add('destructuring-write', 'bindings', 'let x=a; [x]=[b]; return x;', 2, 'assignment target must be an initialized identifier', { prerequisites: ['array values'] });
add('defaults', 'bindings', '', 6, 'parameter modifiers, defaults and decorators are not admitted',
  { source: 'export function run(a=4,b=2) { return a+b; }', args: [] });
add('rest', 'bindings', '', 6, ordinaryParameters,
  { source: 'export function run(a,...rest) { return a+rest[0]; }', prerequisites: ['array values'] });

add('closure-capture', 'closures', 'const read=()=>a; a=b; return read();', 2, expression,
  { milestone: 'L5', prerequisites: ['function values', 'calls'] });
add('catch', 'exceptions', 'try { throw a; } catch (e) { return e+b; }', 6, statement, { milestone: 'L5' });
add('finally-return', 'exceptions', 'try { return a; } finally { return b; }', 2, statement, { milestone: 'L5' });
add('class-instance', 'classes', 'class Box { constructor(x) { this.x=x; } get() { return this.x; } } const box=new Box(a); return box.get()+b;', 6, statement,
  { prerequisites: ['objects', 'construction', 'method calls'] });
add('generator', 'generators-async', 'function* values() { yield a; return b; } const it=values(); return it.next().value+it.next().value;', 6, statement,
  { milestone: 'L5', prerequisites: ['nested functions', 'iterator objects', 'calls'] });
add('async-await', 'generators-async', '', 6, plainFunction, { milestone: 'L5',
  source: 'export async function run(a,b) { return a + await Promise.resolve(b); }', prerequisites: ['Promise built-in', 'calls'] });

add('export-name', 'input-shapes', '', 6, plainFunction,
  { source: 'export function sum(a,b) { return a+b; }', entry: 'sum', milestone: 'L3' });
add('one-parameter', 'input-shapes', '', 4, ordinaryParameters,
  { source: 'export function run(a) { return a; }', milestone: 'L3' });
add('extra-export', 'input-shapes', '', 6, topLevel,
  { source: 'export const n=2; export function run(a,b) { return a+n; }', milestone: 'L3' });
add('while-guaranteed-return', 'structural-analysis', 'while(1) { return a; }', 4, 'every structural path must return a numeric value', { milestone: 'L2' });
add('unreachable-after-return', 'structural-analysis', 'return a; a=b;', 4, 'statements after a guaranteed return are not admitted', { milestone: 'L2' });
