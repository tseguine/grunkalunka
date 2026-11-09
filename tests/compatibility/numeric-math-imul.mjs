// Pristine Math.imul: two ToInt32 operands with wrapping i32 multiplication.
// Native Node independently supplies each expected binary64 result.
const cases = [
  ['inputs','return Math.imul(a,b);'],
  ['empty','return Math.imul();'],
  ['one','return Math.imul(a);'],
  ['signed','return Math.imul(-1,2);'],
  ['max-positive','return Math.imul(2147483647,2);'],
  ['max-unsigned','return Math.imul(4294967295,5);'],
  ['overflow','return Math.imul(2147483648,2);'],
  ['wrap-source','return Math.imul(4294967297,3);'],
  ['fraction','return Math.imul(3.9,2.2);'],
  ['negative-fraction','return Math.imul(-3.9,2.2);'],
  ['nonfinite','return Math.imul(NaN,Infinity);'],
  ['boolean','return Math.imul(true,b);'],
  ['null','return Math.imul(null,b);'],
  ['undefined','return Math.imul(undefined,b);'],
  ['zero','return Math.imul(-0,b);'],
  ['nested','return Math.imul(Math.min(a,b),Math.max(a,b));'],
  ['effects','let n=0;const p=Math.imul((n=1,a),(n=n*10+2,b),n=n*10+3);return p+n;'],
];
export const numericMathImul=cases.map(([id,body])=>[
  'math-imul-'+id,body,body,
]);
numericMathImul.push(['math-imul-ts-erasure','','',undefined,undefined,{
  sourceJS:'export function run(a,b){return Math.imul(a,b);}',
  sourceTS:'export function run(a:number,b:boolean):number{return Math.imul<never>(a,b);}',
}]);
