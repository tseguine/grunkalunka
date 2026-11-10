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

for (const [id, body] of [
  ['power-assignment-param', 'return a**=2;'],
  ['power-assignment-zero', 'return a**=-0;'],
  ['power-assignment-slot', 'let x=a;const r=(x**=-1);return r+x;'],
  ['power-assignment-closure', 'let x=a;const square=()=>x**=2;const r=square();return r+x;'],
  ['power-assignment-array-key', 'const xs=[a,b];let i=0;const r=(xs[i++]**=2);return +xs[0]+r+i;'],
  ['power-assignment-property', 'const o={x:a};const r=(o.x**=-1);return +o.x+r;'],
  ['power-assignment-primitive', 'let x=a?null:true;const r=(x**=2);return +x+r;'],
]) {
  literalPowers.push([id, body, body]);
}
literalPowers.push(['power-assignment-erased-target', 'return a**=2;',
  'return (a as number)**=(2 satisfies number);']);
