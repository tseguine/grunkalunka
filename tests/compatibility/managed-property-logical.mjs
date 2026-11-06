// Static logical property writes preserve selected values and saved receivers.
export const managedPropertyLogical = [
  ['managed-property-logical-former-missing-guard', 'const o={};return o.n||=b;'],
  ['managed-property-logical-nullish', "const o={x:null};return (o.x??='ok')==='ok'?1:0;"],
  ['managed-property-logical-missing', 'const o={};return (o.x??=a)===a?1:0;'],
  ['managed-property-logical-and-skip', "const xs=[false];return (xs[0]&&='skip')===false?1:0;"],
  ['managed-property-logical-identity', 'const other={n:a};const o={x:null};const value=o.x||=other;return o.x===other&&value===other?1:0;'],
  ['managed-property-logical-receiver', "let current={x:null};const old=current;const value=current.x??=(current={x:false},'new');return old.x===value&&current.x===false?1:0;"],
  ['managed-property-logical-sparse', "const xs=[];xs[2]??='new';return xs.length===3&&xs[0]===undefined&&xs[2]==='new'?1:0;"],
].map(([id, body]) => [id, body, body]);
