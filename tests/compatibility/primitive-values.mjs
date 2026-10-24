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
];
const numericOrigins = {
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
  standards: [returns, types, conditional],
}));
const expression = 'expression is outside the numeric experiment';
export const valueRejected = [
  ['general-local', 'const x=null; return x;', expression],
  ['empty-return', 'return;', 'return must have a primitive expression'],
  ['implicit-return', 'if(a) return null;', 'every structural path must return a primitive value'],
  ['general-condition', 'if(null) return a; return b;', expression],
  ['nullish-operator', 'return null??b;', expression],
  ['coercion', 'return null+1;', expression],
  ['string', 'return "x";', expression],
  ['general-number-call', 'return run(0,b)+1;', 'value-returning calls require a General result position'],
].map(([id, body, diagnostic]) => ({
  id, sourceJS: wrap(body, false), sourceTS: wrap(body, true), diagnostic,
  standards: [returns, types, conditional],
}));
