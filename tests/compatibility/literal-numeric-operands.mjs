// Keep the exact former rejection text in the native-JS/Wasm corpus.
export const literalNumericOrigins = [
  ["bit-not-string", "return ~\"4\";",
    "4a58f7959fe1ffd74f8767c95e833171dd7b4c8bbe343f20c621bdbc4d346ce7"],
  ["bitwise-string-left", "return \"5\" & a;",
    "a30107d50e81db5cd130e58724acfcfbc2a4405d99597ba0801fca5ff4716973"],
  ["bitwise-string-right", "return a | \"3\";",
    "7960476a6076db774895a13375eeb78b1057d9364a7ecb8ec716933c3904925b"],
  ["bitwise-unselected-string", "return 1 || (a ^ \"4\");",
    "d3a3528eb38e71a294471e5cf2b9dee3d1f6c0e25d43e916b98c7dfb17804135"],
  ["shift-string-left", "return \"5\" << a;",
    "40d99b2ffd84e261ba8380e1fae22ff086675829c43ecb08f75c43b418e5af2a"],
  ["shift-string-right", "return a >> \"3\";",
    "9064e62771a098959b06295841a7c9bd7e276b613a9938452df9b22061cc9d19"],
  ["shift-unselected-string", "return 1 || (a >>> \"4\");",
    "681c57aeec3887f32bba64e2616514776a144e7735f7b0b7a31b4792cbd28e1b"],
  ["compound-string-rhs", "return a|=\"3\";",
    "1739d5d5eeb567e98f644fc237c4e32bd1629423c907d09b892fdddc6a4a01da"],
  ["math-abs-string", "return Math.abs(\"3\");",
    "9e57a5545efc92a1fc86bb1b9e51e7b08308c1d7d979f88ddb0775ccb6b94a34"],
  ["math-unary-sqrt-string", "return Math.sqrt(\"3\");",
    "95e06a92558683372574350beb18769543accaef667db8e194e01c7987530950"],
  ["math-unary-floor-string", "return Math.floor(\"3\");",
    "bec9c5da2f9fd0f6ffbc613764a76d13cc9ca6d054b71fa1b27ab5b846f3afc8"],
  ["math-unary-ceil-string", "return Math.ceil(\"3\");",
    "d8ff10397b2dd950820c48f44132334efff7a605086f9410fab4c9d2e920767f"],
  ["math-unary-trunc-string", "return Math.trunc(\"3\");",
    "6f870a11a9581c0c74a616defaa1911b0c583a8ffb9302e1f3d66dc2b4dcbadb"],
  ["boolean-numeric-unary-reject-string", "return +\"1\";",
    "7a8d7aa7a1eaf36cb075292b42c90dfbeb09b64e20b8f8965654ebab02dcc075"],
];
export const literalNumericPrograms = literalNumericOrigins.map(([id, body]) => [
  id, body, body, undefined, undefined, {
    sourceJS: `export function run(a,b) { ${body} }`,
    sourceTS: `export function run(a:number,b:number):number { ${body} }`,
  },
]);
for (const [id, source] of [
  ['literal-number-remainder-original', 'export function run(a,b) { return a%"3"; }'],
  ['literal-number-selected-original', "export function run(a,b){return Number(a?'3':'4');}"],
  ['literal-number-unary-original', 'export function run(a,b) {return +"1";}'],
]) {
  literalNumericPrograms.push([id, '', '', undefined, undefined, { sourceJS: source, sourceTS: source }]);
}
literalNumericPrograms.push(['literal-number-math-type-argument-original', '', '', undefined, undefined, {
  sourceJS: "export function run(a,b){return Math.abs('1');}",
  sourceTS: "export function run(a,b){return Math.abs<never>('1');}",
}]);
