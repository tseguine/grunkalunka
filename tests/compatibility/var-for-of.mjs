// Original rejected source is retained verbatim as a compiled numeric pair.
export const varForOfPromotion = {
  id: 'var-guard-for-of',
  source: 'export function run(a,b){for(var x of [a]){}return a;}',
  sha256: 'f0758a441b4b3dcc4a409905cdeaffdfaa02cbb83deb8ab5b61a7e100d166fad',
};
export const varForOfPrograms = [[varForOfPromotion.id, '', '', undefined, undefined,
  {sourceJS:varForOfPromotion.source,sourceTS:varForOfPromotion.source}]];
for (const [id, body] of [
  ['sum','let n=0;for(var item of [a,b])n+=+item;return n;'],
  ['before','const before=item;for(var item of []){}return before===undefined?1:0;'],
  ['empty','var item=3;for(var item of []){}return +item;'],
  ['bare','for(var item of [a,b]){}var item;return +item;'],
  ['parameter-capture','const get=()=>a;for(var a of [null,b]){}return +get();'],
  ['break','for(var item of [a,b])break;return +item;'],
  ['continue','let n=0;for(var item of [a,b]){if(item===a)continue;n++;}return n;'],
  ['string',"let n=0;for(var item of 'a\\u{1f600}b')n+=item==='a'?1:item==='b'?2:3;return n;"],
  ['shared','const f={a:()=>item,b:()=>item};let n=0;for(var item of [1,2]){if(n++===0)f.a=()=>item;else f.b=()=>item;}return +f.a()+(+f.b());'],
]) varForOfPrograms.push(['var-for-of-'+id,body,body]);
