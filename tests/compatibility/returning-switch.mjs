export const promotedSwitchGap = {
  id: 'gap-switch',
  sourceSha256: 'ced717662fb6f26e1bd4f553db7716c7dc42697c174e5df51bbf25b5426ab604',
  sourceJS: 'export function run(a,b) { switch(a) { case 4: return b; default: return a; } }',
  sourceTS: 'export function run(a,b) { switch(a) { case 4: return b; default: return a; } }',
};
export const returningSwitch = [
  [promotedSwitchGap.id, '', '', undefined, undefined, {
    sourceJS: promotedSwitchGap.sourceJS, sourceTS: promotedSwitchGap.sourceTS,
  }],
  ...[
    ['returning-switch-default-middle', 'let n=0;switch(a){case (n++,0):return n;default:return n;case (n=n*10+2,1):return n;}'],
    ['returning-switch-strict-tags', 'switch(a?true:null){case 1:return 99;case true:return a;case null:return b;default:return 98;}'],
    ['returning-switch-strings', "switch(a?'x':'y'){case 'x':return a;case ('y'+''):return b;default:return 99;}"],
    ['returning-switch-snapshot', 'switch(a){case (a=b,b):return a+10;case b:return 99;default:return a;}'],
    ['returning-switch-nested', 'switch(a){case 0:switch(b){case 0:return 1;default:return 2;}default:return 3;}'],
  ].map(([id, body]) => [id, body, body]),
];
