export const memberPatternPrograms=[
 [
  "member-pattern-array",
  "const o={x:a};[o.x]=[b];return o.x;",
  "const o={x:a};[o.x]=[b];return o.x;"
 ],
 [
  "member-pattern-object",
  "const o={x:a};({v:o.x}={v:b});return o.x;",
  "const o={x:a};({v:o.x}={v:b});return o.x;"
 ],
 [
  "member-pattern-index",
  "const xs=[];[xs[0],xs[1]]=[a,b];return xs.length;",
  "const xs=[];[xs[0],xs[1]]=[a,b];return xs.length;"
 ],
 [
  "member-pattern-default",
  "const o={x:a};[o.x=b]=[];return o.x;",
  "const o={x:a};[o.x=b]=[];return o.x;"
 ],
 [
  "member-pattern-reference-before-read",
  "const rows=[a];const o={x:0};const get=()=>{rows[0]=b;return o;};[get().x]=rows;return o.x;",
  "const rows=[a];const o={x:0};const get=()=>{rows[0]=b;return o;};[get().x]=rows;return o.x;"
 ],
 [
  "member-pattern-object-reference-before-read",
  "const row={v:a};const o={x:0};const get=()=>{row.v=b;return o;};({v:get().x}=row);return o.x;",
  "const row={v:a};const o={x:0};const get=()=>{row.v=b;return o;};({v:get().x}=row);return o.x;"
 ],
 [
  "member-pattern-saved-reference",
  "let o={x:a};const old=o;[o.x=(o={x:b},1)]=[];return old.x===1&&o.x===b?1:0;",
  "let o={x:a};const old=o;[o.x=(o={x:b},1)]=[];return old.x===1&&o.x===b?1:0;"
 ],
 [
  "member-pattern-later-selection",
  "const row={v:undefined,w:a};const o={x:0,y:0};({v:o.x=(row.w=b,1),w:o.y}=row);return o.y;",
  "const row={v:undefined,w:a};const o={x:0,y:0};({v:o.x=(row.w=b,1),w:o.y}=row);return o.y;"
 ],
 [
  "member-pattern-empty-string",
  "const o={x:undefined};[o.x]='';return o.x===undefined?1:0;",
  "const o={x:undefined};[o.x]='';return o.x===undefined?1:0;"
 ],
 [
  "member-pattern-string",
  "const o={x:''};[o.x]='\ud83d\udca0';return o.x==='\ud83d\udca0'?1:0;",
  "const o={x:''};[o.x]='\ud83d\udca0';return o.x==='\ud83d\udca0'?1:0;"
 ],
 [
  "member-pattern-done-iterator",
  "const values=[];const o={x:undefined,y:undefined};const grow=()=>{values[1]=a;return o;};[o.x,grow().y]=values;return o.x===undefined&&o.y===undefined?1:0;",
  "const values=[];const o={x:undefined,y:undefined};const grow=()=>{values[1]=a;return o;};[o.x,grow().y]=values;return o.x===undefined&&o.y===undefined?1:0;"
 ],
 [
  "member-pattern-result",
  "const o={x:a};const values=[b];return ([o.x]=values)===values?1:0;",
  "const o={x:a};const values=[b];return ([o.x]=values)===values?1:0;"
 ]
];
