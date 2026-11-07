// String-only += preserves the old operand before RHS effects and stores once.
export const stringLocalAddition = [
  ['string-local-addition-former-guard', "let text='x';text+='y';return text.length;"],
  ['string-local-addition-pattern', "let [text]=['x'];return (text+='y').length;"],
  ['string-local-addition-snapshot', "let text='old';const value=text+=(text='changed','new');return text==='oldnew'&&value===text?1:0;"],
  ['string-local-addition-capture', "let text='x';const read=()=>text;text+='y';return read()==='xy'?1:0;"],
  ['string-local-addition-closure', "let text='x';const add=()=>text+='y';return add()==='xy'&&text==='xy'?1:0;"],
  ['string-local-addition-iteration', "let text='';for(let value of ['a','b'])text+=value;return text==='ab'?1:0;"],
].map(([id, body]) => [id, body, body]);
