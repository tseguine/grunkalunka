// Runtime-selected factories return Number-parameter closures with primitive results.
export const primitiveFactoryLeaves = [
  ['null', 'let make=x=>()=>null;return +make(a)();'],
  ['boolean', 'let make=x=>()=>true;return +make(a)();'],
  ['undefined', 'let make=x=>()=>undefined;return +make(a)();'],
  ['conditional', 'const make=a?x=>()=>null:x=>()=>false;return +make(b)();'],
  ['snapshot', 'let make=x=>()=>null;const old=make;make=x=>()=>true;return +old(a)()+ +make(b)();'],
  ['block', 'let make=x=>()=>{if(x)return b;};return +make(a)();'],
].map(([id,body]) => [`primitive-factory-leaf-${id}`,body,body]);
