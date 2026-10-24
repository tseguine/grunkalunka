// Initialized mutable Number identifiers; nullish/general/const assignment remains outside admission.
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation
export const numericLogicalAssignment = [];
const add = (id, js, ts = js, expected) => numericLogicalAssignment.push([`numeric-logical-assignment-${id}`, js, ts, expected]);
for (const [op, name, start, expectedLoop] of [['&&=', 'and', 1, 33], ['||=', 'or', 0, 11]]) {
  add(`${name}-arguments`, `return a ${op} b;`);
  add(`${name}-local`, `let x=a; return x ${op} b;`);
  add(`${name}-same-target-rhs`, `return a ${op} (a=2,a+1);`);
  add(`${name}-later-sibling`, `return (a ${op} (a=2,a+1))+(a=7);`);
  add(`${name}-selected-result`, `let x=${start}; return (x ${op} (x=2,x+1))+(x=7);`, undefined, 10);
  add(`${name}-captured-local`, `let x=a; const y=(x ${op} b); return (x=7)+y;`);
  add(`${name}-rhs-once`, `let x=${start},n=0; const y=(x ${op} (n++,3)); return n*100+y*10+x;`, undefined, 133);
  add(`${name}-skip-once`, `let x=${1-start},n=0; const y=(x ${op} (n++,3)); return n*100+y*10+x;`, undefined, (1-start)*11);
  add(`${name}-conditional-rhs`, `let x=a; const y=(x ${op} (b?3:4)); return x+y;`);
  add(`${name}-nested`, `return a ${op} (b ${op} 3);`);
  add(`${name}-comma`, `let x=a; return (x ${op} (b=2,b+1),x);`);
  add(`${name}-discard`, `let x=a; x ${op} b; return x;`);
  add(`${name}-predicate`, `return (a ${op} b)?1:2;`);
  add(`${name}-arithmetic`, `return (a ${op} b)+b;`);
  add(`${name}-bitwise`, `return (a ${op} b)&3;`);
  add(`${name}-numeric-bridge`, `return a ${op} +(a<b);`);
  add(`${name}-shadow`, `let x=a; { let x=b; x ${op} 3; } return x ${op} b;`);
  add(`${name}-loop`, `let n=0,x=${start}; for(let i=0;i<3;i++) { x ${op} (n=n+1); } return n*10+x;`, undefined, expectedLoop);
  add(`${name}-for-step`, `let x=${start},n=0; for(let i=0;i<3;(i++,x ${op} (n=n+1))) {} return n*10+x;`, undefined, expectedLoop);
  for (const [kind, value] of [['zero', '0'], ['negative-zero', '-0'], ['nan', 'NaN'], ['infinity', 'Infinity']])
    add(`${name}-${kind}`, `let x=${value}; return x ${op} b;`);
  for (const [kind, target] of [['as', '(a as string)'], ['angle', '(<number>a)'], ['nonnull', '(a!)'], ['satisfies', '(a satisfies unknown)']])
    add(`${name}-target-${kind}`, `return a ${op} b;`, `return ${target} ${op} b;`);
  add(`${name}-rhs-wrapper`, `return a ${op} b;`, `return a ${op} ((b as string) satisfies boolean);`);
  add(`${name}-rhs-depth128`, `return a ${op} ${'('.repeat(127)}b${')'.repeat(127)};`);
  add(`${name}-ts-depth128`, `return a ${op} b;`, `return a ${op} b${'!'.repeat(127)};`);
  numericLogicalAssignment.push([`numeric-logical-assignment-${name}-helper`, '', '', undefined, undefined, {
    sourceJS: `function f(x,y) { return x ${op} (y+1); } export function run(a,b) { return f(a,b); }`,
    sourceTS: `function f(x:number,y:number):number { return x ${op} (y+1); } export function run(a:number,b:number):number { return f(a,b); }`,
  }]);
}
export const numericLogicalAssignmentPromotions = [
  ['unsupported-compound-8', 'export function run(a,b) { return a &&= b; }', 'export function run(a:number,b:number):number { return a &&= b; }', '21deae1a20f4ee62d59fc6e650952f03e48b9afe2af2971741a6fda9c5c16de3'],
  ['unsupported-compound-9', 'export function run(a,b) { return a ||= b; }', 'export function run(a:number,b:number):number { return a ||= b; }', '9c28b7ff2bcad9b5f2d99ceec226f94c347001bb68abcbe47bacb6b3a82af28e'],
];
for (const [id, sourceJS, sourceTS] of numericLogicalAssignmentPromotions)
  numericLogicalAssignment.push([id, '', '', undefined, undefined, { sourceJS, sourceTS }]);
const negatives = [];
const negative = (id, body, diagnostic, group, extension = 'js') => negatives.push([id, body, diagnostic, group, extension]);
for (const [op, name] of [['&&=', 'and'], ['||=', 'or']]) {
  negative(`${name}-const-skip`, `const x=${name==='and'?0:1}; return x ${op} b;`, 'assignment to a const binding is not admitted', 'const-errors');
  negative(`${name}-const-selected`, `const x=${name==='and'?1:0}; return x ${op} b;`, 'assignment to a const binding is not admitted', 'const-errors');
  negative(`${name}-property`, `return a.x ${op} b;`, 'assignment target must be an initialized identifier', 'property-targets');
  negative(`${name}-unresolved`, `return clock ${op} b;`, 'reference is not an initialized numeric parameter or local', 'binding-errors');
  negative(`${name}-self`, `let x=(x ${op} b); return x;`, 'reference is not an initialized numeric parameter or local', 'binding-errors');
  negative(`${name}-forward`, `let x=(y ${op} b); let y=a; return x;`, 'reference is not an initialized numeric parameter or local', 'binding-errors');
  negative(`${name}-shadow-tdz`, `let x=a; { let y=(x ${op} b); let x=b; } return a;`, 'reference is not an initialized numeric parameter or local', 'binding-errors');
  negative(`${name}-scope`, `{let x=a;} return x ${op} b;`, 'reference is not an initialized numeric parameter or local', 'binding-errors');
  negative(`${name}-unselected-call`, `let x=${name==='and'?0:1}; return x ${op} Math.random();`, 'expression is outside the numeric experiment', 'function-calls');
  negative(`${name}-boolean-rhs`, `return a ${op} true;`, 'expression is outside the numeric experiment', 'boolean-values');
  negative(`${name}-string-rhs`, `return a ${op} "x";`, 'expression is outside the numeric experiment', 'string-values');
  negative(`${name}-bigint-rhs`, `return a ${op} 1n;`, 'expression is outside the numeric experiment', 'bigint-values');
  negative(`${name}-js-depth129`, `return a ${op} ${'('.repeat(128)}b${')'.repeat(128)};`, 'expression nesting exceeds', 'expression-depth');
  negative(`${name}-ts-depth129`, `return a ${op} b${'!'.repeat(128)};`, 'expression nesting exceeds', 'expression-depth', 'ts');
}
export const numericLogicalAssignmentRejected = negatives.map(([id, body, diagnostic, , extension]) =>
  [`numeric-logical-assignment-reject-${id}`, `export function run(a,b) { ${body} }`, extension, diagnostic]);
export const numericLogicalAssignmentRejectionGroups = new Map(negatives.map(([id, , , group]) => [`numeric-logical-assignment-reject-${id}`, group]));