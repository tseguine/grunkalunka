export const stringObjectAssignmentPrograms=[];
for(const [id,body] of [
 ['length',"let n=0;({length:n}='abc');return n;"],
 ['result',"let x;const result=({0:x}='abc');return result.length+(x==='a'?10:0);"],
 ['skip',"let n=0,x='';({0:x=(n++,'?'),length:n}='abc');return n+(x==='a'?10:0);"],
 ['take',"let n=0,x='';({'99':x=(n++,'?')}='abc');return n+(x==='?'?10:0);"],
 ['empty',"let x;return ({}='')?1:0;"],
 ['nonempty',"let x;return ({0:x}='abc')?1:0;"],
 ['snapshot',"let x,n=0;let source='';const result=({'99':x=(source='abc','?'),length:n}=source);return result===''&&n===0&&source==='abc'?1:0;"],
 ['utf16',"let x,y,n=0;({0:x,1:y,length:n}='\\u{1f600}');return n+(x==='\\ud83d'?10:0)+(y==='\\ude00'?100:0);"],
 ['capture',"var n;const get=()=>n;({length:n}='abc');return +get();"],
 ['once',"let n=0,c=0;const make=()=>{c++;return 'abc';};({length:n}=make());return n+c;"],
])stringObjectAssignmentPrograms.push(['string-object-assignment-'+id,body,body]);
