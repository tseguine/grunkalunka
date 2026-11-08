export const varPatternPromotion = {
  id: 'var-guard-pattern',
  source: "export function run(a,b){var {x}={x:a};return x;}",
  sha256: "bfb2edfdf1365476de247e2a05f4caec375373c0524483ac2f0c4a0e52f7b245",
};
export const varPatternPrograms = [[varPatternPromotion.id,'','',undefined,undefined,
  {sourceJS:varPatternPromotion.source,sourceTS:varPatternPromotion.source}]];
for(const[id,body]of[
 ['array','var [x,y]=[a,b];return +x+(+y);'],
 ['before','const before=x;var [x]=[a];return before===undefined?1:0;'],
 ['self','var [x=x]=[];return +x;'],
 ['default','var [x=3,y=x]=[];return +y;'],
 ['redeclare','const get=()=>x;var {x}={x:a};var {x}={x:b};return +get();'],
 ['object-rest','var {x,...rest}={x:a,y:b};return +rest.y;'],
 ['array-rest','var [x,...rest]=[a,b];return +rest[0];'],
 ['parameter','var [a]=[null];return +a;'],
 ['header','for(var [x]=[a];0;){}return +x;'],
])varPatternPrograms.push(['var-pattern-'+id,body,body]);
