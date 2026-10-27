// Same-full-signature functions are truthy and non-nullish; RHS effects stay selected.
export const functionLogicalAssignment = [
  ['and', 'let f=()=>a;f&&=()=>b;return f();'],
  ['or', 'let f=()=>a;f||=()=>b;return f();'],
  ['nullish', 'let f=()=>a;f??=()=>b;return f();'],
  ['snapshot', 'let f=()=>a;const old=f;const selected=f&&=()=>b;f=()=>a+b;return old()+selected()+f();'],
  ['constructor-skip', 'let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f??=make(b);return f()+n;'],
  ['callback', 'const apply=(fn,x)=>{fn&&=y=>y+b;return fn(x);};return apply(x=>x+a,a);'],
].map(([id,body]) => [`function-logical-assignment-${id}`,body,body]);
