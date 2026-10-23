# Selected Test262 numeric assertions

Pinned upstream: tc39/test262 at
`d53ebc8b1931d9f1541e43d59650776e4b766248`. Four original files are retained
byte-for-byte under `originals/`, with the BSD license and their Sputnik author
notices. `.gitattributes` preserves original bytes and trailing whitespace.
The MIT adapter and per-check modifications are in `../test262.mjs` and
`manifest.json`. The manifest maps every one of the 18 original assertions to its
source line, original condition, numeric setup and adapted condition.

The adaptations return zero on success and a one-based failing assertion index.
They use the current two-number ABI but do not depend on the arguments. Intrinsic
NaN, infinities and finite limits become numeric literals/arithmetic. Numeric
`isNaN` checks become self-comparisons. Global variable declarations become local
initialized bindings, and exceptions/messages become numeric failure results.
Reciprocal checks retain the distinction between positive and negative zero.

All four originals run as scripts in a Node oracle test, with a minimal
`Test262Error` class. This checks the captured source and expected numeric facts
in Node; it is not execution of those originals in Grunkalunka. The adaptations
run as JS and TS compiled to Wasm in Wasmtime and Node-Wasm. The differential
harness independently requires result zero, rather than relying solely on
agreement between engines. A permanent test falsifies each mapped condition in
turn and requires its own failure index, so omitted assertions are observable.

This is adapted numeric-assertion coverage. It does not implement the Test262
harness, globals, throw, object access, coercion, strict/sloppy variants, modules,
negative phases or async tests. Full upstream conformance remains unexecuted.
All other upstream files are unselected; there is no whole-suite pass percentage.
