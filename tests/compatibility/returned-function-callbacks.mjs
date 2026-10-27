export const returnedFunctionCallbacks = [
  ['call', 'let make=x=>(fn,y)=>fn(y)+x;return make(a)(z=>z+b,b);'],
  ['block', 'let make=x=>{return (fn,y)=>fn(y)+x;};return make(a)(z=>z+b,b);'],
  ['capture', 'let make=x=>(fn,y)=>()=>fn(y)+x;return make(a)(z=>z+b,b)();'],
  ['rebind', 'let make=x=>(fn,y)=>{const old=fn;fn=z=>z+b;return old(y)+fn(y)+x;};return make(a)(z=>z+a,b);'],
  ['snapshot', 'let make=x=>(fn,y)=>()=>fn(y);let fn=z=>z+a;const f=make(a)(fn,(fn=z=>z-b,b));return f()+fn(a);'],
  ['two-callbacks', 'let make=x=>(f,g,y)=>f(y)+g(y)+x;return make(a)(z=>z+a,z=>z-b,b);'],
].map(([id,body])=>[`returned-function-callback-${id}`,body,body]);
