// Strings in the shared WasmGC value carrier; numeric observations only.
const cases = [
  ['managed-string-snapshot', "const o={x:'same'};const saved=o.x;o.x='other';return saved==='same'&&o.x==='other'&&saved!==o.x?a:b;"],
  ['managed-string-truthiness', "const empty='';const zero='0';return empty?b:zero?a:b;"],
  ['managed-string-selected', "const value=a?'':b;return value?a:b;"],
  ['managed-string-kinds', "return ''!==null&&''!==undefined&&'0'!==0&&'false'!==false&&'x'!=={}?a:b;"],
  ['managed-string-utf16', String.raw`const x='\ud800\0';return x==='\ud800\x00'&&x!=='\ufffd\0'?a:b;`],
  ['managed-string-unicode', String.raw`return '\ud83d\ude00'==='\u{1f600}'&&'\u00e9'!=='e\u0301'?a:b;`],
];
export const managedStrings = cases.map(([id, body]) => [id, body, body]);
const helper = "function identity(v){return v;} export function run(a,b){const o={};return identity('x')==='x'&&identity(o)===o&&identity(a)===a?a:b;}";
managedStrings.push(['managed-string-helper', 'return a;', 'return a;', undefined, undefined,
  {sourceJS: helper, sourceTS: helper}]);

// Exact former admission rejections, with explicit erased JS oracles.
export const managedStringPromotions = [
  {
    "id": "statement-string",
    "extension": "js",
    "sourceSha256": "3c4da174eb7511c8b413c8df118039c4125db879323da7feda1b3c3662026ceb",
    "sourceJS": "export function run(a,b) { (\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"x\"); return a; }"
  },
  {
    "id": "statement-asserted-string",
    "extension": "ts",
    "sourceSha256": "06c3609fda021daff6ebff5682c0472e401b8ecb003a07327c55e0bf0af58604",
    "sourceJS": "export function run(a,b) { (\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"x\") satisfies number; return a; }"
  },
  {
    "id": "declarators-later-string",
    "extension": "js",
    "sourceSha256": "3774ff2405fa1891566dd43175e92f8fce87998c39b6e16f0061be83a5fdc0af",
    "sourceJS": "export function run(a,b) { let x=a,y=\"x\"; return x; }",
    "sourceTS": "export function run(a,b) { let x=a,y=\"x\"; return x; }"
  },
  {
    "id": "directive-nonprologue",
    "extension": "js",
    "sourceSha256": "035fea659bf657c903ad75c0dc2300a79154cdac7cb25d348a22ba4af6b08a56",
    "sourceJS": "export function run(a,b) { ; \"hint\"; return a; }",
    "sourceTS": "export function run(a,b) { ; \"hint\"; return a; }"
  },
  {
    "id": "directive-parenthesized",
    "extension": "js",
    "sourceSha256": "21ff8f54238bf46a925eba453ff03cd966ef74399dbe3f5b13f7424d96c17a43",
    "sourceJS": "export function run(a,b) { (\"hint\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"hint\"); return a; }"
  },
  {
    "id": "directive-nested-block",
    "extension": "js",
    "sourceSha256": "88679bbefbc9a522d50ae74defc37ea25add51d9e02736c971099114477e71d5",
    "sourceJS": "export function run(a,b) { { \"hint\"; } return a; }",
    "sourceTS": "export function run(a,b) { { \"hint\"; } return a; }"
  },
  {
    "id": "directive-ts-wrapped",
    "extension": "ts",
    "sourceSha256": "df418ae5526b5e27ff6c319dc815affe615cc6410e5f67db246cdaafec85578d",
    // Keep the erased expression out of the JS directive prologue.
    "sourceJS": "export function run(a,b) { (\"hint\"); return a; }",
    "sourceTS": "export function run(a:number,b:number):number { \"hint\" as string; return a; }"
  },
  {
    "id": "direct-call-unused-unsupported",
    "extension": "js",
    "sourceSha256": "908cd397bcd62589d46868fab6f0763284ce4c63b7533968fd164831344a0f3d",
    "sourceJS": "function unused() { return \"bad\"; } export function run(a,b) { return a; }",
    "sourceTS": "function unused() { return \"bad\"; } export function run(a,b) { return a; }"
  },
  {
    "id": "discarded-string",
    "extension": "js",
    "sourceSha256": "3c4da174eb7511c8b413c8df118039c4125db879323da7feda1b3c3662026ceb",
    "sourceJS": "export function run(a,b) { (\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"x\"); return a; }"
  },
  {
    "id": "discarded-asserted-string",
    "extension": "ts",
    "sourceSha256": "f353a3ac4e130e4c4633ba6c3bd1f3f01d0f48ba5a8f449f7cc5b4406052a6e1",
    "sourceJS": "export function run(a,b) { (\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"x\") as boolean; return a; }"
  },
  {
    "id": "const-boolean-asserted-string",
    "extension": "ts",
    "sourceSha256": "c8a998f49d6f128d3f51e8236910a21618852a0dc33076c0cdd260ea13895414",
    "sourceJS": "export function run(a,b) { const flag=(\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { const flag=(\"x\" as boolean); return a; }"
  },
  {
    "id": "discarded-predicate-binding-reject-string",
    "extension": "ts",
    "sourceSha256": "cc2e7ccdf2600267d4a18735c35124cce6a4eab92c51625dd445f47d025ee836",
    "sourceJS": "export function run(a,b) { (\"x\"); return a; }",
    "sourceTS": "export function run(a,b) { (\"x\" as boolean); return a; }"
  },
  {
    "id": "strict-boolean-equality-reject-asserted-string",
    "extension": "ts",
    "sourceSha256": "b1657a22efde4adc90c7a188c452de6ffc3e1ddd38ae3178fc21995123be2213",
    "sourceJS": "export function run(a,b) { return (\"x\")===true?1:0; }",
    "sourceTS": "export function run(a,b) { return (\"x\" as boolean)===true?1:0; }"
  },
  {
    "id": "predicate-logical-string",
    "extension": "js",
    "sourceSha256": "065e0954a014ea57da82aaf8ab109d84161f386cdbd1161cda25b1821ce474f5",
    "sourceJS": "export function run(a,b) { if(a && \"x\") return a; return b; }",
    "sourceTS": "export function run(a,b) { if(a && \"x\") return a; return b; }"
  },
  {
    "id": "comma-unsupported-3",
    "extension": "js",
    "sourceSha256": "7fb89db8138c039adb129d43d5584256a230c8256fae45b4d255b1bc99455d92",
    "sourceJS": "export function run(a,b) { return (\"4\",a); }",
    "sourceTS": "export function run(a,b) { return (\"4\",a); }"
  }
];
for(const p of managedStringPromotions) managedStrings.push([p.id,'','',undefined,undefined,{sourceJS:p.sourceJS,sourceTS:p.sourceTS}]);

// This valid original program never returns; evidence is resource exhaustion.
export const managedStringFuel = {
  "id": "do-string-test",
  "sourceJS": "export function run(a,b) { do { a=b; } while(\"x\"); return a; }",
  "sourceTS": "export function run(a,b) { do { a=b; } while(\"x\"); return a; }",
  "resourceOutcomes": [
    "fuel-exhausted"
  ],
  "standards": [
    "https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean",
    "https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-do-while-statement"
  ]
};
