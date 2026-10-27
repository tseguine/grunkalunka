export const higherCallbackShapes = [
  ['direct','const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(apply,a);'],
  ['selected','const apply=(fn,x)=>fn(x);const invoke=a?((cb,x)=>cb(y=>y+b,x)):((cb,x)=>cb(y=>y-b,x));return invoke(apply,a);'],
  ['forward','const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);return invoke(apply,z=>z+b,a);'],
  ['layers','const apply=(fn,x)=>fn(x);const middle=(cb,x)=>cb(y=>y+b,x);const outer=(step,x)=>step(apply,x);return outer(middle,a);'],
  ['returned','const apply=(fn,x)=>fn(x);const make=offset=>(cb,x)=>()=>cb(y=>y+offset,x);return make(a)(apply,b)();'],
  ['snapshot','const invoke=(cb,x)=>cb(y=>y+b,x);let apply=(fn,x)=>fn(x);return invoke(apply,(apply=(fn,x)=>fn(x)-b,a));'],
].map(([id,body])=>[`higher-callback-shape-${id}`,body,body]);
