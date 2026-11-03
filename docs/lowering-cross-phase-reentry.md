# Cross-phase source/expression lowering reentry

Updated: 2026-11-03. Frozen integrated baseline: master
[`926a4aa`](https://github.com/tseguine/grunkalunka/commit/926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c).
Status: working implementation inventory; the complete cross-phase
stack-independence claim remains open. Complements
[ADR 0020](adr/0020-source-lowering-control-kernel.md),
[engine steering](javascript-engine-steering.md#source-lowering-architecture-completed-base-and-remaining-work)
and [native-stack issue #281](https://github.com/tseguine/grunkalunka/issues/281).

## Integrated source/evaluator boundary at master `926a4aa`

Source statements and lexical control-flow share
`lower/control_cursor.rs::Cursor`. An `EvaluationCursor` now owns its
`Work`/value/child-Region stacks and root Region. The source cursor drives it
through `Next::Expression` and `advance_owned_expression`; `run_evaluation`
can complete or yield a typed request. The synchronous `evaluate` compatibility
entry point retains a separate `EvaluationLease` to restore its caller's Region.

The following crossings have explicit source-owned continuations:

| Converted crossing | Owning continuation and behavior |
| --- | --- |
| Block-bodied `Work::LocalCall` | `EvaluationRun::LowerBody` yields to `Frame::EvaluateBody`; the original source cursor lowers the body and resumes the evaluator with the same inline-call completion contract. |
| Source expression consumers | Predicates, discard statements, for headers, direct and closure-valued returns, async return resolution/throw reasons and single numeric/primitive-General `let` initializers use typed source expression requests. Other declarations retain their specialized handlers. |
| Direct zero-argument local returned block factory | `LowerFunctionLeaf` can enter `EvaluateFactoryBody`, retaining the evaluator, block-arrow activation, caller binding checkpoint and function-result charge while the body runs on the same source cursor. |
| Immediate invocation of that returned function, such as `get()()` | `LowerSyncClosureCallee` / `EvaluateFactoryCallee` finishes the factory before resuming the outer call's argument evaluation and result-mode routing. The outer evaluator remains owned throughout. |

These are integrated through [#421](https://github.com/tseguine/grunkalunka/pull/421),
[#427](https://github.com/tseguine/grunkalunka/pull/427) and
[#431](https://github.com/tseguine/grunkalunka/pull/431). The returned-factory
source/runtime tests in [#433](https://github.com/tseguine/grunkalunka/pull/433)
cover native JS, Node-Wasm, Wasmtime and JS/TS compilation on 2 MiB native stacks.
Their existence and the observed same-cursor handoff do not establish that every
constructor/checking/helper path is nonrecursive.

Two other integrated changes prepare the remaining crossings without completing
their migration:

- [#428](https://github.com/tseguine/grunkalunka/pull/428) yields
  `LowerFunctionEffect` for discarded/truthy function values. This releases
  `run_evaluation` before effects execute, but `resume_lowered_function_effect`
  still calls `function_value` synchronously.
- [#430](https://github.com/tseguine/grunkalunka/pull/430) owns function-result
  admission and factory binding checkpoints. [#432](https://github.com/tseguine/grunkalunka/pull/432)
  shares `FactoryArgumentKind::{Discard, Callback(signature), Scalar(Mode)}`
  classification and compatibility evaluation. Its argument evaluator still
  invokes `evaluate` or `callback_argument` synchronously.

## In-review argument/effect/callback continuation series

The pushed sequence below is **unmerged** and has its own source/verification
state. It extends the frozen master baseline without changing that baseline's
historical claims:

| PR | Additional source-owned behavior |
| --- | --- |
| [#435](https://github.com/tseguine/grunkalunka/pull/435) | One factory expression state and value/callee resume contract share body completion, resource restoration and diagnostic unwind. |
| [#436](https://github.com/tseguine/grunkalunka/pull/436) | Direct argument-bearing local block factories suspend for each scalar, callback or surplus-discard argument; ordered `ValueId` snapshots remain in caller scope until captures/formals are installed. Both result and immediately invoked callee routes use the same mechanism. |
| [#437](https://github.com/tseguine/grunkalunka/pull/437) | The source-owned Discard/Truthy function-effect path schedules a Function-mode child. It restores the original Region, function-value admission and aggregate result-depth checkpoint before completing the effect and resuming the parent. |
| [#438](https://github.com/tseguine/grunkalunka/pull/438) | `Work::LocalCall` and `Work::ClosureCall` queue the next argument position, yield `LowerCallbackArgument` and consume the resulting Function value through their ordinary argument-result path. |

At the callback-argument checkpoint, the published tip was
[`425f9b6`](https://github.com/tseguine/grunkalunka/commit/425f9b6c7dda02224d64901f67d980cc84a07f84).
Its source tree `6ead7eca6fccb4e39cd11ccd7af37eaf9618ae68` matches the locally
tested implementation. The final local verification receipt in
[#438](https://github.com/tseguine/grunkalunka/pull/438) records:

| Check | Observed result |
| --- | --- |
| Compiler and source probes | 500 compiler tests passed, including callback diagnostics/admission and routing probes on 2 MiB threads; formatting and strict all-target compiler Clippy passed. |
| Runtime oracles | 118 runtime-closure tests, 16 callback-filtered async-suspension tests and 9 module-callback-input tests passed. The async filter intentionally excluded 399 other tests. |
| Frozen-baseline parity | 5,796 existing corpus cases plus 206 targeted JS/TS cases retain byte-identical WAT/Wasm, first diagnostics and admission results against master `926a4aa`. Another 106 CONTROL records retain identical private IR/WAT/Wasm/error characterizations. |
| Routing sensitivity | Overlaying only the new tests onto local predecessor `eb5cda6`, whose tree matches [#437's published head `fd4bccf`](https://github.com/tseguine/grunkalunka/commit/fd4bccf53e8b8810f29334c2bd721172244fe40c), makes both separate `LocalCall` and `ClosureCall` routing tests fail on zero bridge count while compilation itself succeeds. |

Hosted checks and merge status remain separate from these local receipts. The
series does not establish whole-SCC completion or compiler-performance parity.

The reviewed callback handoff keeps the parent evaluator's pending call on the
heap while the source cursor drives the callback's Function-mode child. It shares
`EvaluateFunctionValue` with the function-effect path and uses
`FunctionValueResume::{Effect, Callback}` for completion. The callback-specific
guard runs before general function-value admission; result depth and caller
statement depth are restored before the pending call consumes its result.
Local calls retain inferred callback signatures, while runtime closure calls
retain the formal's complete signature. Success and diagnostic unwind both
restore the resource/depth checkpoints exactly once.

## In-review callee and function-result continuations

This follow-on checkpoint preserves the master `926a4aa` and #438 receipts above.
[#439](https://github.com/tseguine/grunkalunka/pull/439) is their documentation
checkpoint. These dependent PRs extend native stack #440; each PR records its
parent branch. Native membership, hosted checks and integration are separate
states. The maintainer handles native registration where connector support is
unavailable.

| PR | Source-owned coverage and remaining boundary |
| --- | --- |
| [#441](https://github.com/tseguine/grunkalunka/pull/441) | `PreparedClosureCallee::{Ready, Function}` separates validation/direct-slot snapshots from effectful non-slot construction. The prerequisite alone retains synchronous evaluation. |
| [#442](https://github.com/tseguine/grunkalunka/pull/442) | `LowerClosureCallee` and `Work::ResumeClosureCallee` cover source-owned synchronous, awaited and Pending-mode closure calls. `EvaluateFunctionValue` owns the non-slot child with `FunctionValueResume::Callee`; direct-slot loads and the direct block-factory specialization retain their original order. The synchronous `evaluate` host consumes the same request through `function_value`. |
| [#443](https://github.com/tseguine/grunkalunka/pull/443) | `PreparedConciseFactory` and prepare/evaluate-arguments/finish stages retain the known-local closure-metadata contract in the synchronous wrapper. |
| [#444](https://github.com/tseguine/grunkalunka/pull/444) | `source_function_value_leaf` selects metadata preparation once and extends `FactoryArguments` to its ordered arguments. It restores caller bindings before real closure materialization. There is no eight-active-result token, body Region or InlineCall on this metadata path. |
| [#445](https://github.com/tseguine/grunkalunka/pull/445) | `PreparedFactory::LocalResult` also admits general concise bodies; `EvaluateFactoryValue` retains factory bindings/result admission plus the body's ordinary function-value charge. The body starts at expression depth zero with inherited caller statement/result context and the same Region. Completion returns the existing body-produced `ValueId` after restoration. Direct block method calls retain their compatibility path. |
| [#446](https://github.com/tseguine/grunkalunka/pull/446) | `PreparedRuntimeFactory` owns admission and validated call/signature metadata; `finish_runtime_factory` charges expansion and emits `ClosureCall` after callee and arguments complete. This prerequisite retains synchronous callee/argument execution. |
| [#447](https://github.com/tseguine/grunkalunka/pull/447) | `EvaluateRuntimeFactoryCallee` schedules all runtime-factory callees, including slots, with ordinary function-value admission and inherited result depth. It releases that admission before `FactoryArguments` evaluates complete-formal inputs and discarded receiver/surplus effects. `PreparedFactory::Runtime` retains the callee snapshot and result token through final expansion and Function-valued `ClosureCall` emission. The compatibility wrapper remains synchronous and shares the same argument classifier. |

The published implementation tip is
[`c64f222`](https://github.com/tseguine/grunkalunka/commit/c64f222dcda3a682edb55825f820cfd0198cb150),
with tested source tree `3a12c294a2d55d26ae12f3672d6d00192511cd43`.
The local receipts below are scoped to their exact heads and do not establish
hosted CI or integration into the frozen master:

| Checkpoint | Observed local verification |
| --- | --- |
| #442, tree `25836c35da7a96c0db155cc04559eaab86e3cb06` | 506 compiler tests; 119 runtime-closure, all 417 async-suspension and 9 module-callback tests; 6,174 exact WAT/Wasm/diagnostic/admission comparisons and 106 unchanged CONTROL records. Formatting and strict compiler Clippy passed. Separate Sync/Await/Start test overlays on the predecessor compile the admitted sources but fail their zero-bridge assertions. |
| #443, tree `113f62d1cd26eafc1ea50296ebe2f0e42aae9301` | 506 compiler tests and strict compiler Clippy/formatting after integration; 6,260 exact compilation comparisons. The isolated staging extraction separately preserved 52 concise cases and 106 CONTROL records. |
| #444, tree `1c27adfcb4273a1e259db1a7b12987da2dcedaac` | 512 compiler tests; 122 runtime-closure, all 417 async-suspension and 9 module-callback tests; 6,260 exact compilation comparisons and 106 unchanged CONTROL records. Formatting and strict compiler Clippy passed. Three predecessor overlays detect missing ordinary, callback/effect and awaited-async metadata routing. |
| #445, tree `2316518037a40846ee2a2e9a243d6e66a76e75e7` | 519 compiler tests and strict compiler Clippy/formatting passed; 6,314 continuation cases plus 186 runtime-result cases retain exact compilation parity, with 106 unchanged CONTROL records. All 124 runtime-closure tests and 19 affected async tests passed (399 async tests filtered); the async checks are selected, not a full-suite receipt. Two predecessor overlays detect missing synchronous and async body routing. Exact probes preserve result nesting 7/8, body function-value nesting 13/14, the admitted 100+100 wrapper reset, and caller-depth branch boundary 63/64. |
| #446, tree `1eb2f26a0792985536ca9012833986074394c384` | 519 compiler tests and strict compiler Clippy/formatting passed; 186 frozen runtime-factory cases preserve all 318 artifact/error files, plus 106 unchanged CONTROL records. Integrated source `29ef879` and independently rebased prerequisite `d8ec2f9` match the published tree. This synchronous split does not establish runtime-factory scheduling. |
| #447, tree `3a12c294a2d55d26ae12f3672d6d00192511cd43` | 529 compiler tests and strict compiler Clippy/formatting passed; 6,500 exact compilation comparisons cover 11,908 emitted WAT/Wasm/error files, with 106 unchanged CONTROL records. All 125 runtime-closure, 419 async-suspension, 9 module-callback and 9 managed-closure-input tests passed, with no ignored or filtered tests. Three predecessor overlays detect missing callee, typed-argument and async routing. Private-IR tests preserve callee/call spans and snapshots; limit probes preserve runtime8/ordinary16/callback8 ordering and expansion after argument effects. |

The metadata and charged probes intentionally exercise different budgets. The
metadata argument ladder remains admitted at fifteen and rejects at sixteen
active function-value cursors. General concise results instead preserve their
eight-active-result token, plus a separate body function-value admission. The
charged body already supplies a function `ValueId` under its factory bindings;
the metadata path restores its caller before materializing. These are different
ownership contracts even though they share argument scheduling.

The runtime-factory path has its own callee context. It acquires runtime-result
admission before validation, then ordinary function-value admission even for a
slot callee. It inherits the existing result-depth context, releases the callee
charge before callback admission, and retains the result token until all argument
effects and final emission finish. Normal completion and diagnostic unwind
restore only the resources each continuation owns.

The final impact check passed all 106 planner regression tests. Its advisory
shadow plan selects the compiler/runtime owners and their existing downstream
checks. The standalone platform rehearsal retains a previously reproduced
failure on unchanged master `926a4aa`: its expected-command assertion omits
existing dependency selections, before any selected build/test executes. No CI
ownership, source admission or command selection was weakened for this series.

## Residual native reentry circuits after this series

After #447, the following synchronous routes remain. A request being handled by
`advance_owned_expression` is distinct from that same request being consumed by
the compatibility `evaluate` host:

| Route | Current calls and ownership boundary |
| --- | --- |
| Source declarations | `lower.rs::declaration` handles `const`, multiple declarators and function/Promise/managed initializers outside `scalar_let_initializer`. It calls `factory_result`, `function_value`, `evaluate` and scalar/Promise adapters synchronously. Preserve per-declarator admission, argument effects and binding publication. |
| Compatibility local factories and block `.call` | `closures::factory_result` still evaluates caller arguments through its synchronous stage; `function_values::local_function_result` retains argument and concise-result/block-body evaluation for compatibility callers. `prepare_local_function_result` deliberately leaves block method calls on that route even when the outer evaluator is source-owned. Preserve caller captures, fresh formals, signatures and the metadata/result charge distinction. |
| Compatibility runtime function results | Source-owned `source_function_value_leaf` selects the new runtime continuation. `function_value_leaf` still calls synchronous `runtime_function_result`, which constructs its callee through `function_value` and evaluates arguments through the shared classifier and compatibility argument evaluator. Migrate the remaining callers before retiring that wrapper; preserve the distinct callee admission/context and complete formal/result signatures. |
| Compatibility calculated/async callee construction | Source-owned `LowerClosureCallee` requests now schedule a child. The synchronous `evaluate` host still handles those requests through `function_value`, while its direct-block compatibility route calls `prepare_closure_call`. Releasing the large runner preserves a native host frame and is not equivalent to source-owned suspension. |
| Synchronous evaluator host | `evaluate` also handles yielded callback arguments through `callback_argument`, function effects through `function_value`, function leaves through `function_value_leaf`, and `LowerBody` through `arrow_block_result`. Source-owned scheduling does not replace these compatibility entry points automatically. |
| Disposable definition checking | `closures::check_closure_at` and `check_factory_chain` call expression/function/body lowering on cloned checking state. Direct factory chains already use a local worklist, but their helper crossings can reenter other lowerers. Definitions are validated even when runtime effects never execute; checking order, diagnostics and scratch identities must remain isolated. |
| Lifted closure-body materialization | `function_values::materialize_closure` lowers a cloned child through `evaluate`, `function_value` or `arrow_block_result`, then publishes real lifted function IR and `MakeClosure`. Its result ownership and publication differ from disposable checking state. |

A remaining `arrow_block_result → sequence` crossing can still enter another
source cursor, whose body may repeat one of the expression/function routes.
The converted argument/effect/callback, callee, concise-result and runtime-factory
routes do not by themselves eliminate these composed native cycles or the
separately owned checking and materialization activations.

The original eight-active-block, eight-active-function-result and
sixteen-active-function-value budgets remain admission constraints. They do not
constitute architectural stack independence. Effect-free signature, callback-origin,
stored-kind and Promise fact queries retain their separate analysis ownership.
The async IR splitter and runtime Promise job queue also remain separate phases.

**This is an inventory of inspected call paths, not a mechanically complete
function-level SCC graph.** Indirect calls, constructor checking, diagnostics and
other helper edges still need the same audit before declaring source lowering
acyclic.

## Ownership required across each suspension

| Owner | State retained or still to be made resumable |
| --- | --- |
| `EvaluationCursor` | Original Oxc expressions, requested modes, pending `Work`, typed values, source nesting charge and owned unfinished Regions, including the output root |
| Call/argument continuation | Source call/callee, argument index and left-to-right `ValueId` snapshots, signature/result type, pending await site and captures; factory arguments must finish in caller scope before fresh formals are published |
| Block-arrow activation | Return type, loop/await flags, inline and statement depths, visible bindings/closures, source charging, body Region and its completion contract |
| Local factory activation | A metadata preparation owns no result token and restores caller bindings before materialization. General block/concise results own the eight-result admission and caller binding/source-charging checkpoint; `FactoryExpression` owns result-depth restoration and any direct-callee charge. |
| `EvaluateFactoryValue` | General concise body expression, inherited caller statement depth, installed factory bindings, result admission and body function-value charge; completion restores the caller and delivers the body's existing `ValueId` in the original Region |
| `EvaluateFunctionValue` | Suspended evaluator, original Region handoff, caller statement depth, function-value admission and aggregate result-depth checkpoint; `FunctionValueResume::{Effect, Callback, Callee}` selects effect completion or callback/callee value delivery |
| `EvaluateRuntimeFactoryCallee` / runtime `FactoryArguments` | Runtime-result admission and validated call/signature metadata span callee construction, ordered arguments and final emission. The callee owns ordinary function-value admission with inherited result depth; completion releases that charge before arguments and retains its `ValueId` snapshot. |
| Source cursor | Pending source-body request, lexical scope, abrupt `Completion` and the typed continuation consuming the completed Region or expression result |

Every pending value and Region needs one explicit owner. A suspension cannot
leave a mutable caller-Region borrow inside another native call. Diagnostic
unwinding must restore nested contexts in reverse order and preserve the original
first error. Preserve the existing point at which a child Region, formal slot or
result value is allocated, as well as the source order of effects.

## Remaining L4/L5 work and evidence

1. **Remaining function and source circuits:** address the declaration,
   block-method and synchronous compatibility callers listed above. Keep the
   converted factory/effect/callback, callee, concise-result and runtime-factory
   routes covered for
   callee-before-arguments order, snapshots, complete signatures, distinct result
   charges and first errors.
2. **Checking and materialization activations:** preserve disposable checking
   state separately from the child state that publishes lifted IR. Give both
   bounded continuation ownership without weakening unused-definition validation
   or changing lifted identities. Remove a synchronous fallback only after its
   affected circuit has a complete verified replacement, including restoration.
3. **Close the composed graph and cost evidence:** audit helper, callback,
   checking and diagnostic edges in the final SCC inventory; report measured
   exceptions and compiler-time/peak-memory comparisons. An individual helper
   releasing the evaluator runner is not equivalent to eliminating that SCC.
4. **Measure ownership and stack behavior together:** retain
   `evaluator_tests::active_depth`, source-cursor entry/depth observations and
   the converted-family handoff counters. `EVALUATOR_CROSSINGS` counts source
   cursor entries while an evaluator activation is alive; it is not native-byte
   measurement or a whole-compiler worst-case bound. Keep 2 MiB admitted and
   just-over-budget source probes on Linux/Windows.
5. **Preserve outputs and diagnostics:** compare private IR, WAT, Wasm and exact
   first diagnostics for pure refactors, plus native JS / Node-Wasm / Wasmtime
   results and effects. Keep source admission, resource limits and the canonical
   test/impact-selection contract unchanged. Report actual tested heads and
   residual scope before broadening an L4/L5 completion claim.

The observation tests must not require positive native reentry forever. Each
converted family should demonstrate its own explicit handoff and drive avoidable
reentry down through ownership changes, without increasing native stack reserves
or narrowing accepted source syntax.
