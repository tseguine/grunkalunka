const cases = [
  ['unreachable-const-list', 'export function run(a,b){return a;const x=b,y=x+1;return y;}'],
  ['unreachable-const-chain', 'export function run(a,b){return a;const x=b;const y=x*2;return y;}'],
  ['unreachable-let-write', 'export function run(a,b){return a;let x=b;x=x+1;return x;}'],
  ['unreachable-bare-let', 'export function run(a,b){return a;let x;return x;}'],
  ['unreachable-typed-list', 'export function run(a,b){return a;const x=b,y=x+1;return y;}',
    'export function run(a:number,b:number):number{return a;const x:number=b,y:number=x+1;return y;}'],
];

export const unreachableBareLetPromotion = {
  id: 'unreachable-bare-let',
  source: 'export function run(a,b){return a;let x;return x;}',
  sha256: '545e074d7adc52a659e3055d39611e043df54b5c87a35b730e6147d2476cbd00',
};

export const unreachableDeclarationPrograms = cases.map(([id, sourceJS, sourceTS = sourceJS]) => [
  id, '', '', undefined, undefined, { sourceJS, sourceTS },
]);
