export const varLoopPatternPromotion={id:'var-guard-iteration-pattern',source:"export function run(a,b){for(var [x] of [[a]]){}return a;}",sha256:"764cd5824ece2d7265a1e85d7cbf5c2a1641efc086b9e1305e067b46ccbd8008"};
export const varLoopPatternPrograms=[[varLoopPatternPromotion.id,'','',undefined,undefined,{sourceJS:varLoopPatternPromotion.source,sourceTS:varLoopPatternPromotion.source}]];
for(const[id,body]of[
 ['sum','let n=0;for(var [x] of [[a],[b]])n+=+x;return n;'],
 ['object','for(var {x} of [{x:a},{x:b}]){}return +x;'],
 ['nested','for(var {row:[x]} of [{row:[a]}]){}return +x;'],
 ['default','for(var [x=3,y=x] of [[]]){}return +y;'],
 ['before','const before=x;for(var [x] of (a?[]:[[b]])){}return before===undefined?1:0;'],
 ['empty','var x=3;for(var [x] of (a?[]:[[b]])){}return +x;'],
 ['capture','const get=()=>x;for(var [x] of [[a],[b]]){}return +get();'],
 ['break','for(var [x] of [[a],[b]])break;return +x;'],
 ['continue','let n=0;for(var [x] of [[a],[b]]){if(x===a)continue;n++;}return n;'],
])varLoopPatternPrograms.push(['var-loop-pattern-'+id,body,body]);
