export const stringIteratorPromotions = [
  ['var-guard-in-pattern', 'export function run(a,b){for(var [x] in {x:a}){}return a;}'],
  ['string-array-first-original', "export function run(a,b){const [x]='abc';return a;}"],
  ['string-array-rest-original', "export function run(a,b){const [...rest]='abc';return a;}"],
  ['string-array-for-of-original', "export function run(a,b){for(var [x] of 'ab'){}return a;}"],
  ['string-array-rest-text-original', "export function run(a,b){const [...rest]='text';return a;}"],
  ['string-array-lexical-loop-original', "export function run(a,b){for(const [first] of 'abc'){}return a;}"],
];
export const stringIteratorPrograms = stringIteratorPromotions.map(([id, source]) =>
  [id, '', '', undefined, undefined, {sourceJS: source, sourceTS: source}]);
for (const [id, body] of [
  ['unicode', "const [x='?',y='?',z='?',...rest]='\\u{1f600}\\ud800a\\udc00';return x.length*1000+y.charCodeAt(0)+z.charCodeAt(0)+rest.length;"],
  ['elision', "const [,x='?',...rest]='\\u{1f600}ab';return x.charCodeAt(0)+rest.length;"],
  ['default', "let n=0;const [x=(n++,'?'),y=(n++,'!'),...tail]='a';return n*100+x.charCodeAt(0)+y.charCodeAt(0)+tail.length;"],
  ['snapshot', "let source='';const [x=(source='abc','?'),y='!',...tail]=source;return x.length+y.charCodeAt(0)+tail.length;"],
  ['nested', "const {s:[x='?',...tail]}={s:'abc'};return x.charCodeAt(0)+tail.length;"],
  ['nested-default', "const [[x='?']='z']='a';return x.charCodeAt(0);"],
  ['fresh-rest', "const [...a1]='ab';const [...a2]='ab';a1[0]='z';return a1!==a2&&a2[0]==='a'?1:0;"],
  ['capture', "const get=()=>x;var [x='?']='abc';return get()==='a'?1:0;"],
  ['for-of-array', "let n=0;for(const [x='?',...tail] of ['abc',''])n+=x.charCodeAt(0)+tail.length;return n;"],
  ['for-of-string', "let n=0;for(const {length:l} of '\\u{1f600}a')n=n*10+l;return n;"],
  ['for-in', "let n=0;for(const [x='?'] in {'':a,abc:b})n+=x.charCodeAt(0);return n;"],
  ['for-in-capture', "const get=()=>x;for(var [x='?'] in {abc:a,def:b}){}return get()==='d'?1:0;"],
]) stringIteratorPrograms.push(['string-iterator-' + id, body, body]);
