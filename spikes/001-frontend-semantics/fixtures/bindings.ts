export function run(): number {
  const x: number = 40;
  function inner(x: number): number { const capture = () => x + 1; return capture(); }
  return x + inner(1);
}
