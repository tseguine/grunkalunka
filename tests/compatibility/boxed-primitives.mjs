// Preserve represented primitive tags when leaving boxed storage for scalar use.
export const boxedPrimitives = [
  ['boxed-primitive-number-bool', 'const o={v:a?true:b};return +o.v;'],
  ['boxed-primitive-number-null', 'const o={v:a?null:b};return o.v/b;'],
  ['boxed-primitive-bool-undefined', 'const o={v:a?undefined:false};return ~o.v;'],
  ['boxed-primitive-deleted-field', 'const o={x:a};delete o.x;return o.x+b;'],
  ['boxed-primitive-sparse-hole', 'const xs=[a,,b];return xs[1]+b;'],
  ['boxed-primitive-loose-equality', 'const o={flag:true};return o.flag==1?1:0;'],
  ['boxed-primitive-equality-snapshot', 'const o={v:a?true:b};const value=o.v==(o.v=null,b);return (value?1:0)+ +o.v;'],
  ['boxed-primitive-helper', 'const read=x=>x.v;const o={v:a?null:b};return +read(o);'],
].map(([id, body]) => [id, body, body]);
