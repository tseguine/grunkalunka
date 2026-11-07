// Exact original rejection sources now execute with their unchanged JS oracle.
export const managedNumberPromotions = [
  ['gap-object-property', 'export function run(a,b) { return ({x:a}).x + b; }',
    'a7d2e7d7306cbde71e2fb1db6397b0a9d87f8c33d7f449c82347b1bea9157e46'],
  ['gap-array-value', 'export function run(a,b) { const items = [a,b]; return items[1]; }',
    '2d0bd7338ed4499fdf7c1ff686f81383b08a807793cd605b91113bb4dc850ec7'],
  ['gap-local-destructure', 'export function run(a,b) { const {x}={x:a}; return x+b; }',
    'bd0e98405bb8cd832047966de53955f4ecf99acb4dddacea6b7830490c439841'],
  ['gap-destructuring-write', 'export function run(a,b) { let x=a; [x]=[b]; return x; }',
    '1afc439b3efa7d353c3da4f0bbb7ee0aa4b9a3f0fd1ad3023b53881875b717d0'],
  ['for-of', 'export function run(a,b) { for(const i of [a]) { return i; } return b; }',
    '6ec79ccff9f7ab9e84d3c6f91990edb072bd1aa0bf66834c1aa199df066c6845'],
];

export const managedNumbers = managedNumberPromotions.map(([id, source]) => [
  id, '', '', undefined, undefined,
  { sourceJS: source, sourceTS: source, oracleSource: source },
]);
