// Pristine Math.fround: binary64 -> binary32 -> binary64, compared against
// independent native Node results (including signed zero and NaN classification).
const cases = [
  ['input', 'return Math.fround(a);'],
  ['missing', 'return Math.fround();'],
  ['zero', 'return Math.fround(-0);'],
  ['nan', 'return Math.fround(NaN);'],
  ['infinite', 'return Math.fround(Infinity);'],
  ['negative-infinite', 'return Math.fround(-Infinity);'],
  ['precision', 'return Math.fround(16777217);'],
  ['negative-precision', 'return Math.fround(-16777217);'],
  ['ties-even', 'return Math.fround(1.0000000596046448);'],
  ['tiny', 'return Math.fround(1e-46);'],
  ['tiny-negative', 'return Math.fround(-1e-46);'],
  ['subnormal', 'return Math.fround(1e-40);'],
  ['max-f32', 'return Math.fround(3.4028234663852886e38);'],
  ['overflow', 'return Math.fround(3.5e38);'],
  ['boolean', 'return Math.fround(true);'],
  ['null', 'return Math.fround(null);'],
  ['undefined', 'return Math.fround(undefined);'],
  ['nested', 'return Math.fround(Math.imul(a,b));'],
  ['effects', 'let n=0;return Math.fround((n=1,a),n=n*10+2)+n;'],
];
export const numericMathFround=cases.map(([id,body])=>[
  'math-fround-'+id,body,body,
]);
numericMathFround.push(['math-fround-ts-erasure','','',undefined,undefined,{
  sourceJS:'export function run(a,b){return Math.fround(a);}',
  sourceTS:'export function run(a:number,b:boolean):number{return Math.fround<never>(a);}',
}]);
