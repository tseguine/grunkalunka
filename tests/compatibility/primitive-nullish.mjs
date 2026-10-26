// Binary ?? retains its left value unless it is Null or Undefined.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-binary-logical-operators-runtime-semantics-evaluation
export const primitiveNullishPromotions = [
  ['discarded-nullish', 'export function run(a,b) { a??true; return a; }', '8579a29f43d4555ffafe768dfed4a60cf9affe0d36d4e4092c8541d1b7814de4'],
  ['predicate-logical-nullish', 'export function run(a,b) { if(a ?? b) return a; return b; }', '2516615f116e201c581665e2e9e48d0409cbe91889d9f82979221a907cd6c764'],
  ['logical-unsupported-8', 'export function run(a,b) { return a ?? b; }', '2d29afbcb3f53e9fecc4dfe4d06880b0af936d608feefbc8da26e835d131db80'],
];
export const primitiveNullish = primitiveNullishPromotions.map(([id, sourceJS]) => [id, '', '', undefined, undefined, {
  sourceJS, sourceTS: sourceJS.replace('run(a,b)', 'run(a:number,b:number)'), oracleSource: sourceJS,
}]);
for (const [id, body] of [
  ['numeric-effects', 'return (b++,a)??(b=a);'],
  ['numeric-accumulator', 'let sum=a;for(let i=0;i<3;i++)sum=sum??b;return sum;'],
  ['general-condition', 'if((a?null:false)??b)return a;return b;'],
  ['zero-condition', 'if(0??b)return a;return b;'],
  ['general-conversion', 'const value=(a?null:b)??a;return +value;'],
  ['discard-effects', 'let count=0;(count++,a?undefined:a)??count++;return count;'],
  ['general-snapshot', 'let value=a?null:false;const old=value??(value=b);value=null;return old===false?a:b;'],
  ['wrapper', 'return (a??b)+b;'],
]) primitiveNullish.push([`primitive-nullish-${id}`, body, body]);
