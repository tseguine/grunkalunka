// GetValue effects survive discarded results. Boolean storage and returns remain unsupported.
// https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-expression-statement-runtime-semantics-evaluation
// https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-comma-operator-runtime-semantics-evaluation
// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean
const cases = [
  ['literals', 'true; false; return a;'],
  ['comparisons', 'a<b; a<=b; a>b; a>=b; a==b; a!=b; a===b; a!==b; return a;'],
  ['effect-once', 'let x=0; x++<1; return x;'],
  ['negation-effect', 'let x=0; !(x=3); return x;'],
  ['snapshot', 'let x=5; x<(x=3); return x;'],
  ['logical-skip', 'let x=0; false&&(x++<1); true||(x++<1); return x;'],
  ['logical-select', 'let x=0; true&&(x++<1); false||(x++<1); return x;'],
  ['mixed-left-number', 'let x=0; a&&(x++<b); a||(x++<b); return x;'],
  ['mixed-right-number', 'let x=0; (a<b)&&x++; (a>=b)||x++; return x;'],
  ['mixed-conditional', 'let x=0; a?x++:false; b?true:x++; return x;'],
  ['conditional-selected', 'let x=0; true?x++<1:x++<1; false?x++<1:x++<1; return x;'],
  ['nested-logical', 'let x=0; (a && (b || true)) && (x++<1); return x;'],
  ['nested-conditional', 'let x=0; a?(b?x++<1:x++):false; return x;'],
  ['numeric-comma', 'let x=0; return (x++<1,!(x++),x);'],
  ['predicate-comma', 'let x=0; if((x++<1,!x,x)) x+=10; return x;'],
  ['nested-comma', 'let x=0; return ((x++<1,(true,x++)),false,x);'],
  ['comma-result-number', 'let x=0; (true,x++)+(x=3); return x;'],
  ['sequence-shape-last', 'let x=0; (false,x++) && (x++<3); return x;'],
  ['conditional-test-boolean-only', 'let x=0; (true?x++:x++) && (x+=3); return x;'],
  ['for-init', 'let x=0; for(x++<1;x<3;x++) {} return x;'],
  ['for-update', 'let x=0; for(;x<3;!(x++)) {} return x;'],
  ['for-both', 'let x=0; for(true;x<3;x++<3) { a+b; } return x;'],
  ['while-body', 'let x=0; while(x<3) x++<3; return x;'],
  ['do-body', 'let x=0; do !(x++); while(x<3); return x;'],
  ['numeric-shadow', 'let Math=a; true; Math=b; return Math;'],
  ['depth-boundary', `${'('.repeat(128)}true${')'.repeat(128)}; return a;`],
  ['comma-depth-boundary', `return ${'('.repeat(126)}a<b,a${')'.repeat(126)};`],
  ['count-boundary', `${'true;'.repeat(255)}return a;`],
  ['number-asserted-boolean', 'a; return a;', 'a as boolean; return a;'],
  ['boolean-asserted-number', 'true; return a;', 'true as number; return a;'],
  ['wrapped-mixed', 'let x=0; a?(x++<1):false; return x;', 'let x=0; ((a?((x++<1) as number):false) satisfies number)!; return x;'],
];
export const discardedValues = cases.map(([id, js, ts=js]) => [`discarded-${id}`, js, ts]);
discardedValues.push(['discarded-helper-effects','','',undefined,undefined,{
  sourceJS:'function bump(x) { return x+1; } export function run(a,b) { let x=0; bump(x++)<2; true&&bump(x++); return x; }',
  sourceTS:'function bump(x:number):number { return x+1; } export function run(a:number,b:number):number { let x=0; bump(x++)<2; true&&bump(x++); return x; }',
}]);

const expression='expression is outside the numeric experiment';
const binary='only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted';
const reference='reference is not an initialized numeric parameter or local';
const negatives=[
  ['string','("x"); return a;',expression],
  ['asserted-string','("x") as boolean; return a;',expression,'ts'],
  ['object','({}); return a;',expression],
  ['global','clock; return a;',reference],
  ['call','Math.random(); return a;',expression],
  ['number-return','return true;',expression],
  ['number-rhs','a=true; return a;','General bindings require a primitive value position'],
  ['number-argument','return Math.abs(true);',expression],
  ['final-comma-boolean','return (a,true);',expression],
  ['dead-logical-call','false&&Math.random(); return a;',expression],
  ['dead-conditional-call','true?a:Math.random(); return a;',expression],
  ['dead-conditional-boolean-call','true?false:Math.random(); return a;',expression],
  ['const-write','const x=a; false&&(x=b); return a;','assignment to a const'],
  ['tdz','true&&(x<0); let x=a; return a;',reference],
  ['unselected-global','true||clock; return a;',reference],
  ['property-target','false&&(a.x=b); return a;','assignment target must be an initialized identifier'],
  ['nullish','a??true; return a;',expression],
  ['predicate-depth',`${'('.repeat(129)}true${')'.repeat(129)}; return a;`,'expression nesting exceeds'],
  ['comparison-depth',`${'('.repeat(128)}a<b${')'.repeat(128)}; return a;`,'expression nesting exceeds'],
  ['comma-depth-limit',`return ${'('.repeat(127)}a<b,a${')'.repeat(127)};`,'expression nesting exceeds'],
  ['count-limit',`${'true;'.repeat(256)}return a;`,'at most 256 statements'],
  ['after-return','return a; true;','statements after a guaranteed return'],
  ['for-update-dead','for(;false;true&&Math.random()) {} return a;',expression],
];
export const discardedValuesRejected=negatives.map(([id,body,diagnostic,extension='js'])=>[
  `discarded-${id}`,`export function run(a,b) { ${body} }`,extension,diagnostic,
]);
discardedValuesRejected.push(['discarded-unused-helper','function unused(x) { false&&Math.random(); return x; } export function run(a,b) { return a; }','js',expression]);

// Original negative source/mode/identity retained during this promotion.
export const discardedPromotions=[
  {
    "id": "equality-expression-statement",
    "source": "export function run(a,b) { a==b; return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "9a413cccaee1dc4f9c73d8579f0295e5ac6c0dc0bf3e21ef85ae633142d2dfb8",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-boolean",
    "source": "export function run(a,b) { true; return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "db79c8e5dd97798f5a9405cd09a1eb1840e175d91583bfcecff8b054fd03dbae",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-comparison",
    "source": "export function run(a,b) { a<b; return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "6e183dfc094450f74558bb6df857014d947acae6ab9079a02650188871cf7656",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-negation",
    "source": "export function run(a,b) { !a; return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "ee0f3eb7ea217c0c1457098844623b80ff52cb67645832e4f96700ce8e4e7c28",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-comma-predicate",
    "source": "export function run(a,b) { (a<b,a); return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "1dedb3e74899c95352521a9736b57dfa1995314eb7eae575e0e2f83f3c367f25",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-asserted-boolean",
    "source": "export function run(a,b) { true as number; return a; }",
    "extension": "ts",
    "sourceMode": "module-strict",
    "sourceSha256": "c254902c30acc3b1955ee8fe0be86287c00a1ec84758589510c67ed5d85381c5",
    "previousGroup": "boolean-values"
  },
  {
    "id": "statement-asserted-negation",
    "source": "export function run(a,b) { (!a) as number; return a; }",
    "extension": "ts",
    "sourceMode": "module-strict",
    "sourceSha256": "b6c6ac080c46a9549ede680cddcb9ecc39ab280cb2b17007bd2ce4b95171ca49",
    "previousGroup": "boolean-values"
  },
  {
    "id": "predicate-conditional-nonfinal-comma",
    "source": "export function run(a,b) { if((a?true:false,b)) return a; return b; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "7be4e641cc20c1ef9cc10ec8e58395a4451c5ffe21add34f986ba65d5d8816ca",
    "previousGroup": "boolean-values"
  },
  {
    "id": "boolean-comma-nonfinal",
    "source": "export function run(a,b) { if((true,a)) return a; return b; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "50792141834a5857f7131e55f7ec4aa642d2e4f5be0857b03129caec5f8d3804",
    "previousGroup": "boolean-values"
  },
  {
    "id": "predicate-logical-comma-nonfinal",
    "source": "export function run(a,b) { if((a<b,a) && b) return a; return b; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "b34adccb684501f5388c6f0a63f0ba44d16a292f51e287b0484edb7d185685d9",
    "previousGroup": "boolean-values"
  },
  {
    "id": "predicate-logical-statement",
    "source": "export function run(a,b) { true && a++; return a; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "558ef32470c6698fa7ea247b884a689d9efca4702cf6c2bca74c565dc01526b5",
    "previousGroup": "boolean-values"
  },
  {
    "id": "comma-nonfinal-predicate",
    "source": "export function run(a,b) { if((a<b,a)) return a; return b; }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "94ca296198789fea9709218eeb10c6fb96c7eb52be2349778023be779942ee41",
    "previousGroup": "boolean-values"
  },
  {
    "id": "comma-unsupported-7",
    "source": "export function run(a,b) { return (a<b,a); }",
    "extension": "js",
    "sourceMode": "module-strict",
    "sourceSha256": "2b1d7d8b00083e30a22feeeddd1dd7a74573f3228c1e40c4a8daa647a0ad64ba",
    "previousGroup": "boolean-values"
  }
];
const promotedJS={
  'statement-asserted-boolean':'export function run(a,b) { true; return a; }',
  'statement-asserted-negation':'export function run(a,b) { (!a); return a; }',
};
for(const ref of discardedPromotions){const js=ref.extension==='ts'?promotedJS[ref.id]:ref.source;discardedValues.push([ref.id,'','',undefined,undefined,{sourceJS:js,sourceTS:ref.source}]);}
