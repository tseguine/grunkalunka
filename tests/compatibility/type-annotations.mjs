// Annotation syntax remains on Oxc's AST as unvalidated metadata, not runtime proof.
export const annotationPrograms = [];
function add(id, bodyJS, bodyTS = bodyJS, parameters = 'a:unknown,b:unknown', result = 'unknown') {
  const sourceJS = `export function run(a,b) { ${bodyJS} }`;
  const sourceTS = `export function run(${parameters})${result ? `:${result}` : ''} { ${bodyTS} }`;
  annotationPrograms.push([id, bodyJS, bodyTS, undefined, undefined, { sourceJS, sourceTS }]);
}
add('annotation-promoted-string-parameter', 'return a;', 'return a;', 'a:string,b:number', 'number');
add('annotation-promoted-return', 'return a;', 'return a;', 'a:number,b:number', 'string');
add('annotation-promoted-local', 'const x=a; return x;', 'const x:string=a; return x;', 'a:number,b:number', 'number');
add('annotation-promoted-later-declarator', 'let x=a,y=b; return x;', 'let x:number=a,y:string=b; return x;', 'a,b', '');
add('annotation-unknown', 'return a+b;');
add('annotation-any-never', 'return a-b;', 'return a-b;', 'a:any,b:never', 'never');
add('annotation-unresolved-names', 'const x=a+b; return x;', 'const x:MissingResult=a+b; return x;',
  'a:MissingInput<number>,b:OtherType', 'MissingOutput');
add('annotation-union-intersection', 'return a*b;', 'return a*b;',
  'a:number|string,b:number & { readonly brand: "value" }', 'string|boolean');
add('annotation-object-tuple-function', 'const x=a+b; return x;',
  'const x: (n: number) => string=a+b; return x;', 'a:{ value:number },b:readonly [number,string]', 'number[]');
add('annotation-type-query', 'const x=a; const y=b; return x+y;',
  'const x:typeof y=a; const y:typeof a=b; return x+y;', 'a:typeof b,b:typeof a', 'typeof a');
add('annotation-unresolved-type-query', 'const x=a; return x;', 'const x:typeof MissingValue=a; return x;');
add('annotation-type-operators', 'return a+b;', 'return a+b;',
  'a:keyof { x:number },b:{ x:number }["x"]', 'number extends string ? never : number');
add('annotation-mapped-return', 'return a;', 'return a;', 'a:unknown,b:unknown', '{ readonly [K in "a"|"b"]?:number }');
add('annotation-predicate-return', 'return a;', 'return a;', 'a:unknown,b:unknown', 'a is number');
add('annotation-assertion-return', 'return a;', 'return a;', 'a:unknown,b:unknown', 'asserts a is number');
add('annotation-shadow-and-effects', 'let x=a; { let x=b; a=x++; } return x+a;',
  'let x:string=a; { let x:never=b; a=x++; } return x+a;');
add('annotation-for-bindings', 'let x=a; for(let i=0,j=2;i<3;i++) { x+=j+b; } return x;',
  'let x:string=a; for(let i:never=0,j:string=2;i<3;i++) { x+=j+b; } return x;');
add('annotation-wrappers', 'const x=a+b; return x;', 'const x:string=(a+b) as unknown as string; return x satisfies never;');
add('annotation-unsigned-shift-snapshot', 'return a >>> (a=b);', 'return a >>> (a=b);', 'a:unknown,b:number|string');
add('annotation-expression-depth', `return ${'('.repeat(128)}a${')'.repeat(128)};`);
add('annotation-statement-budget', ';'.repeat(255) + 'return a;');
add('annotation-local-budget', `const ${Array.from({length:128},(_,i)=>`x${i}=a`).join(',')}; return a;`,
  `const ${Array.from({length:128},(_,i)=>`x${i}:unknown=a`).join(',')}; return a;`);

function byteLimitSource(extra = 0) {
  const prefix = 'export function run(a:{ "';
  const suffix = '":number },b:unknown):unknown { return a; }';
  return prefix + 'λ' + 'x'.repeat(16_384 + extra - Buffer.byteLength(prefix + 'λ' + suffix)) + suffix;
}
annotationPrograms.push(['annotation-original-byte-limit', 'return a;', 'return a;', undefined, undefined,
  { sourceJS: 'export function run(a,b) { return a; }', sourceTS: byteLimitSource() }]);

const source = body => `export function run(a:unknown,b:unknown):unknown { ${body} }`;
const expression = 'expression is outside the numeric experiment';
const reference = 'reference is not an initialized numeric parameter or local';
export const annotationRejected = [
  ...[['string', '"4"'], ['object', '{}'], ['boolean', 'true'], ['null', 'null'], ['call', 'Math.random()']]
    .map(([id, value]) => [`annotation-runtime-${id}`, source(`const x:number=${value}; return x;`), 'ts', id === 'boolean' ? 'ERROR: unsupported at bytes 79..80: Boolean bindings are not admitted in numeric value positions' : id === 'null' ? 'ERROR: unsupported at bytes 79..80: General bindings require a primitive value position' : expression]),
  ['annotation-self-reference', source('const x:typeof a=x; return x;'), 'ts', reference],
  ['annotation-forward-reference', source('const x:typeof y=y; const y:number=a; return x;'), 'ts', reference],
  ['annotation-const-write', source('const x:unknown=a; x=b; return x;'), 'ts', 'assignment to a const binding'],
  ['annotation-definite', source('let x!:unknown; return a;'), 'ts', 'definite-assignment assertions are not admitted'],
  ['annotation-optional-parameter', 'export function run(a?:unknown,b?:unknown):unknown { return a; }', 'ts',
    'parameter modifiers, defaults and decorators are not admitted'],
  ['annotation-default-parameter', 'export function run(a:unknown=1,b:unknown=2):unknown { return a; }', 'ts',
    'parameter modifiers, defaults and decorators are not admitted'],
  ['annotation-rest-parameter', 'export function run(a:unknown,...b:unknown[]):unknown { return a; }', 'ts',
    'exactly two ordinary numeric parameters'],
  ['annotation-arity', 'export function run(a:unknown):unknown { return a; }', 'ts', 'exactly two ordinary numeric parameters'],
  ['annotation-malformed-type', 'export function run(a:number|,b:unknown):unknown { return a; }', 'ts', 'parse diagnostics'],
  ['annotation-duplicate-parameter', 'export function run(a:unknown,a:unknown):unknown { return a; }', 'ts'],
  ['annotation-unselected-call', source('if(false) { const x:number=Math.random(); } return a;'), 'ts', expression],
  ['annotation-expression-limit', source(`return ${'('.repeat(129)}a${')'.repeat(129)};`), 'ts', 'expression nesting exceeds'],
  ['annotation-statement-limit', source(';'.repeat(256) + 'return a;'), 'ts', 'at most 256 statements'],
  ['annotation-local-limit', source(`const ${Array.from({length:129},(_,i)=>`x${i}:unknown=a`).join(',')}; return a;`), 'ts',
    'at most 128 source locals'],
  ['annotation-original-byte-overflow', byteLimitSource(1), 'ts', '16 KiB'],
];
