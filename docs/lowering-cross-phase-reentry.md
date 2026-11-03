# Cross-phase source/expression lowering reentry

Updated: 2026-11-03. Frozen integrated baseline: master
[`926a4aa`](https://github.com/tseguine/grunkalunka/commit/926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c).
Status: effectful-source ownership and compatibility retirement are complete in
the final in-review implementation. The dated receipts below remain historical;
the [final inventory](#final-in-review-effectful-source-ownership) defines this
bounded completion claim. Broader compiler/native-stack evidence remains separate.
Complements
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

At this checkpoint the sequence below was pushed and **unmerged**, with its own
source/verification state. It extended the frozen master baseline without
changing that baseline's historical claims:

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

At this checkpoint the published implementation tip was
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

This is the historical residual inventory after #447. The final implementation
below closes these effectful-source routes; retain this table to identify what
the later units replaced. At that checkpoint a request being handled by
`advance_owned_expression` was distinct from that same request being consumed by
the compatibility `evaluate` host:

| Route | Calls and ownership boundary at #447 |
| --- | --- |
| Source declarations | `lower.rs::declaration` handles `const`, multiple declarators and function/Promise/managed initializers outside `scalar_let_initializer`. It calls `factory_result`, `function_value`, `evaluate` and scalar/Promise adapters synchronously. Preserve per-declarator admission, argument effects and binding publication. |
| Compatibility local factories and block `.call` | `closures::factory_result` still evaluates caller arguments through its synchronous stage; `function_values::local_function_result` retains argument and concise-result/block-body evaluation for compatibility callers. `prepare_local_function_result` deliberately leaves block method calls on that route even when the outer evaluator is source-owned. Preserve caller captures, fresh formals, signatures and the metadata/result charge distinction. |
| Compatibility runtime function results | Source-owned `source_function_value_leaf` selects the new runtime continuation. `function_value_leaf` still calls synchronous `runtime_function_result`, which constructs its callee through `function_value` and evaluates arguments through the shared classifier and compatibility argument evaluator. Migrate the remaining callers before retiring that wrapper; preserve the distinct callee admission/context and complete formal/result signatures. |
| Compatibility calculated/async callee construction | Source-owned `LowerClosureCallee` requests now schedule a child. The synchronous `evaluate` host still handles those requests through `function_value`, while its direct-block compatibility route calls `prepare_closure_call`. Releasing the large runner preserves a native host frame and is not equivalent to source-owned suspension. |
| Synchronous evaluator host | `evaluate` also handles yielded callback arguments through `callback_argument`, function effects through `function_value`, function leaves through `function_value_leaf`, and `LowerBody` through `arrow_block_result`. Source-owned scheduling does not replace these compatibility entry points automatically. |
| Disposable definition checking | `closures::check_closure_at` and `check_factory_chain` call expression/function/body lowering on cloned checking state. Direct factory chains already use a local worklist, but their helper crossings can reenter other lowerers. Definitions are validated even when runtime effects never execute; checking order, diagnostics and scratch identities must remain isolated. |
| Lifted closure-body materialization | `function_values::materialize_closure` lowers a cloned child through `evaluate`, `function_value` or `arrow_block_result`, then publishes real lifted function IR and `MakeClosure`. Its result ownership and publication differ from disposable checking state. |

At #447 a remaining `arrow_block_result → sequence` crossing could still enter
another source cursor, whose body could repeat an expression/function route.
The converted argument/effect/callback, callee, concise-result and runtime-factory
routes at that checkpoint did not by themselves eliminate these composed native
cycles or the separately owned checking and materialization activations.

The original eight-active-block, eight-active-function-result and
sixteen-active-function-value budgets remain admission constraints. They do not
constitute architectural stack independence. Effect-free signature, callback-origin,
stored-kind and Promise fact queries retain their separate analysis ownership.
The async IR splitter and runtime Promise job queue also remain separate phases.

**This historical inventory was not a mechanically complete function-level SCC
graph.** The final audit below also considers helper, checking and diagnostic
edges; neither inventory supports a blanket claim that the compiler is acyclic.

## Final in-review effectful source ownership

The dependent units after the [#448](https://github.com/tseguine/grunkalunka/pull/448)
documentation checkpoint complete the maintained effectful-source circuits and
remove their superseded synchronous drivers. The maintainer authorized continuing
the native PR stack through this remaining refactor; registration, hosted checks
and merge status remain separate from source ownership and local test evidence.

| PR | Completed boundary |
| --- | --- |
| [#449](https://github.com/tseguine/grunkalunka/pull/449) | Separate declaration admission, initializer selection and binding publication without changing their order. |
| [#450](https://github.com/tseguine/grunkalunka/pull/450) | Prepare disposable checking children and complete their success-only metadata/source-cost publication. |
| [#451](https://github.com/tseguine/grunkalunka/pull/451) | Schedule block-factory `.call` through the general local-result continuation, preserving direct-block shape selection and outer validation priority. |
| [#452](https://github.com/tseguine/grunkalunka/pull/452) | Schedule ordered ordinary declaration initializers, including multiple locals and function/Promise/managed modes, with per-local publication. |
| [#453](https://github.com/tseguine/grunkalunka/pull/453) | Put declaration initializers in `for` headers on the same source-owned path. |
| [#454](https://github.com/tseguine/grunkalunka/pull/454) | Prepare every known-local function result through typed stages; remove the optional-call compatibility fallback while preserving earlier definition/resource errors and rejected public optional syntax. |
| [#455](https://github.com/tseguine/grunkalunka/pull/455) | Enter disposable local-callee definition checks as owned `Lowering` children on the same source cursor. |
| [#456](https://github.com/tseguine/grunkalunka/pull/456) | Schedule declaration closure checking and publication, retaining checked aliases and identity-signature ordering. |
| [#457](https://github.com/tseguine/grunkalunka/pull/457) | Share ordered factory arguments with const metadata declarations without an additional consumer Function16/Local8 token or forced materialization; argument-owned admissions remain unchanged. Queue declaration advancement after zero-argument completion. |
| [#458](https://github.com/tseguine/grunkalunka/pull/458) | Retire unreachable synchronous statement/declaration/return dispatch and obsolete adapters; source leaves contain only intrinsic, erased or rejected cases. |
| [#459](https://github.com/tseguine/grunkalunka/pull/459) | Separate metadata/local-result factory preparation and returned-definition checking checkpoints, retaining the original fallback snapshot and restoration ownership. |
| [#460](https://github.com/tseguine/grunkalunka/pull/460) | Separate real lifted-child admission, preparation, body completion and publication, preserving shared signatures and late capacity behavior. |
| [#461](https://github.com/tseguine/grunkalunka/pull/461) | Schedule real lifted bodies and non-call Function constructors as owned source children; parent restoration precedes real IR publication. |
| [#462](https://github.com/tseguine/grunkalunka/pull/462) | Schedule identity declaration materialization with its original signature lookup and no extra function-value charge. |
| [#463](https://github.com/tseguine/grunkalunka/pull/463) | Queue evaluator resumptions on the main source loop, removing direct restart paths including zero-argument expression-factory completion. |
| [#464](https://github.com/tseguine/grunkalunka/pull/464) | Schedule metadata/local-result factory checks and returned-definition checks with typed token/binding ownership; restore the caller before declaration publication or source-owned materialization. Retain the selected direct-block callee, removing its synchronous fallback structurally. |
| [#465](https://github.com/tseguine/grunkalunka/pull/465) | Share the existing source-runner loop with a test-only checking root, preserving its normal error unwind without adding a synthetic statement or value charge. |
| [#466](https://github.com/tseguine/grunkalunka/pull/466) | Delete the obsolete synchronous evaluator, function/factory/callback, checking and materialization drivers and their Region lease. Preserve the complete optional-helper state oracle as frozen fixtures tested through the production checking child. |

The final retirement is published as
[`67400f9`](https://github.com/tseguine/grunkalunka/commit/67400f91882124e021f4bf30157930bb91acba31)
in [#466](https://github.com/tseguine/grunkalunka/pull/466), based on that runner
prerequisite. Its source tree `9ea19ddd1ff1dbd0d39c46216d13a22747130735` matches
independently tested local `4bef43c`.
It deletes `evaluate`/`EvaluationLease`, synchronous function-value/callback and
factory drivers, `arrow_block_result`, `check_local_closure` and its two drivers,
and the compatibility real-body materializer. Their replacement is the production
source machine; there is no retained test interpreter. The old lease test is
replaced with a test of actual owned-cursor Region transfer, completion and
diagnostic state.

The optional helper oracle preserves all 21 complete legacy rejection states from
immutable `511db645`: original source, active-result provenance, exact diagnostic,
admission/source counters, slots, SSA values, Region contents and shared signature
identities. The staged test uses the real checking child, not a copied checker.
Public `ChainExpression` rejection and this extracted inner-call helper contract
remain distinct evidence categories.

### Composed native-call boundary

Top-level function lowering calls `sequence`, then `control_statement_iterative`
and `run_source_cursor`. The source loop alone drives
`advance_owned_expression`, which calls `run_evaluation`. The evaluator returns a
typed request before any checking, materialization or source-body child enters.
`Next::ResumeExpression` queues later evaluator work with its caller context;
completed declaration initializers likewise queue the next declarator.
The only other source-runner entry is the test-only checking root.

Owned `EvaluationCursor` and `Lowering` activations may coexist on the heap. That
is different from nested native runner invocations. The test-only native evaluator
entry guard now rejects a second active runner, while source-cursor and individual
handoff counters retain their route-sensitive coverage. Error unwind restores
child parents before releasing resources or bindings owned by those parents.

The [final validation report](lowering-refactor-validation-2026-11-03.md#call-graph-audit)
records the completed composed graph and value-flow audit. The baseline's 48-node
effectful compatibility component is absent after retirement. The final syntactic
graph retains a seven-node factory/declaration component and a 25-node fact-query
component, with different source arguments for their native nesting:

- Declaration closure publication returns only `Complete`, `Materialize` or
  `CheckClosure`, and the consumer receives that exact result. This excludes the
  return edge into the distinct `Factory` argument phase. `Complete` queues
  `AdvanceDeclaration`; it does not directly restart declaration selection.
- The query component contains 11 named function bodies and 14 closures. All 11
  bodies have unchanged normalized tokens from master `926a4aa`. The reviewed
  certificate contracts two generic shape walkers and fixed lexical callbacks
  into nine semantic helpers with 17 dependencies. Every cycle increases the
  guarded `calls` argument; the same-count edges and the finite 81-state
  overapproximation are acyclic. This bounds helper nesting under the reviewed
  source premises, without claiming a native-byte bound or bounded total work.

The graph retains unresolved and conservative calls explicitly. Its typed
variant exclusion and query-ranking certificate do not establish that all compiler
phases, dependencies or automatic destruction are acyclic. Source routing,
diagnostic and 2 MiB boundary tests remain separate evidence.

### Final local receipt and limits

The retirement tree passed 589 compiler unit tests, 10 compiler integration
tests, strict all-target compiler Clippy, formatting and 106 impact-planner
regression tests. All 7,487 frozen compilation cases match: 6,033 admitted and
1,454 rejected. The 13,554 compared output files comprise 13,520 artifact/diagnostic
files and 34 per-matrix result summaries. WAT, Wasm and diagnostics remain
byte-identical; only the predeclared `expected=observe` result metadata is
normalized after confirming the exact frozen outcome. The 34 matrices retain
their own historical oracle revisions across the series; supplemental matrices
were frozen at later checkpoints, rather than all directly on starting master.
All 106 CONTROL records preserve private IR/WAT/Wasm/diagnostic equality.
Runtime verification passed 136 closure,
9 module-callback, 9 managed-closure-input, 9 managed-function-value and 1 storage
tests. All 426 async-suspension tests passed with no failures, ignored or filtered
tests. The source implementation, scoped audit and bounded cost study are
complete in this in-review series.

The [cost report](lowering-refactor-validation-2026-11-03.md#compiler-time-and-process-memory)
retains 840 primary process observations across 60 cases. Across the case-specific
median paired candidate/baseline compile-time ratios, the median is 1.011 and the
unweighted geometric mean is 0.990. Differences between each revision's per-case
median whole-process peak RSS range from -32 to +252 KiB. A separate 320-process,
16-case confirmation has median paired ratios from 0.907 to 1.161. The charged JS
limit rejection changes from 1.579 to 0.937; the TS lifted-capacity rejection
changes from 1.251 to 1.072 and remains a possible modest increase. Individual
affine-control ratios span 0.226–3.275, showing substantial shared-host uncertainty.
The experiments retain their different batching and process-memory boundaries;
their memory levels are not pooled. These observations establish neither
performance equivalence nor a general speedup or absence of smaller regressions.

Neither a local pass nor native stack membership establishes hosted exact-head
CI, native macOS execution or integration into master. The validation report must
keep measured host/workload limitations and the existing platform-rehearsal
baseline limitation explicit. Existing source admission, generated-region,
function-result, native-stack and guest stack/fuel budgets remain separate.

## Ownership required across each suspension

| Owner | State retained in the final implementation |
| --- | --- |
| `EvaluationCursor` | Original Oxc expressions, requested modes, pending `Work`, typed values, source nesting charge and owned unfinished Regions, including the output root |
| Call/argument continuation | Source call/callee, argument index and left-to-right `ValueId` snapshots, signature/result type, pending await site and captures; factory arguments must finish in caller scope before fresh formals are published |
| Block-arrow activation | Return type, loop/await flags, inline and statement depths, visible bindings/closures, source charging, body Region and its completion contract |
| Local factory activation | A metadata preparation owns no result token and restores caller bindings before materialization. General block/concise results own the eight-result admission and caller binding/source-charging checkpoint; `FactoryExpression` owns result-depth restoration and any direct-callee charge. |
| `EvaluateFactoryValue` | General concise body expression, inherited caller statement depth, installed factory bindings, result admission and body function-value charge; completion restores the caller and delivers the body's existing `ValueId` in the original Region |
| `EvaluateFunctionValue` | Suspended evaluator, original Region handoff, caller statement depth, function-value admission and aggregate result-depth checkpoint; `FunctionValueResume::{Effect, Callback, Callee}` selects effect completion or callback/callee value delivery |
| `EvaluateRuntimeFactoryCallee` / runtime `FactoryArguments` | Runtime-result admission and validated call/signature metadata span callee construction, ordered arguments and final emission. The callee owns ordinary function-value admission with inherited result depth; completion releases that charge before arguments and retains its `ValueId` snapshot. |
| Declaration continuation | Source declaration/index, statement depth, current local and owned Region. A value, metadata arguments, checking or materialization must finish before publication and queued next-local selection. The metadata consumer adds no result or function-value token; each argument retains its own admission policy. |
| Disposable checking child | Boxed parent `Lowering`, original closure and factory-depth context. Success publishes source costs and checked metadata; scratch IR is discarded. Shared signature interning is not rolled back on error. |
| Real materialization child | Boxed parent and prepared lifted-closure state, body Region and result policy. Completion restores the parent, retains original nested-pool publication order and emits the real lifted closure. |
| Factory-check continuation | Original metadata/local-result selection, any Local8 token, independent fallback closure snapshot, or fresh factory binding checkpoint. Returned definitions finish checking under those fresh bindings; consumers run only after caller restoration. |
| Queued evaluator resumption | Owned evaluator, requested mode and caller statement depth, scheduled after completed children restore their resource/depth checkpoints. |
| Source cursor | Pending source-body request, lexical scope, abrupt `Completion` and the typed continuation consuming the completed Region or expression result |

Every pending value and Region needs one explicit owner. A suspension cannot
leave a mutable caller-Region borrow inside another native call. Diagnostic
unwinding must restore nested contexts in reverse order and preserve the original
first error. Preserve the existing point at which a child Region, formal slot or
result value is allocated, as well as the source order of effects.

## Continuing evidence and broader scope

1. **Preserve the completed source ownership contracts:** retain callee-before-
   arguments order, snapshots, complete signatures, distinct charges, checking
   publication, lifted identities and first errors as new source forms are added.
   Add a typed continuation at the existing scheduler boundary rather than a
   synchronous compatibility route.
2. **Keep the audit scoped:** L3/L4 effectful-source ownership and L5 retirement
   are complete in this implementation. L6 consolidation of signature, callback,
   stored-kind and Promise queries remains optional. New dynamic Get/Set/Call,
   general source exceptions and GC-aware async identity have their own designs
   and execution gates.
3. **Retain measured closure evidence:** preserve the final composed graph and
   value-flow rationale, exact-head test receipts and compiler-time/peak-memory
   observations. A bounded workload measurement is not an asymptotic or
   whole-compiler stack proof; the wider #281 acceptance invariant remains open.
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
   residual scope before broadening the completed source-lowering claim to other
   compiler phases or the runtime.

The observation tests must not require positive native reentry forever. Each
converted family should demonstrate its own explicit handoff and drive avoidable
reentry down through ownership changes, without increasing native stack reserves
or narrowing accepted source syntax.
