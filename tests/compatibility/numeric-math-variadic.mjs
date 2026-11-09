// Pristine numeric Math.min/max for admitted primitive ToNumber operands.
// Every expected value comes from the independent native Node oracle.
const cases = [
  ['min-empty','return Math.min();'],
  ['max-empty','return Math.max();'],
  ['min-pair','return Math.min(a,b);'],
  ['max-pair','return Math.max(a,b);'],
  ['min-single','return Math.min(a);'],
  ['max-single','return Math.max(b);'],
  ['min-negative-zero','return Math.min(+0,-0);'],
  ['max-negative-zero','return Math.max(+0,-0);'],
  ['min-reversed-zero','return Math.min(-0,+0);'],
  ['max-reversed-zero','return Math.max(-0,+0);'],
  ['min-nan','return Math.min(a,NaN,b);'],
  ['max-nan','return Math.max(NaN,a);'],
  ['min-mixed','return Math.min(null,a,true,b);'],
  ['max-mixed','return Math.max(false,a,undefined);'],
  ['nested','return Math.max(Math.min(a,b),-5);'],
  ['effects','let n=0;const x=Math.min((n=1,a),(n=n*10+2,b),n=n*10+3);return x+n;'],
];
export const numericMathVariadic = cases.map(([id, body]) => [
  'math-variadic-'+id, body, body,
]);
numericMathVariadic.push([
  'math-variadic-ts-erasure', '', '', undefined, undefined, {
    sourceJS: 'export function run(a,b){return Math.max(a,b);}',
    sourceTS: 'export function run(a:number,b:boolean):number{return Math.max<never>(a,b);}',
  },
]);
