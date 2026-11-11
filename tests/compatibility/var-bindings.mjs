// Exact former numeric guards, now executed with function-scoped var cells.
export const varPromotions = [
  ['gap-var', 'export function run(a,b) { var x=a; return x+b; }', '1cb57d3c74e3fb1b2b0314edb65c84f07971621077d965fa3b2f1a0cb63f1485'],
  ['gap-var-hoisting', 'export function run(a,b) { const before = x === undefined ? 1 : 0; var x=a; return before; }', 'ac1fe3ad1d6d4dd5c28604560eafd117cadd8dd3c17430ca0f1c9172cb23abae'],
  ['for-var', 'export function run(a,b) { for(var i=0;0;i++) {} return a; }', 'a6620ba9393f1dca83875cf50d0b77505c345a480fe4e010dd3b39cb0580918f'],
  ['var-local', 'export function run(a,b) { var x=a; return x; }', 'be922e9aeb7a9bad2063d8daef59581ace5be470049563dc3a74db5cd3efffa3'],
  ['var-unreachable-hoisting', 'export function run(a,b){return x;var x=3;}', '32dfb7a4d7a9c9994371a96efc09eeb9eb8c85ffbd50c5bf5f0acbfa9726c0ee'],
  ['var-unreachable-initializer', 'export function run(a,b){return a;var x=b;return x;}', '823990be04ccf6f5e8891aa47d40357c0e18d0b5e008cf7c9d8cabd097aca5a7'],
];
export const varValuePromotion = varPromotions.find(([id])=>id==='var-unreachable-hoisting');
export const varPrograms = varPromotions.filter(row=>row!==varValuePromotion).map(([id,source])=>[id,'','',undefined,undefined,{sourceJS:source,sourceTS:source}]);
for (const [id,body] of [
  ['undefined','var x;return +x;'],
  ['before-read','const before=x===undefined?1:0;var x=a;return before;'],
  ['bare-redeclaration','var x=a;var x;return x;'],
  ['initialized-redeclaration','var x=a;var x=b;return x;'],
  ['assignment-before-declaration','x=a;var x;return x;'],
  ['snapshot','var x=a;const before=x;x=b;var x;return before;'],
  ['counter','for(var i=0;i<3;i++){}return i;'],
  ['null-update','var x=null;var old=x++;return +old;'],
  ['boolean-update','var x=false;return ++x;'],
  ['string','var x="abc";var x;return x.length;'],
  ['object-alias','var x={v:a};var y=x;y.v=b;return x.v;'],
  ['block-scope','var x=a;{var x=b;}return x;'],
  ['lexical-shadow','var x=a;{let x=b;}return x;'],
  ['parameter-alias','var a;return a;'],
]) varPrograms.push(['var-'+id,body,body]);
export const varRejected = [
  ['function-init','var f=x=>x;return f(a);','function-valued var initializers'],
  ['catch-name','try{throw a;}catch(e){var e=3;}return 0;','var cannot redeclare a catch binding'],
].map(([id,body,diagnostic])=>['var-guard-'+id,`export function run(a,b){${body}}`,'js',diagnostic]);
