// Retain the original source and provenance as an executed positive pair.
export const varForInPromotion = {
  id: 'var-guard-for-in',
  source: 'export function run(a,b){for(var x in {x:a}){}return a;}',
  sha256: 'cdd7ce8eabb12ca1bfc922e43389c909d743d29be475ee773b308e825e5b70ec',
};
export const varForInPrograms = [[varForInPromotion.id, '', '', undefined, undefined,
  {sourceJS:varForInPromotion.source,sourceTS:varForInPromotion.source}]];
for (const [id,body] of [
  ['order',"let n=0;for(var key in {b:a,'1':b,a:3,'0':a})n=n*10+(key==='0'?1:key==='1'?2:key==='b'?3:4);return n;"],
  ['before','const before=key;for(var key in {}){}return before===undefined?1:0;'],
  ['empty','var key=3;for(var key in {}){}return key===3?1:0;'],
  ['bare',"for(var key in {x:a,y:b}){}var key;return key==='y'?1:0;"],
  ['parameter-capture',"const get=()=>a;for(var a in {x:b}){}return get()==='x'?1:0;"],
  ['arrays',"let n=0;for(var key in [a,b])n+=key==='0'?1:2;return n;"],
  ['string',"let n=0;for(var key in 'ab')n+=key==='0'?1:2;return n;"],
  ['deleted',"const get=()=>key;const obj={x:a,y:b};for(var key in obj){delete obj.y;}return get()==='x'?1:0;"],
  ['break',"for(var key in {x:a,y:b})break;return key==='x'?1:0;"],
]) varForInPrograms.push(['var-for-in-'+id,body,body]);
