// The JavaScript oracle for a TypeScript source erases only its local type declarations.
const cases = [
  ["gap-unreachable-after-return", "export function run(a,b) { return a; a=b; }"],
  ["unreachable-bare-return", "export function run(a,b){return a;return;}"],
  ["unreachable-numeric-throw", "export function run(a,b){return a;throw b;}"],
  ["do-after-guaranteed-branches", "export function run(a,b) { do { if(a<b) return a; else return b; } while(0); a=b; }"],
  ["break-after-direct", "export function run(a,b) { while(1) { break; a=b; } return a; }"],
  ["break-after-mixed", "export function run(a,b) { while(1) { if(a) break; else return b; a=b; } return a; }"],
  ["continue-after-direct", "export function run(a,b) { while(0) { continue; a=b; } return a; }"],
  ["continue-after-mixed-break", "export function run(a,b) { while(0) { if(a) continue; else break; a=b; } return a; }"],
  ["continue-after-mixed-return", "export function run(a,b) { while(0) { if(a) continue; else return b; a=b; } return a; }"],
  ["type-declaration-runtime-after-return", "export function run(a,b) { return a; a=b; }",
    "export function run(a,b) { return a; type T=number; a=b; }"],
  ["type-declaration-runtime-after-break", "export function run(a,b) { do { break; a=b; } while(0); return a; }",
    "export function run(a,b) { do { break; interface I {} a=b; } while(0); return a; }"],
  ["type-declaration-runtime-after-continue", "export function run(a,b) { for(let i=0;i<1;i++) { continue; a=b; } return a; }",
    "export function run(a,b) { for(let i=0;i<1;i++) { continue; type T=number; a=b; } return a; }"],
  ["type-declaration-runtime-after-mixed", "export function run(a,b) { do { if(a) return a; else break; a=b; } while(0); return b; }",
    "export function run(a,b) { do { if(a) return a; else break; type T=number; a=b; } while(0); return b; }"],
  ["block-unreachable", "export function run(a,b) { { return a; b=a; } }"],
  ["dead-store-after-debugger", "export function run(a,b){return a; true; ;debugger; a=b;}"],
  ["dead-store-after-break-interface", "export function run(a,b) { do { break; a=b; } while(0); return a; }",
    "export function run(a,b) { do { break; interface T {} a=b; } while(0); return a; }"],
  ["dead-store-after-return-types", "export function run(a,b) { return a; a=b; }",
    "export function run(a,b) { return a; type T=number; interface I {} a=b; }"],
  ["dead-store-after-break-types", "export function run(a,b) { do { break; a=b; } while(0); return a; }",
    "export function run(a,b) { do { break; type T=number; interface I {} a=b; } while(0); return a; }"],
  ["dead-store-after-continue-types", "export function run(a,b) { for(let i=0;i<1;i++) { continue; a=b; } return a; }",
    "export function run(a,b) { for(let i=0;i<1;i++) { continue; type T=number; interface I {} a=b; } return a; }"],
  ["dead-store-finally-observes-original", "export function run(a,b){try{return a;a=b;}finally{return a;}}"],
  ["dead-store-break-observes-original", "export function run(a,b){let x=a;while(true){break;x=b;}return x;}"],
];
export const unreachableBareReturnPromotion = {
  id: 'unreachable-bare-return',
  source: 'export function run(a,b){return a;return;}',
  sha256: 'ac78e528be69edad7a95abd2f1202cc000cbf34e4ad6d739d8d12338e61d573a',
};
export const unreachableNumericThrowPromotion = {
  id: 'unreachable-numeric-throw',
  source: 'export function run(a,b){return a;throw b;}',
  sha256: 'd3ccd98bf85e89418efd456281784383444401d4b1147897e4fbf8537997fe39',
};
export const unreachableCopyPrograms = cases.map(([id, sourceJS, sourceTS = sourceJS]) => [
  id, '', '', undefined, undefined, { sourceJS, sourceTS },
]);
