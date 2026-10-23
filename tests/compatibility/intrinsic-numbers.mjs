// Only unresolved reads of these two immutable primitive globals become constants.
// https://tc39.es/ecma262/multipage/global-object.html#sec-value-properties-of-the-global-object
const cases = [
  ['intrinsic-infinity-promoted-reference', 'return Infinity;'],
  ['intrinsic-nan-promoted-reference', 'return NaN;'],
  ['intrinsic-negative-infinity', 'return -Infinity;'],
  ['intrinsic-infinity-signed-zero', 'return a/Infinity;'],
  ['intrinsic-nan-arithmetic', 'return NaN+a;'],
  ['intrinsic-truthiness', 'let x=0; if(Infinity) x++; if(NaN) x+=10; return a+x;'],
  ['intrinsic-comparisons', 'return NaN==a?1:(Infinity===a?2:3);'],
  ['intrinsic-operand-order', 'return (a=Infinity)+(a=b);'],
  ['intrinsic-logical-effects', 'return (NaN&&(a=b)) || (Infinity&&(a+=1));'],
  ['intrinsic-mutable-infinity', 'let Infinity=a; Infinity+=b; return Infinity;'],
  ['intrinsic-mutable-nan', 'let NaN=a; return NaN++ + NaN;'],
  ['intrinsic-const-shadows', 'const Infinity=a; const NaN=b; return Infinity+NaN;'],
  ['intrinsic-infinity-scope-restoration',
    'let x=a; { let Infinity=b; Infinity+=a; x=Infinity; } return Infinity===1e999?x:-0;'],
  ['intrinsic-nan-scope-restoration',
    'let x=a; { let NaN=b; NaN++; x=NaN; } return NaN!==NaN?x:-0;'],
  ['intrinsic-for-header-shadow',
    'let x=a; for(let Infinity=0;Infinity<3;Infinity++) x+=b; return Infinity===1e999?x:0;'],
  ['intrinsic-earlier-declarator-shadows',
    'let Infinity=a,NaN=Infinity+b; Infinity+=NaN; NaN++; return Infinity+NaN;'],
  ['intrinsic-depth-budget', `return ${'('.repeat(128)}Infinity${')'.repeat(128)};`],
  ['intrinsic-escaped-name', String.raw`return \u0049nfinity;`],
];
export const intrinsicNumberPrograms = cases.map(([id, body]) => [id, body, body]);
intrinsicNumberPrograms.push(['intrinsic-typescript-wrappers', 'return Infinity+NaN;',
  'return ((Infinity as string)!) + (<number>(NaN satisfies number));']);

const reference = 'reference is not an initialized numeric parameter or local';
const negatives = [];
for (const name of ['Infinity', 'NaN']) {
  const id = name.toLowerCase();
  negatives.push(
    [`intrinsic-${id}-self-tdz`, `let ${name}=${name}; return ${name};`, reference],
    [`intrinsic-${id}-forward-tdz`, `const x=${name}; let ${name}=a; return x;`, reference],
    [`intrinsic-${id}-nested-self-tdz`, `let ${name}=a; { let ${name}=${name}; } return a;`, reference],
    [`intrinsic-${id}-unselected-tdz`, `const x=0?${name}:a; let ${name}=b; return x;`, reference],
    [`intrinsic-${id}-later-declarator-tdz`, `let x=${name},${name}=a; return x;`, reference],
    [`intrinsic-${id}-unselected-later-declarator`, `let x=0?${name}:a,${name}=b; return x;`, reference],
    [`intrinsic-${id}-for-later-declarator`, `for(let x=${name},${name}=0;0;) {} return a;`, reference],
    [`intrinsic-${id}-write`, `return ${name}=a;`, reference],
    [`intrinsic-${id}-compound-write`, `return ${name}+=a;`, reference],
    [`intrinsic-${id}-prefix-write`, `return ++${name};`, reference],
    [`intrinsic-${id}-postfix-write`, `return ${name}--;`, reference],
    [`intrinsic-${id}-unselected-write`, `return 0?(${name}=a):b;`, reference],
  );
}
negatives.push(
  ['intrinsic-const-infinity-write', 'const Infinity=a; return Infinity=b;',
    'assignment to a const binding is not admitted'],
  ['intrinsic-const-nan-update', 'const NaN=a; return NaN++;', 'update to a const binding is not admitted'],
  ['intrinsic-inexact-infinity', 'return infinity;', reference],
  ['intrinsic-inexact-nan', 'return nan;', reference],
  ['intrinsic-global-object-property', 'return globalThis.Infinity;', 'expression is outside the numeric experiment'],
  ['intrinsic-number-property', 'return Number.NaN;', 'expression is outside the numeric experiment'],
  ['intrinsic-unselected-unknown-global', 'return Infinity?a:clock;', reference],
  ['intrinsic-depth-limit', `return ${'('.repeat(129)}Infinity${')'.repeat(129)};`, 'expression nesting exceeds'],
);
export const intrinsicNumberRejected = negatives.map(([id, body, diagnostic]) => [id,
  `export function run(a,b) { ${body} }`, 'js', diagnostic]);
