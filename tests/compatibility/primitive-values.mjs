// Finite source evidence for the explicit primitive-return profile.
// No fixture here changes the numeric corpus or claims full language coverage.
const spec = 'https://tc39.es/ecma262/multipage/';
const returns = spec + 'ecmascript-language-statements-and-declarations.html#sec-return-statement';
const types = spec + 'ecmascript-data-types-and-values.html#sec-ecmascript-language-types';
const conditional = spec + 'ecmascript-language-expressions.html#sec-conditional-operator';
const wrap = (body, ts) => ts
  ? `export function run(a:number,b:number):unknown {${body}}\n`
  : `export function run(a,b) {${body}}\n`;
const rows = [
  ['null', 'return null;'],
  ['undefined', 'return undefined;'],
  ['false', 'return false;'],
  ['true', 'return true;'],
  ['number', 'return a;'],
  ['negative-zero', 'return -0;'],
  ['nan', 'return NaN;'],
  ['infinity', 'return Infinity;'],
  ['comparison', 'return a<b;'],
  ['predicate-local', 'const flag=a<b; return flag;'],
  ['mixed', 'return a?null:false;'],
  ['nested', 'return a?b:(b>0?true:undefined);'],
  ['sequence', 'return (b+=2,a?b:undefined);'],
  ['snapshot', 'return a?b+(b=2):(b=a+4);'],
  ['numeric-local', 'const n=a?b:0; return n;'],
  ['helper', 'return a?rem(a,b):undefined;'],
  ['entry-call', 'return a?run(0,b):null;'],
  ['type-wrapper', 'return a?null:b;'],
  ['boolean-choice', 'return a?true:false;'],
  ['general-local', 'const x=null; return x;'],
  ['general-condition', 'if(null) return a; return b;'],
  ['logical-and','return a&&(b<1);'],
  ['logical-or','return (a<1)||b;'],
  ['logical-number-or-boolean','return a||true;'],
  ['logical-false-and-number','return false&&a;'],
  ['mutable-late-kind','let value=a; value=null; return value;'],
  ['mutable-boolean','let value=true; value=a?false:true; return value;'],
  ['mutable-alias','let first=a,second=b; first=second; second=null; return first;'],
  ['mutable-assignment','let left=null,right=false; return left=(right=a?true:b);'],
  ['coercion','return null+1;'],
  ['tonumber-nullish','const value=a?undefined:null; return +value;'],
  ['tonumber-boolean','let value=null; value=a?true:false; return value*b;'],
  ['tonumber-snapshot','let value=null; value=a; return value-(value=b);'],
  ['tonumber-recursive-result','return a?null:run(1,b)+1;'],
];
const numericOrigins = {
  "logical-and": ["logical-unsupported-4", "export function run(a,b) { return a && (b<1); }"],
  "logical-or": ["logical-unsupported-5", "export function run(a,b) { return (a<1) || b; }"],
  "logical-number-or-boolean": ["logical-unsupported-6", "export function run(a,b) { return a || true; }"],
  "logical-false-and-number": ["logical-unsupported-7", "export function run(a,b) { return false && a; }"],
  true: ['boolean', 'export function run(a,b) { return true; }'],
  null: ['null', 'export function run(a,b) { return null; }'],
  comparison: ['comparison', 'export function run(a,b) { return a < b; }'],
  'boolean-choice': ['predicate-conditional-numeric-return', 'export function run(a,b) { return a?true:false; }'],
};
export const valuePrograms = rows.map(([id, body]) => ({
  id,
  numericOrigin: numericOrigins[id]?.[0] ?? null,
  sourceJS: numericOrigins[id]?.[1] ?? (id === 'helper' ? 'function rem(x,y){return x%y;}\n' : '') + wrap(body, false),
  sourceTS: (id === 'helper' ? 'function rem<T>(x:number,y:number):number{return x%y;}\n' : '')
    + wrap(id === 'type-wrapper' ? 'return ((a?null:b) as unknown)!;' : body, true),
  standards: (id.startsWith('tonumber-') || id === 'coercion') ? [returns, types, conditional, spec + 'abstract-operations.html#sec-tonumber', spec + 'ecmascript-language-expressions.html#sec-applystringornumericbinaryoperator'] : id.startsWith('mutable-') ? [returns, types, conditional, spec + 'ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations', spec + 'ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation'] : id.startsWith('logical-') ? [returns, types, conditional, spec + 'ecmascript-language-expressions.html#sec-binary-logical-operators-runtime-semantics-evaluation'] : id === 'general-condition' ? [returns, types, conditional, spec + 'abstract-operations.html#sec-toboolean'] : id === 'general-local' ? [returns, types, conditional, spec + 'ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations'] : [returns, types, conditional],
}));
const expression = 'expression is outside the numeric experiment';
export const valueRejected = [
  ['empty-return', 'return;', 'return must have a primitive expression'],
  ['implicit-return', 'if(a) return null;', 'every structural path must return a primitive value'],
  ['nullish-operator', 'return null??b;', expression],
  ['string', 'return "x";', expression],
].map(([id, body, diagnostic]) => ({
  id, sourceJS: wrap(body, false), sourceTS: wrap(body, true), diagnostic,
  standards: [returns, types, conditional],
}));

// Exact former admission rejection; unconditional recursion has no JS-result oracle.
// Its generated module is checked separately for bounded fuel exhaustion.
export const valueFuelCases = [{
  id: 'general-number-call',
  sourceJS: wrap('return run(0,b)+1;', false),
  sourceTS: wrap('return run(0,b)+1;', true),
  standards: [returns, types, conditional, spec + 'abstract-operations.html#sec-tonumber'],
}];
