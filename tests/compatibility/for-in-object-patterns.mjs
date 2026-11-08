export const forInObjectPrograms = [];
for (const [id, body] of [
  ['length', 'let n=0;for(const {length:l} in {a:1,abc:2})n=n*10+l;return n;'],
  ['index', "let n=0;for(let {0:x='?'} in {'':1,abc:2})n+=x.charCodeAt(0);return n;"],
  ['nested', "let n=0;for(const {0:{length:l}='z'} in {'':1,abc:2})n+=l;return n;"],
  ['array', 'let n=0;for(const {length:l} in [a,b])n+=l;return n;'],
  ['string', "let n=0;for(const {length:l} in 'ab')n+=l;return n;"],
  ['empty', 'const before=n;var n=3;for(var {length:n} in {}){}return (before===undefined?10:0)+(+n);'],
  ['shared', 'const get=()=>n;for(var {length:n} in {a:1,abc:2}){}return +get();'],
  ['lexical', 'let first=()=>0,last=()=>0,i=0;for(let {length:n} in {a:1,abc:2}){const get=()=>n;if(i++===0)first=get;last=get;}return first()*10+last();'],
  ['deleted', "let count=0;const obj={a:1,zz:2};for(var {'99':x=(count++,'?')} in obj){delete obj.zz;}return count;"],
  ['defaults', "let count=0;for(const {0:x=(count++,'?'),length:n} in {'':1,a:2}){count+=n;}return count;"],
  ['control', 'let n=0;for(const {length:l} in {a:1,zz:2,abc:3}){if(l===1)continue;if(l===3)break;n+=l;}return n;'],
  ['utf16', "let n=0;for(const {0:x='?',1:y='?',length:l} in {'\\u{1f600}':1})n=l+x.charCodeAt(0)+y.charCodeAt(0);return n;"],
  ['number', 'for(var {} in 17){}return a;'],
]) forInObjectPrograms.push(['for-in-object-' + id, body, body]);
