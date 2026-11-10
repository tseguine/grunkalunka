// Keep the original named-export rejection byte-for-byte as an executing oracle.
export const localExportEntries = [
  ['named-export', 'function run(a,b) { return a; } export {run};'],
  ['local-export-before-declaration', 'export { main as run }; function main(a,b){return scale(a)+b;} function scale(x){return x*2;}'],
  ['local-export-quoted-name', 'function impl(a,b){return a-b;} export { impl as "run" };'],
  ['local-export-shadowed-name', 'function run(x){return x+1;} function main(a,b){return run(a)+b;} export {main as run};'],
].map(([id, source]) => [id, '', '', undefined, undefined, { sourceJS: source, sourceTS: source }]);
