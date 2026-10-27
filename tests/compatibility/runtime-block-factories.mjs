// Complete synchronous block creators retain typed function return boundaries.
export const runtimeBlockFactories = [
  ['branch', 'let make=x=>{if(x)return ()=>x+a;return ()=>x-b;};return make(a)();'],
  ['loop', 'let make=x=>{let i=0;while(i<3){i++;if(x)return ()=>i+b;}return ()=>i;};return make(a)();'],
  ['fresh', 'let make=x=>{let y=x;return ()=>++y;};const f=make(a),g=make(b);return f()+g()+f();'],
  ['callback', 'let make=(fn,x)=>{return ()=>fn(x);};return make(y=>y+b,a)();'],
  ['snapshot', 'let make=x=>{return ()=>x+a;};const f=make((make=x=>{return ()=>x-b;},b));return f()+make(a)();'],
  ['mixed-body', 'const first=x=>{return ()=>x+a;};const second=x=>()=>x-b;const make=a?first:second;return make(b)();'],
].map(([id,body])=>[`runtime-block-factory-${id}`,body,body]);
