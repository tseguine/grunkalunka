# External tests and benchmarks

All six reference suites are registered in
[`suites.json`](../tests/compatibility/suites.json) at immutable upstream revisions.
The registry records source and root-license hashes observed during public source
inspection. These hashes describe upstream files; files not retained locally are
not integrity-checked offline. No suite is downloaded or installed by normal checks.
Review selected-file and bundled third-party terms before adding more material.

| Suite | Purpose | Current execution | Next executable milestone |
| --- | --- | --- | --- |
| Test262 | ECMAScript language conformance | Four adapted numeric programs covering 18 assertions | An original-harness runner for supported semantics |
| TypeScript tests | Compiler and syntax references | 32 adapted numeric cases; zero selected admission rejections | Broader language cases and separate type-query checks |
| Computer Language Benchmarks Game | Complete algorithms and performance workloads | One instrumented Mandelbrot scalar kernel | Full program output, then performance measurements |
| JetStream | JavaScript and Wasm engine performance | Pending | A full shell workload with its correctness checks |
| Node.js tests | Host API compatibility and regressions | Pending | Selected original API tests with capability grant/denial cases |
| Speedometer | Browser application responsiveness | Pending | Correct application interactions after DOM, events and rendering exist |

Run `node tests/compatibility/suites.mjs` to inspect pins, prerequisites and the
next slice for every suite. Without execution evidence, all execution counts are
zero. Run `node tests/compatibility/suites.mjs <report.json>` to summarize an actual
compatibility report. Selected results must exist, be unique, have the right
admission expectation and have completed execution; malformed evidence is rejected.

`node scripts/check.mjs` validates the registry/reporting behavior and runs the
current differential corpus. Its generated `report.json` includes `externalSuites`:
adapted runtime results and successful rejection checks are separate counts.
Unmodified upstream suite execution remains `not-executed`, including for TypeScript.
This is a selected-case inventory, not a denominator for the full upstream suites.

The separate [lowering-gap inventory](lowering-gaps.md) specifies 40 reference
programs beyond current admission. Node checks their expected results; the compiler
must still reject each with the recorded diagnostic and create no artifact. These
are executable requirements for later work, not upstream suite or lowering passes.

The [complete rejection inventory](../tests/compatibility/rejection-inventory.json)
also covers all 666 negatives, including the curated references. Its 56 groups
distinguish 26 invalid sources, 85 resource restrictions and 555 open requirements.
Records pin original source hashes, JS/TS extensions and the actual strict Module
goal. Tests reject missing, stale, duplicate or changed records and invalid group,
evidence, phase, harness or completion metadata before fixture execution. Feature
dependencies describe interacting semantics; cycles are legal and do not schedule work.

The report embeds the assessments and validates every resolved rejection classification
after observation. A finite sensitivity probe drops classification after input validation;
the final report check must fail. Rejection-test execution remains separate from
unexecuted compiled lowering, and arbitrary negatives never run in Node. Const writes,
TDZ/unresolved reads and unselected error paths are valid source with possible runtime
errors, not invalid syntax. Entry, numeric ABI and structural-return policies remain
open requirements. Language-error phase metadata describes the applicable standard,
not the compiler frontend stage which happened to diagnose it. Browser regressions
must cite the applicable standard even when engines agree; any intentional de facto
deviation needs an explicit documented policy and configuration where appropriate.

Mandelbrot's suite record separates demonstrated numeric `for` and bitwise/shift
prerequisites from remaining typed-array and process/stdout blockers. This does not
claim complete original output, upstream execution or benchmark performance.

Test262 needs its original metadata, harness includes, strict/sloppy variants,
negative-error phases, module and async semantics before claiming upstream passes.
The TypeScript tests also include intentionally invalid programs and compiler-only
expectations. Adapted runtime tests do not establish type-checker conformance.

The annotation fixtures pair exact annotated TS source with unannotated numeric
JS. They include unknown and contradictory annotations without treating them as
runtime proofs. Numeric values, binding rules and source budgets are checked
independently. Structural review confirms the original annotated Oxc AST remains
available through compilation; these tests do not claim a type checker, a
persistent metadata artifact or a type-guided optimization bridge.

The explicit type-declaration fixtures cover aliases/interfaces in module,
function and block statement positions, including their direct/default-interface
exports and source charges. Types after return/break/continue preserve completion;
runtime tails still reject. Type/value namespace, depth/byte and combined
directive/type/body statement boundaries are checked independently of type checking.

The Node test-source pin is independent of the installed Node oracle version;
the actual oracle is recorded in each compatibility report. Node tests will need
module loading and host APIs; executing them in ordinary Node alone would not test
Grunkalunka's Node compatibility. JetStream needs its shell harness and broader
language/built-in support. Speedometer additionally needs a browser environment.
Their registered blockers are integration work, not passing or expected-failure tests.

Performance measurements follow correct complete workload output. Record frontend
and lowering time, Wasm compilation, instantiation, steady execution, memory and
artifact size separately, with host versions, inputs, warmup, repetitions and
variance. An extracted kernel cannot claim an upstream benchmark score. Retain
upstream scoring and timing rules when a complete benchmark becomes executable.

Admitted numeric fixtures may provide complete JS/TS modules through explicit
sixth-field source metadata. The source-construction tests ensure exact text and
oracle selection, reject incomplete metadata, and preserve legacy body fixtures.
Full-source directive fixtures therefore exercise module prologues in their
original positions. The replay reads retained input/oracle bytes back to verify
source hashes rather than silently replacing full source with generated wrappers.

The direct numeric call fixtures use full JS/TS modules and unchanged full-source
JS oracles. They cover zero/one/multiple arguments, a 1,000-parameter backend
boundary, snapshots, invocation isolation and fixed-counter self/mutual recursion.
All 631 prior rejection sources/modes/diagnostics and 41 gap oracles are retained.
A separate Wasmtime-only excessive finite-depth recursion test checks a typed
fuel/stack trap and instance reuse; it never runs a recursive overflow in Node.

The production-compiled remainder kernel checks 23,136 boundary/seeded raw binary64
pairs against Node `%`, an independent BigInt aligned-integer oracle, Node-Wasm and
Wasmtime. It routes through a source helper named `number_remainder`, checking the
private arithmetic-helper namespace and call indices. Non-NaNs compare exact bits;
NaNs compare by class. The full-range matrix concentrates work in one remainder
per invocation rather than dense compositions. Three finite mutations lose negative
zero, use a rounded quotient formula or delay `%=` target loading; each must fail.
Restoration touches the changed source and rebuilds before rechecking.

A separate Wasmtime-only finite repeated-remainder workload exhausts the unchanged
10,000 fuel, then reuses the same instance successfully. It is not run as JS and
is distinct from wrong-result checking. Finite differential evidence and the helper's
conditional integer bound do not prove exhaustive binary64 conformance, arbitrary
composition fuel capacity or performance. The TypeScript adapter now executes all
32 selected numeric expressions while full upstream execution remains unexecuted.
