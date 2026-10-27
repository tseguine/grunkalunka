export const knownLocalExtraArguments = [
  ['original','const f=x=>x; return f(a,b);'],
  ['capture','const f=x=>x+a;return f(a,a=b);'],
  ['discard','const f=x=>x+a;return f(a,(a=b,null));'],
  ['block-factory','const make=x=>{const f=()=>x+a;return f;};return make(a,a=b)();'],
  ['curried','const make=x=>y=>()=>x+y+a;return make(a,a=b)(b)();'],
  ['identity','const id=x=>x;const f=id(()=>a,a=b,null);return f()+id(a,undefined);'],
].map(([id,body])=>[`known-local-extra-argument-${id}`,body,body]);
