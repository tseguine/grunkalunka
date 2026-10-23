export function run() {
  const x = 40;
  function inner(x) { const capture = () => x + 1; return capture(); }
  return x + inner(1);
}
