export const functionForInPrograms=[
 ["function-for-in-arrow", "const f=x=>x+a;let n=0;for(const k in f)n++;return n;", "const f=x=>x+a;let n=0;for(const k in f)n++;return n;"],
 ["function-for-in-literal", "let n=0;for(const k in (x=>x+a))n++;return n;", "let n=0;for(const k in (x=>x+a))n++;return n;"],
 ["function-for-in-conditional", "const f=x=>x+a;const g=x=>x+b;let n=0;for(const k in (a?f:g))n++;return n;", "const f=x=>x+a;const g=x=>x+b;let n=0;for(const k in (a?f:g))n++;return n;"],
 ["function-for-in-rhs-effects", "let n=0;const f=x=>x+a;for(const k in (n++,f))n+=10;return n;", "let n=0;const f=x=>x+a;for(const k in (n++,f))n+=10;return n;"],
 ["function-for-in-factory", "let n=0;const make=()=>{n++;return x=>x+a;};for(const k in make())n+=10;return n;", "let n=0;const make=()=>{n++;return x=>x+a;};for(const k in make())n+=10;return n;"],
 ["function-for-in-body-not-called", "let n=0;const f=()=>{n++;return a;};for(const k in f)n+=10;return n;", "let n=0;const f=()=>{n++;return a;};for(const k in f)n+=10;return n;"],
 ["function-for-in-head-unchanged", "let key='old';const f=x=>x+a;for(key in f)key='changed';return key==='old'?1:0;", "let key='old';const f=x=>x+a;for(key in f)key='changed';return key==='old'?1:0;"],
 ["function-for-in-var", "var key;const f=x=>x+a;for(var key in f){}return key===undefined?1:0;", "var key;const f=x=>x+a;for(var key in f){}return key===undefined?1:0;"],
 ["function-for-in-pattern", "let n=0;const f=x=>x+a;for(const {0:k=(n++,'x')} in f)n+=10;return n;", "let n=0;const f=x=>x+a;for(const {0:k=(n++,'x')} in f)n+=10;return n;"],
 ["function-for-in-nested", "let n=0;const f=x=>x+a;for(const x of [a,b])for(const k in (n++,f))n+=10;return n;", "let n=0;const f=x=>x+a;for(const x of [a,b])for(const k in (n++,f))n+=10;return n;"],
 ["function-for-in-call-after", "const f=x=>x+a;for(const k in f){}return f(b);", "const f=x=>x+a;for(const k in f){}return f(b);"],
];
functionForInPrograms.push(["function-for-in-named", "let n=0;for(const k in f)n++;return n;", "let n=0;for(const k in f)n++;return n;",undefined,undefined,{"sourceJS": "function f(x){return x+1;}export function run(a,b){let n=0;for(const k in f)n++;return n;}", "sourceTS": "function f(x){return x+1;}export function run(a,b){let n=0;for(const k in f)n++;return n;}", "oracleSource": "function f(x){return x+1;}export function run(a,b){let n=0;for(const k in f)n++;return n;}"}]);
