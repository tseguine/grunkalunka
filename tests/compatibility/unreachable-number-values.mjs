// These exact former rejection sources retain native JavaScript as their oracle.
export const unreachableNumberPromotions = [
  ['do-after-guaranteed-return', 'export function run(a,b) { do { return a; } while(0); return b; }', 'e5bda55b05df6538bc3076376efe7a80116f9cf88954d61aa65c11705b91c14d'],
  ['statement-after-return', 'export function run(a,b) { return a; a+b; }', '22c7bceb42e6d9e7e71e6bb5c592c5efe96f16aa48e0f63558f584b0544c505a'],
  ['break-after-block', 'export function run(a,b) { while(1) { { break; } return a; } return b; }', '84dbd2dca2f0b7efa9230ccc27e214146b267a9cc807381c1c8d4c96bdcbbb50'],
  ['break-after-both-arms', 'export function run(a,b) { while(1) { if(a) break; else break; return a; } return b; }', 'ddf203b6c6e93fc2a0529b68fc382b1e15e44b5602a089bc397235a1b5ebf653'],
  ['continue-after-block', 'export function run(a,b) { while(0) { { continue; } return a; } return b; }', 'fec04ceee61ee59abf2e48d7f6b0c28fd8371a1e4878a270627392736dcbce77'],
  ['continue-after-both-arms', 'export function run(a,b) { while(0) { if(a) continue; else continue; return a; } return b; }', '4fb2db2805c1446ae6b02c79480b3cb7674996ad05a1bd7fdb3e8d9e60a838cc'],
  ['if-following-total-return', 'export function run(a,b) { if(a<b) return a; else return b; return 0; }', '4e9b112a8f5c2bd31cc1e0068d2502e49775fb13b0d48b7188588c0a7b8b714b'],
  ['switch-following-total-return', 'export function run(a,b){switch(a){default:a=b;case 0:return a;}return b;}', '6a9114ef7ed457d154c0115a81d258cf0a2268d523724bc6267dec08e1a5859b'],
];

export const unreachableNumberPrograms = unreachableNumberPromotions.map(([id, source]) => [
  id, '', '', undefined, undefined, { sourceJS: source, sourceTS: source },
]);
