// Finite compositions within the numeric admission, not newly supported syntax.
const cases = [
  // Reordering either operand changes the other's input, not only the final state.
  ['ordered-subtraction-writes', 'return (a = a + b) - (b = a - b);'],
  ['ordered-division-writes', 'return (a = b) / (b = -a);'],
  // NaN on the left must not suppress the right-hand assignment.
  ['comparison-rhs-after-nan', 'let x = 0; const y = a < (x = 7) ? 1 : 2; return x + y;'],
  // Test effects and only the selected arm remain visible after the ternary.
  ['ternary-test-and-arm-state', 'let x = 0; let y = 0; const z = (a = a + b) ? (x = x + 1) : (y = y + 2); return a + x + y + z;'],
  // Per-entry initialization also mutates an outer binding.
  ['loop-initializer-writes-outer', 'let i = 0; let x = a; while (i < 3) { let y = (b = b + 1); x = x + y; i = i + 1; } return x + b;'],
  // The terminal false condition performs its assignment too.
  ['while-assignment-truthiness', 'let i = 3; let x = a; while (i = i - 1) { x = x + b; } return x + i;'],
  // A written inner shadow must not replace the outer slot after the loop.
  ['loop-shadow-write-restoration', 'let x = a; let i = 0; while (i < 2) { { let x = b + i; x = x + 1; b = x; } i = i + 1; } return x + b;'],
];

export const loweringRegressions = cases.map(([id, body]) => [id, body, body]);
