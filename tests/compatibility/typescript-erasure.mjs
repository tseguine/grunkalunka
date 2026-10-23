// Hand-paired JS references for runtime-transparent TypeScript expressions.
// Assertions/non-null: https://www.typescriptlang.org/docs/handbook/2/everyday-types.html
// Satisfies: https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-9.html
// No TypeScript checker is run; wrapper types never justify numeric admission.
export const typescriptErasurePrograms = [
  ['ts-erasure-as', 'return a;', 'return a as number;'],
  ['ts-erasure-angle', 'return a;', 'return <number>a;'],
  ['ts-erasure-non-null', 'return a;', 'return a!;'],
  ['ts-erasure-satisfies', 'return a;', 'return a satisfies number;'],
  ['ts-erasure-nested', 'return a;', 'return ((<number>((a as number)!)) satisfies number);'],
  ['ts-erasure-no-conversion', 'return a ? a : b;', 'return (a as unknown as string) ? a : b;'],
  ['ts-erasure-ordered-writes', 'return (a = a + b) - (b = a - b);',
    'return ((a = a + b) as number) - <number>(b = a - b);'],
  ['ts-erasure-snapshot', 'return a + (a = b);', 'return a + ((a = b) satisfies number);'],
  ['ts-erasure-local', 'const x = a + b; return x;', 'const x = (a + b) as number; return x!;'],
  ['ts-erasure-signed-zero', 'return -0;', 'return (-0 as number)!;'],
  ['ts-erasure-nan-truthiness', 'return (a / b) ? a : -b;', 'return ((a / b) as number) ? a : -b;'],
  ['ts-erasure-as-comparison', 'return a < b ? -0 : b;', 'return (a < b) as boolean ? -0 : b;'],
  ['ts-erasure-angle-comparison', 'return a < b ? -0 : b;', 'return <boolean>(a < b) ? -0 : b;'],
  ['ts-erasure-non-null-comparison', 'return a < b ? -0 : b;', 'return (a < b)! ? -0 : b;'],
  ['ts-erasure-satisfies-comparison', 'return a < b ? -0 : b;', 'return (a < b) satisfies boolean ? -0 : b;'],
  ['ts-erasure-if', 'if (a === a) return a; return b;', 'if ((a === a) as boolean) return a; return b;'],
  ['ts-erasure-loop-condition', 'let i = 0; let x = a; while ((i = i + 1) < 3) { x = x + b; } return x + i;',
    'let i = 0; let x = a; while (<boolean>((i = i + 1) < 3)) { x = x + b; } return x + i;'],
  ['ts-erasure-predicate-effect', 'if (a = b) return a; return -a;', 'if ((a = b)!) return a; return -a;'],
  ['ts-erasure-nested-predicate', 'return a !== a ? -0 : b;',
    'return (((<boolean>((a !== a) as boolean))!) satisfies boolean) ? -0 : b;'],
  ['ts-erasure-compound-snapshot', 'return (a += (a = b)) - (a = b);',
    'return ((a += (a = b)) as number) - (a = b);'],
  ['ts-erasure-post-update', 'return (a++) + a;', 'return (a++)! + a;'],
  ['ts-erasure-pre-update', 'return (++a) + a;', 'return ((++a) satisfies number) + a;'],
  ['ts-erasure-angle-update', 'return (a--) - a;', 'return <number>(a--) - a;'],
];

const expression = 'expression is outside the numeric experiment';
const binary = 'only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const reference = 'reference is not an initialized numeric parameter or local';
const source = body => `export function run(a:number,b:number):number { ${body} }`;
const reject = (id, body, diagnostic) => [`ts-erasure-${id}`, source(body), 'ts', diagnostic];
export const typescriptErasureRejected = [];

const forms = [
  ['as', value => `(${value}) as number`, count => `a${' as number'.repeat(count)}`],
  ['angle', value => `<number>(${value})`, count => `${'<number>'.repeat(count)}a`],
  ['non-null', value => `(${value})!`, count => `a${'!'.repeat(count)}`],
  ['satisfies', value => `(${value}) satisfies number`, count => `a${' satisfies number'.repeat(count)}`],
];
for (const [name, wrap, nested] of forms) {
  // Each erased wrapper counts once, with no synthetic parentheses at this boundary.
  typescriptErasurePrograms.push([`ts-erasure-${name}-depth-boundary`, 'return a;', `return ${nested(128)};`]);
  typescriptErasureRejected.push(reject(`${name}-depth-limit`, `return ${nested(129)};`, 'expression nesting exceeds'));
  for (const [kind, inner] of [['call', 'Math.random()'], ['string', '"4"'], ['object', '{x:a}']]) {
    typescriptErasureRejected.push(reject(`${name}-${kind}`, `return ${wrap(inner)};`, expression));
  }
}
typescriptErasurePrograms.push(['ts-erasure-predicate-depth-boundary', 'if (a) return a; return b;',
  `if (a${'!'.repeat(128)}) return a; return b;`]);
typescriptErasureRejected.push(
  reject('predicate-depth-limit', `if (a${'!'.repeat(129)}) return a; return b;`, 'predicate nesting exceeds'),
  reject('const-write', 'const x = a; return (x = b) as number;', 'assignment to a const binding is not admitted'),
  reject('self-initializer', 'const x = x as number; return x;', reference),
  reject('forward-initializer', 'const x = y!; const y = a; return x;', reference),
  reject('unselected-call', 'return a === a ? a : (Math.random() as number);', expression),
  reject('boolean-return', 'return (a < b) as boolean;', binary),
  reject('boolean-arithmetic', 'return ((a < b) as boolean) + b;', binary),
);
