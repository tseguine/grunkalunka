# External tests and benchmarks

Compiler IR tests exercise fixed Number/Predicate cells independently of source
admission: initialization/load/store types, lexical visibility, initialization,
mutability and Number-only entry/helper parameters. A mixed-order scalar-cell
emission witness covers snapshots, branch stores and three loop entries in focused
Node-Wasm/Wasmtime playback. The maintained test checks real IR emission and local
types; `GRUNKALUNKA_IR_TEST_ARTIFACTS` optionally writes its WAT/Wasm for that replay.
No tagged-pair or general value representation is selected. The bounded
[immutable Boolean const policy](compiler.md#immutable-boolean-predicate-locals)
uses the existing Predicate cells for predicate-position reads.

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

The separate [lowering-gap inventory](lowering-gaps.md) specifies 39 reference
programs beyond current admission. Node checks their expected results; the compiler
must still reject each with the recorded diagnostic and create no artifact. These
are executable requirements for later work, not upstream suite or lowering passes.

The [complete rejection inventory](../tests/compatibility/rejection-inventory.json)
also covers all 797 negatives, including the curated references. Its 56 groups
distinguish 26 invalid sources, 99 resource restrictions and 672 open requirements.
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

Math.abs fixtures use the numeric profile's pristine intrinsic and an unmodified
Node Math.abs oracle. No host monkeypatch oracle is executed. The registered raw-bit
kernel test compiles production lowering and compares 4,109 inputs in both engines;
non-NaNs require exact bits and NaNs require the NaN class. Separate rejection tests
cover semantic shadows, dead patches and unused helper patches. Ordinary numeric
Math parameter/local mutation remains admitted; type-only Math names remain erased.
The two promoted requirements retain their source bytes and curated provenance;
664 other old rejection records and all 39 remaining curated references are unchanged.

Wrapped TypeScript write targets add 14 exact source promotions and 37 paired
runtime cases. Original TS modules use hand-paired full JS oracles; no regular
expression erasure or checker supplies the oracle. Four wrapper forms, mixed and
retained inner parentheses, all compound operators, prefix/postfix snapshots,
shadowing, helper/loop compositions and exact depth boundaries are covered.
Compiler tests preserve const/TDZ/global/function-write and nonidentifier rejection,
including unselected and unused helper bodies. Finite mutations skip wrapper depth,
bypass semantic resolution, or delay target loading; each fails before exact source
restoration and a fresh rebuild. None of these negative sources is run in Node.

The unary Math family uses one original-JS kernel per method. Each compares 51
named boundaries and 4,096 seeded binary64 inputs with Node-Wasm and Wasmtime.
Named cases cover signed zeros, fractions, infinities, NaNs, subnormals, 2^52
neighbours, maximum finite inputs and adjacent square values. Non-NaNs require
exact bits, while NaNs require their class. Four wrong-opcode probes, a negative-zero
loss probe and semantic-shadow bypass are detected; restoration refreshes mtime and
rebuilds. Selected effects, snapshots, helpers, wrappers, numeric Math mutation and
rejected patches stay within the pristine execution profile. This finite matrix is
regression evidence, not exhaustive binary64 conformance or a performance claim.

Boolean complement adds four exact promotions and 36 paired cases covering literals,
all numeric comparisons/equalities, negation, source effects, wrappers, misleading
assertions, Number boundaries, depth and helper/loop composition. The verifier
rejects wrong bridge types, undefined/invisible inputs and repeated result definitions.
Finite mutations invert the conversion, classify arbitrary truthy Numbers as Boolean,
duplicate operand evaluation or reset operand depth. Every mutant fails before
exact source restoration, mtime refresh and rebuilding. No new negative is executed
as JS; the remaining 750 rejection observations and 39 gap oracles are retained.

Discarded-value fixtures compare effects, snapshots, selected/skipped logical and
conditional branches, numeric helpers, TS wrappers and finite loop headers/bodies.
Nested comma depth and statement-budget boundaries retain the original source
charges. Four finite sensitivity probes detect dropped evaluation, double
evaluation, eager short-circuit RHS execution and depth reset; exact restoration
refreshes source mtimes before rebuilding. All retained negative programs are
compiled only and never executed as arbitrary Node oracles.

The private runtime [ownership tests](../crates/grunkalunka-runtime/tests/resource_ownership.rs)
are discovered by the existing Cargo workspace test command. Five finite probes
check owned roundtrip identity/value followed by consuming the actual lifted token,
exact guest-triggered destructor counts,
repeated borrowed reads, immediate stale-index replay and duplicate guest drop.
Expected traps use fresh instances and assert `unknown handle index`, host read
counts and backing liveness before teardown. An unrelated-trap mutation verifies
the trap-reason assertion; three omission mutations verify the required operations.
No compiler corpus counts, public runtime policy or existing borrow fixture change.

## Boolean const coverage

`const-boolean.mjs` adds 44 JS/TS pairs, including four exact original promotions,
and 34 source-bound negatives. Differential cases cover captured comparisons,
initializer effects, aliases, scope/TDZ, misleading type syntax, per-entry loop
initialization, original-node depth and cumulative mixed-type local budgets.
Both retained return frontiers pin their full diagnostic/span; 13 pre-existing
local-budget diagnostics change only “numeric” to “source”. The curated promotion's
original source hashes and independent +0 reference are checked directly.
Current corpus: 1008 pairs, 2016 modules and 2,528,064 comparisons; 797 rejections
in 56 groups (26 invalid, 99 resource restrictions, 672 open), including 90 Boolean
frontiers and 38 curated references. Prior admitted source/WAT/Wasm artifacts remain exact.
