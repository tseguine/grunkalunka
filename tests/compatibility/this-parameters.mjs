// TypeScript this metadata is absent from the ordinary argument list.
export const thisParameterPromotions = [
  {
    id: 'this-param',
    sourceTS: 'export function run(this:void,a:number,b:number):number { return a; }',
    sourceJS: 'export function run(a,b) { return a; }',
    sourceSha256: '19e40023e81d66de28b6066c46a0bb78ee91c241543adb2f540075dca0b529cc',
  },
  {
    id: 'direct-call-helper-this',
    sourceTS: 'function helper(this:number,x:number) { return x; } export function run(a,b) { return helper(a); }',
    sourceJS: 'function helper(x) { return x; } export function run(a,b) { return helper(a); }',
    sourceSha256: 'ec44d4f02357682c491b61972ed3715b7928271a0b04af267a8f3d3bb063c5a4',
  },
  {
    id: 'generic-declaration-reject-this',
    sourceTS: 'export function run<T>(this:T,a:T,b:T):T { return a; }',
    sourceJS: 'export function run(a,b) { return a; }',
    sourceSha256: '99f332ee3c5179238953c007de6e249319cd593a489216e6891f6be61556ef68',
  },
];

export const thisParameterPrograms = [
  ...thisParameterPromotions.map(({ id, sourceJS, sourceTS }) =>
    [id, '', '', undefined, undefined, { sourceJS, sourceTS }]),
  ...[
    ['this-parameter-zero-arity',
      'export function run(){return 4+run.length;}',
      'export function run(this:Missing){return 4+run.length;}'],
    ['this-parameter-helper-default-length',
      'function f(x,y=3){return x+y;}export function run(a,b){return f.length+f(a)+f.call(b,a,b);}',
      'function f(this:typeof absent,x:number,y=3){return x+y;}export function run(this:void,a,b){return f.length+f(a)+f.call(b,a,b);}'],
    ['this-parameter-call-effects',
      'function add(x,y){return x+y;}export function run(a,b){let n=0;const r=add.call(n++,n++,n++);return r*10+n;}',
      'function add(this:never,x,y){return x+y;}export function run(this:Missing,a,b){let n=0;const r=add.call(n++,n++,n++);return r*10+n;}'],
  ].map(([id, sourceJS, sourceTS]) => [id, '', '', undefined, undefined, { sourceJS, sourceTS }]),
];
