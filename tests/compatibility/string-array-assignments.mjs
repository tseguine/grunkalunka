export const stringAssignmentPrograms = [];
for (const [id, body] of [
  ['element', "let x;[x]='abc';return x==='a'?1:0;"],
  ['empty-predicate', "let x;return ([x]='')?1:0;"],
  ['nonempty-predicate', "let x;return ([x]='abc')?1:0;"],
  ['negated-predicate', "let x;return !([x]='')?1:0;"],
  ['result', "let x;const result=([x]='abc');return result.length+(x==='a'?10:0);"],
  ['snapshot', "let x,n=0;let source='';const result=([x=(n++,source='abc','?')]=source);return result===''&&source==='abc'&&x==='?'?n:0;"],
  ['defaults', "let x,y,n=0;[x=(n++,'?'),y=(n++,'!')]='a';return n+(x==='a'?10:0)+(y==='!'?100:0);"],
  ['unicode', "let x,y;[x,y]='\\u{1f600}\\ud800';return x==='\\u{1f600}'&&y==='\\ud800'?1:0;"],
  ['elision', "let x,rest=[];[,x,...rest]='\\u{1f600}ab';return (x==='a'?10:0)+rest.length;"],
  ['fresh-rest', "let x,rest=[];[x,...rest]='abc';const saved=rest;[x,...rest]='abc';rest[0]='z';return saved!==rest&&saved[0]==='b'?1:0;"],
  ['capture', "let x;const read=()=>x;[x]='abc';return read()==='a'?1:0;"],
  ['var', "var x;[x]='abc';return x==='a'?1:0;"],
  ['once', "let x,n=0;const make=()=>{n++;return 'abc';};[x]=make();return n+(x==='a'?10:0);"],
]) stringAssignmentPrograms.push(['string-assignment-' + id, body, body]);
