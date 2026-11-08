export const originalStringObjectBinding={id:'string-object-empty-original',source:"export function run(a,b){const {}='abc';return a;}",sha256:"e0799ecdff5d780c2089495c43d9129ee674ce05ed241c206e8cb306670b580d"};
export const stringObjectPrograms=[[originalStringObjectBinding.id,'','',undefined,undefined,{sourceJS:originalStringObjectBinding.source,sourceTS:originalStringObjectBinding.source}]];
for(const[id,body]of[
 ['length',"const {length:n}='abc';return n;"],
 ['index',"const {0:x='?'}='abc';return x.charCodeAt(0);"],
 ['missing',"const {'99':x='z'}='abc';return x.charCodeAt(0);"],
 ['nested',"const {s:{length:n}}={s:'abc'};return n;"],
 ['var',"const get=()=>n;var {length:n}='abc';return +get();"],
 ['skip-default',"let n=0;const {0:x=(n++, '?'),length:l}='ab';return n+l;"],
 ['take-default',"let n=0;const {'99':x=(n++, 'z'),length:l}='ab';return n+l+x.charCodeAt(0);"],
 ['snapshot',"let s='abc';const {'99':x=(s='z','?'),length:n}=s;return n+s.length;"],
 ['utf16',"const {length:n,0:x='?',1:y='?',2:z='?'}='\\u{1f600}\\ud800';return n+x.charCodeAt(0)+y.charCodeAt(0)+z.charCodeAt(0);"],
])stringObjectPrograms.push(['string-object-'+id,body,body]);
