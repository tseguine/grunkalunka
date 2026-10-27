export const runtimeExtraArguments = [
  ['original','const f=a?()=>a:()=>b;return f(a);'],
  ['snapshot','let f=x=>x+a;return f(a,(f=x=>x-b,a=b,null));'],
  ['returned','const make=x=>{return y=>x+y+a;};const f=make(a);return f(b,a=b,null);'],
  ['callback','const invoke=fn=>fn(a,b)+fn(b,a,b);return invoke(()=>a);'],
  ['selected-callback','const h=a?fn=>fn():fn=>fn(a);return h(()=>b);'],
  ['factory','let make=x=>()=>x+a;return make(a,(make=x=>()=>x-b,a=b,null))();'],
].map(([id,body])=>[`runtime-extra-argument-${id}`,body,body]);
