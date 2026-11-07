// Positive Number pairs need no coercion; NaN and signed zero retain JS semantics.
export const numberLooseEquality = [
  ['number-loose-equality-field', 'const o={n:a};return o.n==b?1:0;'],
  ['number-loose-equality-array', 'const xs=[a,b];return xs[0]!=xs[1]?1:0;'],
  ['number-loose-equality-nan', 'const o={n:NaN};return o.n!=o.n&&!(o.n==o.n)?1:0;'],
  ['number-loose-equality-zero', 'const o={n:-0};return o.n==0&&0==o.n&&!(o.n!=0)?1:0;'],
  ['number-loose-equality-snapshot', 'const o={n:a};const value=o.n==(o.n=b,a);return value==(a==a)?1:0;'],
].map(([id, body]) => [id, body, body]);
