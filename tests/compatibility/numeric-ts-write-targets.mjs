// Hand-paired full sources; TS wrappers retain original binding references, not coercions.
// https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#type-assertions
// https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-9.html#the-satisfies-operator
export const numericTsWriteTargets = [];
numericTsWriteTargets.push(["ts-erasure-as-assignment-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a = b; }", "sourceTS": "export function run(a:number,b:number):number { return ((a) as number) = b; }"}]);
numericTsWriteTargets.push(["ts-erasure-as-update-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a++; }", "sourceTS": "export function run(a:number,b:number):number { return ((a) as number)++; }"}]);
numericTsWriteTargets.push(["ts-erasure-angle-assignment-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a = b; }", "sourceTS": "export function run(a:number,b:number):number { return (<number>(a)) = b; }"}]);
numericTsWriteTargets.push(["ts-erasure-angle-update-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a++; }", "sourceTS": "export function run(a:number,b:number):number { return (<number>(a))++; }"}]);
numericTsWriteTargets.push(["ts-erasure-non-null-assignment-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a = b; }", "sourceTS": "export function run(a:number,b:number):number { return ((a)!) = b; }"}]);
numericTsWriteTargets.push(["ts-erasure-non-null-update-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a++; }", "sourceTS": "export function run(a:number,b:number):number { return ((a)!)++; }"}]);
numericTsWriteTargets.push(["ts-erasure-satisfies-assignment-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a = b; }", "sourceTS": "export function run(a:number,b:number):number { return ((a) satisfies number) = b; }"}]);
numericTsWriteTargets.push(["ts-erasure-satisfies-update-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a++; }", "sourceTS": "export function run(a:number,b:number):number { return ((a) satisfies number)++; }"}]);
numericTsWriteTargets.push(["update-wrapped-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a++; }", "sourceTS": "export function run(a:number,b:number):number { return (a as number)++; }"}]);
numericTsWriteTargets.push(["ts-wrapped-write", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a=b; }", "sourceTS": "export function run(a:number,b:number):number { return (a as number)=b; }"}]);
numericTsWriteTargets.push(["compound-ts-wrapped-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a<<=b; }", "sourceTS": "export function run(a,b) { return (a as number)<<=b; }"}]);
numericTsWriteTargets.push(["compound-ts-nonnull-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { return a>>=b; }", "sourceTS": "export function run(a,b) { return a!>>=b; }"}]);
numericTsWriteTargets.push(["statement-wrapped-assignment-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { (a=b); return a; }", "sourceTS": "export function run(a,b) { ((a as number)=b); return a; }"}]);
numericTsWriteTargets.push(["statement-wrapped-update-target", "", "", undefined, undefined, {"sourceJS": "export function run(a,b) { a++; return a; }", "sourceTS": "export function run(a,b) { (a as number)++; return a; }"}]);

const add=(id,js,ts,expected)=>numericTsWriteTargets.push([id,js,ts,expected]);
const forms=[
 ['as', x=>`(${x} as unknown as string)`],
 ['angle', x=>`(<number>${x})`],
 ['nonnull', x=>`(${x}!)`],
 ['satisfies', x=>`(${x} satisfies number)`],
];
for(const [name,wrap]of forms) {
 for(const [kind,js,ts]of [
  ['prefix-inc','return ++x;',`return ++${wrap('x')};`],
  ['postfix-inc','return x++;',`return ${wrap('x')}++;`],
  ['prefix-dec','return --x;',`return --${wrap('x')};`],
  ['postfix-dec','return x--;',`return ${wrap('x')}--;`],
 ]) add(`ts-target-${name}-${kind}`,`let x=a; ${js}`,`let x=a; ${ts}`);
}
for(const [i,op]of ['+=','-=','*=','/=','%=','&=','|=','^=','<<=','>>=','>>>='].entries()) {
 const wrap=forms[i%forms.length][1];
 add(`ts-target-compound-${i}`,`let x=5; const y=(x${op}(x=3)); return y*10+x;`,
  `let x=5; const y=(${wrap('x')}${op}(x=3)); return y*10+x;`);
}
add('ts-target-assign-rhs-once','let x=1; let n=0; const y=(x=(n++,3)); return n*100+y*10+x;',
 'let x=1; let n=0; const y=((x as number)=(n++,3)); return n*100+y*10+x;',133);
add('ts-target-compound-snapshot','let x=5; return x+=(x=3);','let x=5; return (x as number)+=(x=3);',8);
add('ts-target-mixed-inner-parens','let x=a; const y=x++; return y-x;',
 'let x=a; const y=((<number>((x! as number))) satisfies number)++; return y-x;');
add('ts-target-shadow','let x=a; { let x=b; x+=3; } return x;',
 'let x=a; { let x=b; (x as number)+=3; } return x;');
add('ts-target-for-step','let x=a; for(let i=0;i<2;i++) x+=b; return x;',
 'let x=a; for(let i=0;i<2;(i as number)++) (x as unknown)+=b; return x;');
add('ts-target-selected','let x=a; if(1) x=b; return x;',
 'let x=a; if(1) (x satisfies number)=b; return x;');
add('ts-target-depth128','return a=b;',`return a${'!'.repeat(128)} =b;`);
add('ts-target-inner-paren-depth128','return a=b;',`return ((a${'!'.repeat(126)}) as number)=b;`);
add('ts-target-nested-depth128','return a=(b=a);',`return a=(b${'!'.repeat(126)} =a);`);
numericTsWriteTargets.push(['ts-target-helper','','',undefined,undefined,{
 sourceJS:'function f(x,y) { const r=(x+=(x=y)); return r; } export function run(a,b) { return f(a,b); }',
 sourceTS:'function f(x:number,y:number):number { const r=((x as unknown)+=((x!)=y)); return r; } export function run(a:number,b:number):number { return f(a,b); }',
}]);
