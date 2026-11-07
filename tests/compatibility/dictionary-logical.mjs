// Runtime dictionary logical assignments save the receiver, key and old value.
export const dictionaryLogical = [
  ['dictionary-logical-former-guard', "const d=Object.create(null);const key='x';d[key]??=a;return a;"],
  ['dictionary-logical-missing', "const d=Object.create(null);const k='x';return (d[k]??=a)===a?1:0;"],
  ['dictionary-logical-or', "const d=Object.create(null);const k='x';d[k]||='ok';return d[k]==='ok'?1:0;"],
  ['dictionary-logical-and-skip', "const d=Object.create(null);const k='x';d[k]=false;let n=0;const value=d[k]&&=(n++,'new');return value===false&&n===0?1:0;"],
  ['dictionary-logical-saved-reference', "let d=Object.create(null);const old=d;let k='x';const value=d[(d=Object.create(null),k)]??=(k='y','new');return old.x===value&&old.y===undefined&&d.x===undefined?1:0;"],
  ['dictionary-logical-key-read-order', "const d=Object.create(null);const k='x';d.x=null;const value=d[(d.x='old',k)]??='new';return value==='old'&&d.x==='old'?1:0;"],
  ['dictionary-logical-special-key', "const d=Object.create(null);const k='__proto__';const other={n:a};const value=d[k]??=other;return d[k]===other&&value===other?1:0;"],
].map(([id, body]) => [id, body, body]);
