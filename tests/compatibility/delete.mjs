// Preserve the exact original numeric-receiver rejection as executable evidence.
export const promotedDeleteGap = {
  id: 'gap-delete',
  sourceSha256: '007d45b36dfd6d6d941189163094b261019a260d0f23ed0736c45fc848ac0d3f',
  sourceJS: 'export function run(a,b) { return delete a.x ? 1 : 0; }',
  sourceTS: 'export function run(a,b) { return delete a.x ? 1 : 0; }',
};

export const deletePrograms = [[promotedDeleteGap.id, '', '', undefined, undefined, {
  sourceJS: promotedDeleteGap.sourceJS, sourceTS: promotedDeleteGap.sourceTS,
}]];
