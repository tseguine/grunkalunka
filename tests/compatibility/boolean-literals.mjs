// Boolean literals are admitted as predicates; source slots and returns stay numeric.
const cases = [
  ['boolean-true-if', 'if(true) return a; return b;'],
  ['boolean-false-if', 'if(false) return a; return b;'],
  ['boolean-true-ternary', 'return true ? a : b;'],
  ['boolean-false-ternary', 'return false ? a : b;'],
  ['boolean-nested-numeric', 'return a ? (true ? a : b) : (false ? b : a);'],
  ['boolean-true-write', 'if(true) { a+=b; } else { a-=b; } return a;'],
  ['boolean-false-write', 'if(false) { a+=b; } else { a-=b; } return a;'],
  ['boolean-while-false', 'while(false) { return a; } return b;'],
  ['boolean-while-true-return', 'while(true) { return a; } return b;'],
  ['boolean-for-true-return', 'for(;true;) { return a; } return b;'],
  ['boolean-for-false', 'for(;false;) { return a; } return b;'],
  ['boolean-comma-true', 'return (a=b,true) ? a : b;'],
  ['boolean-comma-false', 'return (a=b,false) ? a : -b;'],
  ['boolean-depth-boundary', `if(${'('.repeat(128)}true${')'.repeat(128)}) return a; return b;`],
];

export const booleanLiterals = cases.map(([id, body]) => [id, body, body]);
booleanLiterals.push(
  ['ts-boolean-as', 'if(true) return a; return b;', 'if(true as boolean) return a; return b;'],
  ['ts-boolean-angle', 'return false ? a : b;', 'return <boolean>false ? a : b;'],
);

for (const [id, body] of [
  ['while-only-return', 'while (1) { return a; }'],
  ['gap-while-guaranteed-return', 'while(1) { return a; }'],
]) {
  booleanLiterals.push([id, body, body, undefined, undefined, {
    sourceJS: `export function run(a,b) { ${body} }`,
    sourceTS: `export function run(a:number,b:number):number { ${body} }`,
  }]);
}
booleanLiterals.push(
  ['while-terminal-branch', 'if(a){while(true){if(b)return a+b;else return a;}}else{while(2)return b;}',
    'if(a){while(true as boolean){if(b)return a+b;else return a;}}else{while((2))return b; type T=number;}'],
  ['while-nested-fallback', '{if(a){while(1){return b;}}} return a;',
    '{if(a){while(1){return b;}}} return a;'],
);
