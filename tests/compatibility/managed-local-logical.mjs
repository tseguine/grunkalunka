// Logical writes preserve selected mixed values in boxed lexical cells.
export const managedLocalLogical = [
  ['managed-local-logical-former-guard', "let x='';x||='fallback';return a;"],
  ['managed-local-logical-nullish', "let [x]=[null];return (x??='ok')==='ok'?1:0;"],
  ['managed-local-logical-or', "let {x}={x:''};return (x||=a)===a?1:0;"],
  ['managed-local-logical-and-skip', "let [x]=[false];return (x&&='skip')===false?1:0;"],
  ['managed-local-logical-identity', 'const other={n:a};let [x]=[null];return (x??=other)===other?1:0;'],
  ['managed-local-logical-capture', "let [x]=[null];const read=()=>x;const value=x??=(x=false,'new');return read()===value?1:0;"],
  ['managed-local-logical-snapshot', "let [x]=['old'];let n=0;const value=x||=(n++,'new');x='later';return value==='old'&&n===0?1:0;"],
].map(([id, body]) => [id, body, body]);
