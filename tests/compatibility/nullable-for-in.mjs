export const nullableForInPrograms=[
 [
  "nullable-for-in-object",
  "const o=a?{x:b,yy:a}:null;let n=0;for(const k in o)n+=k.length;return n;",
  "const o=a?{x:b,yy:a}:null;let n=0;for(const k in o)n+=k.length;return n;"
 ],
 [
  "nullable-for-in-array",
  "const xs=a?[b,,a]:undefined;let n=0;for(const k in xs)n+=k.length;return n;",
  "const xs=a?[b,,a]:undefined;let n=0;for(const k in xs)n+=k.length;return n;"
 ],
 [
  "nullable-for-in-string",
  "const s=a?'A\ud83d\udca0':null;let n=0;for(const k in s)n++;return n;",
  "const s=a?'A\ud83d\udca0':null;let n=0;for(const k in s)n++;return n;"
 ],
 [
  "nullable-for-in-rhs-once",
  "let n=0;const get=()=>{n++;return a?{x:b}:null;};for(const k in get())n+=10;return n;",
  "let n=0;const get=()=>{n++;return a?{x:b}:null;};for(const k in get())n+=10;return n;"
 ],
 [
  "nullable-for-in-snapshot",
  "let o=a?{x:b,y:a}:null;let n=0;for(const k in o){n++;o=null;}return n;",
  "let o=a?{x:b,y:a}:null;let n=0;for(const k in o){n++;o=null;}return n;"
 ],
 [
  "nullable-for-in-delete",
  "const row={x:a,y:b,z:a};const o=a?row:null;let n=0;for(const k in o){n++;delete row.y;}return n;",
  "const row={x:a,y:b,z:a};const o=a?row:null;let n=0;for(const k in o){n++;delete row.y;}return n;"
 ],
 [
  "nullable-for-in-array-shrink",
  "const xs=[a,b,b];const o=a?xs:null;let n=0;for(const k in o){n++;xs.length=1;}return n;",
  "const xs=[a,b,b];const o=a?xs:null;let n=0;for(const k in o){n++;xs.length=1;}return n;"
 ],
 [
  "nullable-for-in-head",
  "let k='old';const o=a?{x:b}:null;for(k in o){}return k==='old'?1:0;",
  "let k='old';const o=a?{x:b}:null;for(k in o){}return k==='old'?1:0;"
 ],
 [
  "nullable-for-in-dictionary",
  "const d=Object.create(null);d.x=b;const o=a?d:null;let n=0;for(const k in o)n++;return n;",
  "const d=Object.create(null);d.x=b;const o=a?d:null;let n=0;for(const k in o)n++;return n;"
 ],
 [
  "nullable-for-in-helper",
  "const count=o=>{let n=0;for(const k in o)n++;return n;};return count(a?{x:b}:undefined);",
  "const count=o=>{let n=0;for(const k in o)n++;return n;};return count(a?{x:b}:undefined);"
 ],
 [
  "nullable-for-in-member-head",
  "const dest={k:'old'};const o=a?{x:b}:null;for(dest.k in o){}return dest.k==='old'?1:0;",
  "const dest={k:'old'};const o=a?{x:b}:null;for(dest.k in o){}return dest.k==='old'?1:0;"
 ],
 [
  "nullable-for-in-mixed-nullish",
  "const o=a?{x:b}:(b?null:undefined);let n=0;for(const k in o)n++;return n;",
  "const o=a?{x:b}:(b?null:undefined);let n=0;for(const k in o)n++;return n;"
 ]
];
nullableForInPrograms.push(["nullable-for-in-var-former","const source=a?{x:b}:null;for(var key in source){}return a;","const source=a?{x:b}:null;for(var key in source){}return a;",undefined,undefined,{"sourceJS": "export function run(a,b){const source=a?{x:b}:null;for(var key in source){}return a;}", "sourceTS": "export function run(a,b){const source=a?{x:b}:null;for(var key in source){}return a;}", "oracleSource": "export function run(a,b){const source=a?{x:b}:null;for(var key in source){}return a;}"}]);
nullableForInPrograms.push(["nullable-for-in-pattern-former","for(const {length:n} in (a?'abc':null)){}return a;","for(const {length:n} in ((a?'abc':null) as object)){}return a;",undefined,undefined,{"sourceJS": "export function run(a,b){for(const {length:n} in (a?'abc':null)){}return a;}", "sourceTS": "export function run(a,b){for(const {length:n} in ((a?'abc':null) as object)){}return a;}", "oracleSource": "export function run(a,b){for(const {length:n} in (a?'abc':null)){}return a;}"}]);
