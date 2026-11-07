// String data references save the receiver and old value before the RHS.
export const stringPropertyAddition = [
  ['string-property-addition-value', "const o={s:'x'};return (o.s+='y').length;"],
  ['string-property-addition-array', "const xs=['x'];const value=xs[0]+='y';return value==='xy'&&xs[0]===value&&xs.length===1?1:0;"],
  ['string-property-addition-reference', "let o={s:'old'};const alias=o;const value=o.s+=(o={s:'other'},'new');return alias.s==='oldnew'&&o.s==='other'&&value===alias.s?1:0;"],
  ['string-property-addition-snapshot', "const o={s:'old'};const value=o.s+=(o.s='changed','new');return value==='oldnew'&&o.s===value?1:0;"],
  ['string-property-addition-order', "let n=0;const o={s:'x'};const value=(n=n+1,o).s+=(n=n+10,'y');return n===11&&value==='xy'&&o.s===value?1:0;"],
  ['string-property-addition-loop', "const o={s:''};for(const value of ['a','b'])o.s+=value;const read=()=>o.s;return read()==='ab'?1:0;"],
].map(([id, body]) => [id, body, body]);
