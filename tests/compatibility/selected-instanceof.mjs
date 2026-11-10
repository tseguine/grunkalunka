// The first three JS sources previously had explicit compiler rejection expectations.
const selected = [
  ['gap-instanceof', 'export function run(a,b) { return [] instanceof (a ? Array : Object) ? 1 : 0; }'],
  ['instanceof-selected-array', 'export function run(a,b){return [] instanceof (a?Array:Object)?1:0;}'],
  ['instanceof-sequence-array', 'export function run(a,b){return [] instanceof (a=b,Array)?1:0;}'],
];
export const selectedInstancePrograms = selected.map(([id, sourceJS]) => [id, '', '', 1, undefined, {
  sourceJS, sourceTS: sourceJS.replace('(a,b)', '(a:number,b:number):number'),
}]);

const shadowed = 'export function run(a,b){const Array=x=>x;return [] instanceof (a?Array:Object)?1:0;}';
export const selectedInstanceRejected = [
  ['instanceof-selected-shadowed', shadowed, 'js', 'instanceof requires an intrinsic constructor'],
];
