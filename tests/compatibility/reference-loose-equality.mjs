// Known object/array pairs use identity without primitive conversion.
export const referenceLooseEquality = [
  ['reference-loose-equality-factory', 'const make=()=>({});const first=make();const second=make();return first!=second&&first==first?1:0;'],
  ['reference-loose-equality-object', 'const o={};const alias=o;return o==alias&&o!={}?1:0;'],
  ['reference-loose-equality-array', 'const xs=[];const alias=xs;return xs==alias&&xs!=[]?1:0;'],
  ['reference-loose-equality-kinds', 'return {}!=[]&&[]!={}?1:0;'],
  ['reference-loose-equality-snapshot', 'let o={};const old=o;const value=o==(o={},old);return value&&o!=old?1:0;'],
].map(([id, body]) => [id, body, body]);
