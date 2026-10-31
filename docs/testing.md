# External tests and benchmarks

## Current verification map (2026-10-31)

The canonical check catalog under [`scripts/checks/`](../scripts/checks/README.md)
and the [executable compatibility corpus](../tests/compatibility/corpus.mjs)
are authoritative for today's commands, fixture membership and counts.
The corpus currently declares version **88**. The older numeric-pair and
rejection totals retained in historical paragraphs below describe their
named **past** revisions, not current percentages or suite totals.

The synchronous [WasmGC object increment](managed-values.md) has maintained
compiler typed-IR tests and runtime source differential suites, including
`managed-source`, `managed-calls`, `managed-strings`, `managed-arrays`,
`managed-logical` and `managed-values`. Their asserted
domain is the admitted fresh-object/static-own-property/helper subset and
String construction, content equality and truthiness, plus local arrays with
static element/length operations, identity and ordered reference transport,
not prototype compliance or all JavaScript objects. The
[A1 source import](a1-ui-source-import.md) has a dedicated real-provider
`a1_ui_import` Rust integration test. The
[A1 note preview](a1-note-preview.md) uses the ordinary Node application
test for authorized WIT data, native scene output and host-managed input/repaint;
the two A1 paths are not yet a single source-import-driven app.

Native-stack-depth regressions now cover deeply nested structured IR and
selected iterative frontend, JSON and layout walks on constrained thread stacks.
They do not establish a recursive-call-free Rust codebase; see
[stack-depth steering](https://github.com/tseguine/grunkalunka/issues/281).
These tests are part of normal canonical/selected checks rather than separate
per-feature Actions jobs. [CI](ci.md) describes what Linux/Windows partial PR
selection and the native macOS baseline actually execute.

Local block-arrow checks live in `crates/grunkalunka-runtime/tests/local-arrows.rs`:
Node and Wasmtime compare shared captures, fresh locals, early/loop returns,
caller continuation and argument order with native JS. Additional checks cover
the primitive return tag, TypeScript metadata, unused-body/source-budget charging,
aliases, repeated specialization and bounded nesting on a 2 MiB thread stack.
`async-suspension.rs` checks awaited block-call results through the same async
job-order oracle. Rejection totals are revision-specific; consult the executable
numeric corpus and its recorded report instead of treating older totals as current.
Async tests also compare retained primitive kinds, rejection reasons, strict versus
loose comparisons, numeric coercion and malformed tags at start/resume boundaries.

The normal check command also builds the owned JSON guest offline for
`wasm32-unknown-unknown` and checks the same module in Node and Wasmtime. Install
that Rust target during setup; verification does not download it. Replay this
boundary alone with `node scripts/check-json-guest.mjs`. Its UTF16 oracle records,
artifact and hash receipt are retained in the selected Cargo target directory's
`json-guest/`. Tests cover all code units, finite-number samples, object ordering,
quotas, stale IDs, release, repeats and separate instances. This does not execute
source-lowered `JSON.parse`/`JSON.stringify`; those remain unadmitted.

`node scripts/check-json-composition.mjs` first runs that guest check, then links
the actual guest with generated Wasm fixtures and executes the same combined
artifact in both engines. It covers function/type/data index remapping, passive
literals, internal helpers, numeric and primitive-pair calls, repeats and separate
instances. Retained runtime exports also pass the original transport oracle.
The normal check calls this combined runner once. Evidence is in the selected
Cargo target directory's `wasm-composition/`; fixture Wasm is not JS source lowering.

The guest check also writes `json-guest/string-cases.tsv`: seven raw String records,
including all 65,536 UTF16 units in bounded chunks, with Node JSON quoting oracles.
Both engines check direct construction/readback, 500 String lifecycles, parsed String
compatibility, wrong-kind status 9, mixed root quotas and independent snapshots.

The standalone and composed guest checks also exercise the same temporary-root
protocol in Node and Wasmtime: checked current tokens, parent/host result transfer,
500 temporary allocation cycles, clear/repin, abort, 128-frame and 64-root bounds.
Native tests additionally reach terminal frame/root IDs and check that release
does not accumulate stale pins. These tests do not execute a source heap profile
or claim recovery after arbitrary engine interruptions.

Compiler IR tests exercise fixed Number/Predicate cells independently of source
admission: initialization/load/store types, lexical visibility, initialization,
mutability and Number-only entry/helper parameters. A mixed-order scalar-cell
emission witness covers snapshots, branch stores and three loop entries in focused
Node-Wasm/Wasmtime playback. The maintained test checks real IR emission and local
types; `GRUNKALUNKA_IR_TEST_ARTIFACTS` optionally writes its WAT/Wasm for that replay.
No tagged-pair or general value representation is selected. The bounded
[immutable Boolean const policy](compiler.md#immutable-boolean-predicate-locals)
uses the existing Predicate cells for predicate-position reads.

The private [evaluator control tests](../crates/grunkalunka-compiler/src/evaluator_tests.rs)
use real parsing, semantic analysis and region lowering. Finite characterization
covers each admitted expression family and all four evaluator modes, including
incremental argument/spread validation, bridges, snapshots and nested regions.
Test-only observations measure actual evaluator entries and pending-work length;
entry balance is checked after both success and error, and active entry depth is
one. These observations do not measure native-stack bytes or establish a heap bound.
Small private-IR assertions protect allocation order that runtime results cannot
expose. Whole-corpus source/WAT/Wasm and complete rejection observations provide
the broader control-refactor invariance check; they do not add source support.

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

Public resolved/open browser and embedding bug reports are a separate **proposed**
source of adversarial regression scenarios. See the [historical bug-hardening
matrix and triage criteria](historical-bug-hardening.md), including Electron,
CEF and Ultralight. They are not a seventh adopted suite, new CI requirement,
or permission to change workstream priorities.

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

The compatibility watchdog derives its whole-run budget from the current inventory:
`innerMs = min(600000, max(180000, 30000 + 200*P + 25*R + 100*G))`, where `P` is
paired programs, `R` is rejection rows and `G` is curated rejection-oracle rows
(`0 <= G <= R`). Counts must be safe nonnegative integers, and unsafe intermediate
arithmetic is rejected before execution. The outer compatibility subprocess timeout
is `innerMs + 60000`, at most 660000ms. These rounded allowances are an engineering
watchdog allocation, not measured per-row costs, a benchmark or a performance SLA.
The guard checks elapsed time before each CLI launch, allowing the exact deadline
and rejecting the next launch over budget. Individual CLI subprocesses retain their
30000ms timeout; curated JavaScript oracles retain 5000ms. Semantic assertions and
report completion are required regardless of the allocated budget.

The separate [lowering-gap inventory](lowering-gaps.md) specifies 38 reference
programs beyond current admission. Node checks their expected results; the compiler
must still reject each with the recorded diagnostic and create no artifact. These
are executable requirements for later work, not upstream suite or lowering passes.

The [complete rejection inventory](../tests/compatibility/rejection-inventory.json)
also covers all 995 negatives, including the curated references. Its 56 groups
distinguish 31 invalid sources, 144 resource restrictions and 820 open requirements.
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

Primitive-Boolean unary plus/minus add 41 paired cases and 18 classified boundary
rejections, with no old promotion. Literal results check all four exact bit patterns,
including `-false` as -0. Comparisons, negation, once-only effects, captured results,
helpers, loops, TS wrappers, unchanged Number operands and depth/statement limits
are covered. Four finite lowerer mutations invert conversion, replace negation with
`0 - input` (losing -0), evaluate the operand twice or reset its depth. Each fails
before exact source restoration, mtime refresh and rebuilding. The post-local
baseline has 797 rejection observations and 38 gap oracles to preserve;
new negative programs are compiled only and are not executed in Node.

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
Corpus 42 recorded 1008 pairs, 2016 modules and 2,528,064 comparisons; 797 rejections
in 56 groups (26 invalid, 99 resource restrictions, 672 open), including 90 Boolean
frontiers and 38 curated references. Prior admitted source/WAT/Wasm artifacts remain exact.

## Inline Boolean arithmetic coverage

`numeric-boolean-arithmetic.mjs` adds 66 paired cases (six exact source promotions)
and 30 classified negatives. Every operator has Boolean left/right/both operands;
cases cover 1110/410 evaluation-order snapshots, once-only effects, signed zero,
NaN/infinities/subnormals, wrappers, const-predicate negation, helpers, finite loops
and source bounds. Four targeted mutants exercise conversion, order, duplication
and depth. Three old Rust-only rejection programs are retained as positive checks.
That arithmetic run recorded 1115 pairs, 839 rejections and 38 curated references; 56 inventory
groups classify 26 invalid, 109 resource restrictions and 704 open requirements.
The Boolean-values assessment is widened explicitly; retained record identities
and all other group fields remain unchanged apart from the six promotions/new records.

## Unary immutable Predicate binding coverage

`unary-predicate-bindings.mjs` checks all unary operators on true/false cells,
aliases, captured/once-only initializers, numeric and Predicate shadows, intrinsic
names, misleading annotations, four TS wrappers, original depth and finite loops.
Signed zero is compared by bits; +false is +0 and -false is -0. Three exact ordinary
promotions retain IDs, hashes and both duplicate-source observations. Direct return,
call/assignment/discard/equality, binary and mutable-storage exclusions stay open.

The CLI depth controls run before expanded focus: the exact retained 554-byte
bitwise depth-limit source must keep its diagnostic and no artifact, followed by
Number and Boolean original-depth 128/129 controls. Native failure stops the unit.
Four built lowerer mutations exercise wrong binding kind, conversion, wrapper-depth
charging and duplicate effectful predicate evaluation; exact restoration refreshes
source mtime before rebuilding. The 836 retained rejection objects and all old
Number source/WAT/Wasm artifacts have an explicit baseline audit; negatives are not
executed as arbitrary Node programs.

## Arithmetic immutable Predicate binding coverage

`arithmetic-predicate-bindings.mjs` covers all five operators with binding operands
on either/both sides, all Boolean pairs, aliases, initialization/capture, semantic
shadows, misleading annotations, wrappers, helper calls and finite loops. Fixed
independent outcomes distinguish left snapshots (1310), evaluation order (102),
and once-only effectful predicates (201); binary64 comparison preserves signed zero.
Four exact ordinary promotions retain IDs, hashes and independent full JS sources.
Boundary negatives preserve all excluded operator/value/storage positions.

Before broader focus, the fresh CLI must retain the exact 554-byte Number depth
rejection and all 56 retained nesting observations, then original-depth128/129
Number/inline-Boolean/binding controls. Four built mutants exercise kind evidence,
conversion, wrapper charging and duplicate predicate effects, followed by exact
source restoration and fresh builds. Final artifact/whole-object auditing compares
all 1158 old pairs, 857 retained rejections and 38 curated references; the Boolean
assessment change is explicit rather than a normalization of old observations.

## Discarded immutable Predicate binding coverage

`discarded-predicate-bindings.mjs` covers true/false reads in statements, comma
prefixes and both for headers, four TypeScript wrappers, aliases, initialization
once/capture, semantic shadows, annotations, helper functions and finite loop
re-entry/continue. Existing effectful inline discards retain once-only and skipped
behavior. Root-join, binding, assignment-result and other value-position negatives
keep the eligibility extension narrow. Five exact promotion IDs retain their full
original JS text and independent paired TypeScript sources.

The fresh CLI first checks the exact 554-byte retained Number depth rejection,
then all 61 current nesting negatives and original-depth128/129 Number/inline-
Boolean/binding controls in all four discard contexts. Any native failure stops the
unit. Mutation probes target missing eligibility, root-join widening, wrapper depth
at a comma-prefix entry depth of one, and duplicate effectful predicate evaluation.
Restoration refreshes source mtime and rebuilds. Final auditing preserves all 1246
old pairs, 879 other whole rejection observations and 38 curated oracles, with only
the exact Boolean assessment update and declared promotions/new fixtures permitted.

## Strict Boolean equality coverage

The paired fixture covers both strict operators/all four Boolean pairs, selected
inline shapes, immutable aliases, shadowing, misleading annotations, all TS wrapper
forms, nested comparisons, helpers, finite loops and existing numeric compositions.
Fixed independent results distinguish order/snapshots (120 versus 110), once-only
left/right effects (111), and eager RHS evaluation. Exactly two ordinary source
promotions retain full original JS bytes/hashes; no curated reference transfers.

Before broader focus, the first fresh CLI must preserve the exact 554-byte Number
depth rejection and all 65 retained nesting observations, then new JS/TS original
depth128/129 controls. Four built sensitivity probes disable the bridge, widen the
strict-only gate, reverse order and duplicate a predicate; every probe requires
exact restoration and a fresh successful build/test. Any native failure stops work.
Final auditing must preserve all 1299 old pairs, 903 retained rejection observations
and 38 curated references, with only the explicit Boolean assessment change.

The [numeric logical-assignment fixtures](../tests/compatibility/numeric-logical-assignment.mjs)
cover selected/skipped Number `&&=`/`||=`, signed-zero/NaN truthiness, same-target
RHS writes, later sibling writes, once-only effects, wrappers, helpers and loops.
Real-parser private-IR assertions require captured parent Load/Truthy/If, a selected
RHS/Store and an empty skipped child yielding the old snapshot. An unconditional
same-old Store can be runtime-unobservable for this domain, so structural Store
placement is checked independently of engine results. Selected-result checks use
the RHS snapshot, including a deliberately stale-old-result sensitivity probe.
The two ordinary promotions and two remaining whitelist-diagnostic changes are
pinned individually; no curated reference transfers or broad normalization applies.

The [type-only module-item fixtures](../tests/compatibility/type-only-module-items.mjs)
retain four complete original TS sources and hand-paired JS oracles. They cover
all four declaration-kind families, source order around helpers, namespaces and
value shadows, scalar effects, shared 255/256-statement boundaries, original
16 KiB UTF-8 source bytes and wrapper depth. Runtime/side-effect/inline/mixed
imports and runtime exports remain negative. Parser/Semantic precedence and
exported-entry checks remain independent of erasure. Nested function/block,
after-return, dead-branch and unused-helper cases retain original body admission.
Four built sensitivity probes over-erase runtime imports/exports, omit statement
charging or leak module erasure into the original body predicate; positive
admission tests independently detect omission of the new erasure.

[Generic declaration fixtures](../tests/compatibility/generic-declarations.mjs) preserve
three exact promotion sources and complete handwritten JS oracles. They cover entry
and helper metadata, constraints/defaults/const parameters, type/value namespaces,
ordered argument effects, finite recursion and source budgets. Adjacent shape guards,
explicit type-argument calls and unsupported bodies remain negative. Two built guard
mutants independently reintroduce the entry or helper restriction; original artifact
and diagnostic audits retain eleven exact wording-only exceptions.

The [General logical-assignment fixtures](../tests/compatibility/general-logical-assignment.mjs)
cover exact primitive results, selected/skipped RHS effects, same-target writes,
saved snapshots, aliases, parameters, wrappers, loops and finite recursion.
Runtime tests compare 1600 modules against independent four-kind goldens and
original JavaScript in both Wasm engines, plus 64 effect/order modules and sixteen
exact former guard modules. Depth, local, statement, parameter and unsupported
operand/target boundaries remain checked. Two built mutations omit the selected
Store or invert branch selection. Compatibility accounting retains eight exact
diagnostic advances and two unchanged numeric rejection sources as Value origins;
neither diagnostic movement nor Value execution is counted as numeric promotion.

The compiler's `ir::gc_closure_tests` exercises the traced closure backend independently
of source managed-capture admission. `managed-closure-storage` in the runtime package
checks the shared GC cell/frame type fragment across forced Wasmtime copying
collections. These tests distinguish emitted-IR/Node behavior from hand-built
Wasmtime lifetime evidence; source captures and suspended GC frames remain rejected.
