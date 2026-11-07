// Function values retain allocation identity, not merely code-site identity.
export const functionLooseEquality = [
  ['function-loose-equality-alias', 'const f=x=>x;const alias=f;return f==alias&&f!=(x=>x)?1:0;'],
  ['function-loose-equality-factory', 'const make=()=>x=>x;const first=make();const second=make();return first!=second&&first==first?1:0;'],
  ['function-loose-equality-cross-kind', 'const f=x=>x;const g=()=>a;return f!=g&&f!={}&&f!=[]?1:0;'],
  ['function-loose-equality-union', 'const f=x=>x;const value=a?f:{};return (value==f)==!!a?1:0;'],
  ['function-loose-equality-snapshot', 'let f=()=>a;const old=f;const same=f==(f=()=>b,old);return same&&f!=old?1:0;'],
].map(([id, body]) => [id, body, body]);
