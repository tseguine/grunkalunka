// Selected operands retain their value kind; observations use the numeric ABI.
const cases = [
  ['managed-logical-empty-or', "const x='';const y=x||'fallback';return y==='fallback'?a:b;"],
  ['managed-logical-text-and', "const x='left';const y=x&&'right';return y==='right'?a:b;"],
  ['managed-logical-empty-and', "const x='';const y=x&&'right';return y===x?a:b;"],
  ['managed-logical-reference-or', 'const x={v:a};const y=x||{v:b};return y===x?a:b;'],
  ['managed-logical-reference-and', 'const x={v:a};const other={v:b};const y=x&&other;return y===other?a:b;'],
  ['managed-logical-null-default', "const x={v:null};const y=x.v??'fallback';return y==='fallback'?a:b;"],
  ['managed-logical-missing-default', "const x={};const y=x.v??'fallback';return y==='fallback'?a:b;"],
  ['managed-logical-false-retained', "const x={v:false};const y=x.v??'fallback';return y===false?a:b;"],
  ['managed-logical-empty-retained', "const x={v:''};const y=x.v??'fallback';return y===''?a:b;"],
  ['managed-logical-number-retained', "const x={v:a};const y=x.v??'fallback';return (a!==a?y!==y:y===a)?a:b;"],
  ['managed-logical-rhs-once', "let n=0;const x={v:a?null:'v'};const y=(n++,x.v)??(n+=10,'f');return (n===11&&y==='f')||(n===1&&y==='v')?a:b;"],
  ['managed-logical-left-snapshot', "let n=0;let x='left';const old=x;const y=(n++,x)||(n+=10,x='right');return n===1&&y===old&&x===old?a:b;"],
  ['managed-logical-property-rhs', "const x={v:''};const y=x.v||(x.v='changed','fallback');return y==='fallback'&&x.v==='changed'?a:b;"],
  ['managed-logical-discard-default', "let n=0;const x={v:a};x.v??(n++,'yes');return n;"],
];
export const managedLogical = cases.map(([id, body]) => [id, body, body]);
const helperJS="function choose(x,y){return x??y;}export function run(a,b){const ref={};const x=choose(a?'':null,ref);return a?(x===''?a:b):(x===ref?a:b);}";
const helperTS="function choose(x:number,y:number):number{return (x as number)??(y as number);}export function run(a:number,b:number):number{const ref={};const x=choose(a?'':null,ref);return a?(x===''?a:b):(x===ref?a:b);}";
managedLogical.push(['managed-logical-helper-erasure', 'return a;', 'return a;', undefined, undefined,
  {sourceJS: helperJS, sourceTS: helperTS}]);
const call="function choose(x,y){return x??y;}export function run(a,b){const ref={};let n=0;const x=choose.call(n++,null,ref,n++);return n===2&&x===ref?a:b;}";
managedLogical.push(['managed-logical-call-effects', 'return a;', 'return a;', undefined, undefined,
  {sourceJS: call, sourceTS: call}]);
const forwarded="function flag(x){return !x;}function choose(x,y){return x??y;}export function run(a,b){const x=choose(flag(a),{});return x===!a?a:b;}";
managedLogical.push(['managed-logical-primitive-forward', 'return a;', 'return a;', undefined, undefined,
  {sourceJS: forwarded, sourceTS: forwarded}]);

const forwardedNull="function id(x){return x;}export function run(a,b){const o={v:id(null)};const x=o.v??'fallback';return x==='fallback'?a:b;}";
managedLogical.push(['managed-logical-primitive-null', 'return a;', 'return a;', undefined, undefined,
  {sourceJS: forwardedNull, sourceTS: forwardedNull}]);

managedLogical.push(
  ['managed-logical-array-and', "const x=[];const y=x&&'yes';return y==='yes'?a:b;", "const x=[];const y=x&&'yes';return y==='yes'?a:b;"],
  ['managed-logical-array-coalesce', 'const x=[];const y=x??[b];return y===x?a:b;', 'const x=[];const y=x??[b];return y===x?a:b;'],
  ['managed-logical-array-element', "const x=[a];const y=x[0]||'fallback';return (a?y===a:y==='fallback')?a:b;", "const x=[a];const y=x[0]||'fallback';return (a?y===a:y==='fallback')?a:b;"],
);
