const cases=[
 ['object','const o={child:{x:a}};return o.child.x===a?1:0;'],
 ['object-array','const o={child:[a,b]};return o.child[0]===a?1:0;'],
 ['array-object','const o=[{x:a}];return o[0].x===a?1:0;'],
 ['array-array','const o=[[a,b]];return o[0][0]===a?1:0;'],
 ['object-alias','const o={child:{x:a}};const p=o.child;p.x=b;return o.child.x===b?1:0;'],
 ['array-alias','const o={child:[a]};const p=o.child;p[0]=b;return o.child[0]===b?1:0;'],
 ['object-rhs','const old={x:a},next={x:a},o={child:old};o.child.x=(o.child=next,b);return old.x===b&&next.x===a&&o.child===next?1:0;'],
 ['array-rhs','const old=[a],next=[a],o={child:old};o.child[0]=(o.child=next,b);return old[0]===b&&next[0]===a&&o.child===next?1:0;'],
 ['saved-child','const o={child:{x:a}};const old=o.child;o.child={x:b};return old!==o.child&&old.x===a&&o.child.x===b?1:0;'],
 ['assignment-result','const o={child:{x:a}};return(o.child={x:b}).x===b?1:0;'],
 ['cycle','const o={child:{}};o.child.parent=o;return o.child.parent===o?1:0;'],
 ['helper-receiver','const use=x=>x.x===a?1:0;const o={child:{x:a}};return use(o.child);'],
 ['helper-result','const id=x=>x;const o={child:{x:a}};const p=id(o.child);return p.x===a?1:0;'],
 ['returned-reference','const make=x=>()=>x;const o={child:[a,b]};const f=make(o.child);const p=f();return p[0]===a&&p===o.child?1:0;'],
 ['capture','const o={child:{x:a}};const f=()=>o.child.x===a?1:0;const holder={f};return f();'],
 ['factory','const make=n=>({child:{x:n}});const o=make(a);return o.child.x===a?1:0;'],
 ['length','const o={child:[a,b]};return o.child.length;'],
 ['resize','const o=[[a,b]];o[0].length=1;return o[0].length+(o[0][0]===a?1:0);'],
 ['growth-call','const o={child:[()=>a]};o.child.length=3;return o.child[0]();'],
 ['former-function-guard','const o={child:{f:()=>a}};return o.child.f();'],
 ['receiver-order','let n=0;const o={child:{x:a}};(n++,o.child).x=(n++,b);return n===2&&o.child.x===b?1:0;'],
 ['argument-order','let n=0;const o={child:{f:x=>x}};return (n++,o.child).f((n++,a),(n++,b))+n;'],
 ['callee-snapshot','const o={child:{f:x=>x}};return o.child.f((o.child={f:x=>b},a));'],
 ['function-call','let n=0;const o={child:{f:x=>x}};return o.child.f.call((n++,b),(n++,a),(n++,b))+n;'],
 ['callback','const o={child:{f:x=>x}};const apply=(fn,x)=>fn(x);const value={};return apply(o.child.f,value)===value?1:0;'],
 ['factory-call','let n=0;const pick=x=>x;const o={child:{x:a}};return pick.call((n++,b),(n++,o.child)).x===a?n:0;'],
 ['unicode',"const o={'😀':{x:a}};return o['😀'].x===a?1:0;"],
 ['duplicate-key','let n=0;const o={child:(n++,null),child:{x:(n++,a)}};return n===2&&o.child.x===a?1:0;'],
 ['logical-array-and','const left={child:[a]},right={child:[b]};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child[0]===b?1:0;'],
 ['logical-object-or','const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick||=()=>right.child;const o={child:pick()};return o.child.x===a?1:0;'],
 ['logical-object-nullish','const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick??=()=>right.child;const o={child:pick()};return o.child.x===a?1:0;'],
 ['logical-assignment-result','const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;const o={child:(pick&&=()=>right.child)()};return o.child.x===b?1:0;'],
 ['former-missing-field-guard','const o={x:{}};const x=o.x;return x.y?a:b;'],
];
export const nestedContainerReceivers=cases.map(([id,body])=>[`nested-container-${id}`,body,body]);
nestedContainerReceivers.push(['nested-container-module','','',undefined,undefined,{
 sourceJS:'function read(x,n){return x[0]===n?1:0;}export function run(a,b){const o={child:[a,b]};return read(o.child,a);}',
 sourceTS:'function read<T>(x:string,n:boolean):never{return x[0]===n?1:0;}export function run(a:string,b:boolean):void{const o={child:[a,b]};return read<Missing>(o.child as never,a);}',
}]);
nestedContainerReceivers.push(['nested-container-erasure','','',undefined,undefined,{
 sourceJS:'export function run(a,b){const o={child:[{x:a}]};o.child[0].x=b;return o.child[0].x===b?1:0;}',
 sourceTS:'type Unused=Missing;export function run(a:boolean,b:string):void{const o={child:[{x:a}]};(o.child as unknown)[(0 as never)]!.x=b;return(o.child[0]as Unknown).x===b?1:0;}',
}]);
nestedContainerReceivers.push(['nested-container-logical-object-and','','',undefined,undefined,{
 sourceJS:'export function run(a,b){const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;}',
 sourceTS:'export function run(a,b){const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;(pick as unknown)&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;}',
}]);

export const nestedContainerPrimitiveBodies=[
 ['nested-container-undefined','const o={child:{f:x=>x}};return o.child.f(undefined);'],
 ['nested-container-null','const o={child:{f:x=>x}};return o.child.f(null);'],
 ['nested-container-boolean','const o={child:{f:x=>x}};return o.child.f(true);'],
 ['nested-container-number','const o={child:{f:x=>x}};return o.child.f(a);'],
 ['nested-container-general','const o={child:{f:x=>x?null:b}};return o.child.f(a);'],
 ['nested-container-snapshot','const o={child:{f:()=>null}};const p=o.child;const old=p.f;p.f=()=>false;return old();'],
];
