// Abstract equality with a proven nullish operand needs no managed coercion.
export const managedNullishEquality = [
  ['managed-nullish-equality-former-object-guard', 'const o={};return o==null?a:b;'],
  ['managed-nullish-equality-null', 'const o={x:null};return o.x==null?1:0;'],
  ['managed-nullish-equality-missing', 'const o={};return undefined==o.x?1:0;'],
  ['managed-nullish-equality-mixed', "const o={x:a?'text':null};return (o.x!=null)==!!a?1:0;"],
  ['managed-nullish-equality-tags', "let count=0;for(const value of [null,undefined,false,0,NaN,'','x',{},[],()=>a]){if(value==null)count++;if(undefined!=value)count+=10;}return count;"],
  ['managed-nullish-equality-snapshot', "const o={x:null};let n=0;const value=(n=1,o.x)==void(n=n*10+2,o.x='changed');return value&&n===12&&o.x==='changed'?1:0;"],
  ['managed-nullish-equality-sentinel', "const o={x:'text'};const sentinel=a?null:undefined;return sentinel!=o.x?1:0;"],
].map(([id, body]) => [id, body, body]);
