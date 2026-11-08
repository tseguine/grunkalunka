export const scalarPatternPrograms=[
 [
  "scalar-pattern-array",
  "[a]=[b];return a;",
  "[a]=[b];return a;"
 ],
 [
  "scalar-pattern-object",
  "({x:a}={x:b});return a;",
  "({x:a}={x:b});return a;"
 ],
 [
  "scalar-pattern-swap",
  "[a,b]=[b,a];return a-b;",
  "[a,b]=[b,a];return a-b;"
 ],
 [
  "scalar-pattern-default",
  "[a=b,b=a]=[];return a===b?1:0;",
  "[a=b,b=a]=[];return a===b?1:0;"
 ],
 [
  "scalar-pattern-object-default",
  "({x:a=b}={});return a;",
  "({x:a=b}={});return a;"
 ],
 [
  "scalar-pattern-capture",
  "const read=()=>a;[a]=[b];return read();",
  "const read=()=>a;[a]=[b];return read();"
 ],
 [
  "scalar-pattern-rhs-once",
  "let n=0;const values=()=>{n++;return [b];};[a]=values();return n;",
  "let n=0;const values=()=>{n++;return [b];};[a]=values();return n;"
 ],
 [
  "scalar-pattern-later-default",
  "[a=2,b=a+1]=[];return b;",
  "[a=2,b=a+1]=[];return b;"
 ],
 [
  "scalar-pattern-snapshot",
  "[a,b]=[b,(a=1,2)];return a+b;",
  "[a,b]=[b,(a=1,2)];return a+b;"
 ],
 [
  "scalar-pattern-result",
  "const values=[b];const result=([a]=values);return result===values&&a===b?1:0;",
  "const values=[b];const result=([a]=values);return result===values&&a===b?1:0;"
 ],
 [
  "scalar-pattern-object-live",
  "const row={x:undefined,y:b};({x:a=(row.y=7,1),y:b}=row);return a+b;",
  "const row={x:undefined,y:b};({x:a=(row.y=7,1),y:b}=row);return a+b;"
 ],
 [
  "scalar-pattern-shorthand",
  "({a}={a:b});return a;",
  "({a}={a:b});return a;"
 ]
];
