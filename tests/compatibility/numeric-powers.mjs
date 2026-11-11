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
