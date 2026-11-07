// String relational operators compare unsigned UTF-16 code units, not Numbers.
export const stringRelational = [
  ['string-relational-prefix', "return ''<'a'&&'ab'<'abc'&&'abc'>'ab'?1:0;"],
  ['string-relational-equal', "return ('a'+'')<='a'&&'a'>='a'&&!('a'<'a')&&!('a'>'a')?1:0;"],
  ['string-relational-numeric-text', "return '2'>'10'?1:0;"],
  ['string-relational-utf16', String.raw`return '\u{10000}'<'\ue000'&&'\ud800'<'\udfff'&&'\0a'<'\0b'?1:0;`],
  ['string-relational-snapshot', "const o={s:'a'};const value=o.s<(o.s='z','b');return value&&o.s==='z'?1:0;"],
  ['string-relational-helper', "const less=(x,y)=>x<y;return less('a','b')?1:0;"],
].map(([id, body]) => [id, body, body]);
