// Deliberately simple scalar arithmetic with observable binary64 rounding.
// The fixed portability oracle is independent of runtime source evaluation.
export function run(a, b) {
  return (a * 2 + b) / 3;
}
