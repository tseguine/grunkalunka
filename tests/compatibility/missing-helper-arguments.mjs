export const missingHelperArguments = [
  ['missing-helper-undefined', '', '', undefined, undefined, {
    sourceJS:'function flag(x){return x===undefined?1:0;}export function run(a,b){return flag();}',
    sourceTS:'function flag(x?:unknown){return x===undefined?1:0;}export function run(a,b){return flag();}',
  }],
  ['missing-helper-numeric-coercion', '', '', undefined, undefined, {
    sourceJS:'function add(x,y){return x+ +y;}export function run(a,b){return add(a);}',
    sourceTS:'function add(x,y){return x+ +y;}export function run(a,b){return add(a);}',
  }],
  ['missing-helper-nullish-fallback', '', '', undefined, undefined, {
    sourceJS:'function choose(x,y){return y??x;}export function run(a,b){return choose(a);}',
    sourceTS:'function choose(x?:unknown,y?:unknown){return y??x;}export function run(a,b){return choose(a);}',
  }],
  ['missing-helper-call-effects', '', '', undefined, undefined, {
    sourceJS:'function flag(x){return x===undefined?1:0;}export function run(a,b){let n=0;const value=flag.call((n++,a));return value*10+n;}',
    sourceTS:'function flag(x){return x===undefined?1:0;}export function run(a,b){let n=0;const value=flag.call((n++,a));return value*10+n;}',
  }],
  ['missing-helper-capture', '', '', undefined, undefined, {
    sourceJS:'function f(x){const g=()=>x===undefined?1:0;return g();}export function run(a,b){return f();}',
    sourceTS:'function f(x){const g=()=>x===undefined?1:0;return g();}export function run(a,b){return f();}',
  }],
  ['missing-helper-object-fallback', '', '', undefined, undefined, {
    sourceJS:'function choose(x,y){return y??x;}export function run(a,b){const value=choose({v:a});return +value.v;}',
    sourceTS:'function choose(x,y){return y??x;}export function run(a,b){const value=choose({v:a});return +value.v;}',
  }],
  ['missing-helper-mixed-calls', '', '', undefined, undefined, {
    sourceJS:'function flag(x){return x===undefined?1:0;}export function run(a,b){return flag()*100+flag(a)*10+flag(NaN);}',
    sourceTS:'function flag(x){return x===undefined?1:0;}export function run(a,b){return flag()*100+flag(a)*10+flag(NaN);}',
  }],
];
