// Two proven Strings compare by contents for abstract and strict equality alike.
export const stringLooseEquality = [
  ['string-loose-equality-property', "const o={x:'text'};return o.x=='text'?1:0;"],
  ['string-loose-equality-content', "return ''==''&&'a'!='b'&&('x'+'')=='x'?1:0;"],
  ['string-loose-equality-snapshot', "let text='old';const value=text==(text='new','old');return value&&text==='new'?1:0;"],
  ['string-loose-equality-iteration', "let n=0;for(const text of ['a','b']){if(text=='a')n++;if(text!='b')n+=10;}return n;"],
].map(([id, body]) => [id, body, body]);
