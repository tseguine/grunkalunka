const cases = [
  ['literal-power-zero', 'return a**0;'],
  ['literal-power-negative-zero', 'return a**-0;'],
  ['literal-power-one', 'return a**1;'],
  ['literal-power-square', 'return a**2;'],
  ['literal-power-reciprocal', 'return a**-1;'],
  ['literal-power-base-once', 'let x=b;const r=(x+=a)**2;return x+r;'],
  ['literal-power-zero-base-effect', 'let x=a;const r=(x=b)**0;return x+r;'],
  ['literal-power-primitive-base', "return (a?'2':null)**2;"],
  ['literal-power-helper', 'const f=x=>x**2;return f(a)+b;'],
];
export const literalPowers = cases.map(([id, body]) => [id, body, body]);
literalPowers.push(['literal-power-erased-exponent', 'return a**(-1);', 'return a**(-(1 as number));']);

for (const [id, source] of [
  ['power-default-original-constant', 'function f(x=1**2){return x;}export function run(a,b){return f();}'],
  ['power-default-original-y', 'function f(x,y=x**2){return y;}export function run(a,b){return f(a);}'],
  ['power-default-original-z', 'function f(x,z=x**2){return z;}export function run(a,b){return f(a);}'],
  ['power-default-supplied', 'function f(x,y=x**-1){return y;}export function run(a,b){return +f(a,b);}'],
  ['power-default-closure', 'export function run(a,b){const f=(x,y=x**-1)=>y;return +f(a);}'],
  ['power-default-zero', 'function f(x=NaN**0){return x;}export function run(a,b){return f();}'],
]) {
  literalPowers.push([id, '', '', undefined, undefined, { sourceJS: source, sourceTS: source }]);
}
