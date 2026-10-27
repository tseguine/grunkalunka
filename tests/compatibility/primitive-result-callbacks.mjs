export const primitiveResultCallbacks = [
  ['null','const apply=(fn,x)=>fn(x);return +apply(x=>null,a);'],
  ['undefined','const apply=(fn,x)=>fn(x);return +apply(x=>undefined,a);'],
  ['boolean','const apply=(fn,x)=>fn(x);return apply(x=>x?false:b,a)?a:b;'],
  ['local','const apply=(fn,x)=>fn(x);let value=apply(x=>null,a);value=b;return +value;'],
  ['higher','const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y?false:b,x);return +invoke(apply,a);'],
  ['identity','const apply=(fn,x)=>fn(x);const id=x=>x;const f=x=>x+b;return +apply(x=>null,a)+id(a)+id(f)(b);'],
].map(([id,body])=>[`primitive-result-callback-${id}`,body,body]);
