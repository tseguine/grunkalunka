// Pristine Math.clz32: JavaScript ToUint32 bit pattern, Wasm i32.clz.
// Node is the independent oracle for exact Number outputs.
const cases = [
  ['input', 'return Math.clz32(a);'],
  ['missing', 'return Math.clz32();'],
  ['zero', 'return Math.clz32(0);'],
  ['negative-zero', 'return Math.clz32(-0);'],
  ['one', 'return Math.clz32(1);'],
  ['two', 'return Math.clz32(2);'],
  ['highest-bit', 'return Math.clz32(0x80000000);'],
  ['highest-minus-one', 'return Math.clz32(0x7fffffff);'],
  ['negative-one', 'return Math.clz32(-1);'],
  ['wrapped-zero', 'return Math.clz32(4294967296);'],
  ['wrapped-one', 'return Math.clz32(4294967297);'],
  ['nan', 'return Math.clz32(NaN);'],
  ['infinity', 'return Math.clz32(Infinity);'],
  ['fraction', 'return Math.clz32(3.9);'],
  ['negative-fraction', 'return Math.clz32(-3.9);'],
  ['boolean', 'return Math.clz32(true);'],
  ['null', 'return Math.clz32(null);'],
  ['undefined', 'return Math.clz32(undefined);'],
  ['nested', 'return Math.clz32(Math.imul(a,b));'],
  ['effects', 'let n=0;return Math.clz32((n=1,a),n=n*10+2)+n;'],
];
export const numericMathClz32=cases.map(([id,body])=>[
  'math-clz32-'+id,body,body,
]);
numericMathClz32.push(['math-clz32-ts-erasure','','',undefined,undefined,{
  sourceJS:'export function run(a,b){return Math.clz32(a);}',
  sourceTS:'export function run(a:number,b:boolean):number{return Math.clz32<never>(a);}',
}]);
