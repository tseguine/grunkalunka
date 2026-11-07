// Runtime names retain a missing alternative and every possible dictionary value.
export const dictionaryPrimitives = [
  ['dictionary-primitive-arithmetic', "const d=Object.create(null);d.x=a;const k='x';return d[k]+b;"],
  ['dictionary-primitive-missing', "const d=Object.create(null);d.x=a;const k=a?'x':'missing';return +d[k];"],
  ['dictionary-primitive-union', "const d=Object.create(null);const k='x';d[k]=a?true:b;return +d[k];"],
  ['dictionary-primitive-snapshot', "const d=Object.create(null);d.x=a;const k='x';const saved=d[k];d.x=b;return saved+d[k];"],
  ['dictionary-primitive-helper', "const d=Object.create(null);d.x=a;const get=(o,k)=>o[k];return +get(d,'x');"],
].map(([id, body]) => [id, body, body]);
