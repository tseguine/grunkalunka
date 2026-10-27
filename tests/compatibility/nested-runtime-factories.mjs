export const nestedRuntimeFactories = [
  ['original', 'let make=x=>()=>()=>x;return make(a)()();'],
  ['parameters', 'let make=x=>y=>()=>x+y;return make(a)(b)();'],
  ['branches', 'let make=a?(x=>y=>()=>x+y):(x=>y=>()=>x-y);return make(a)(b)();'],
  ['block', 'let make=x=>{return y=>{return ()=>x+y;};};return make(a)(b)();'],
  ['snapshot', 'let make=x=>y=>()=>x+y;const middle=make(a);make=x=>y=>()=>x-y;return middle(b)()+make(a)(b)();'],
  ['primitive', 'let make=x=>()=>()=>null;return +make(a)()();'],
].map(([id,body])=>[`nested-runtime-factory-${id}`,body,body]);
