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

The published tip is
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

## Residual native reentry circuits after this series

The inspected synchronous routes remaining outside the converted source-owned
paths are:

| Route | Current calls and ownership boundary |
| --- | --- |
| Source declarations | `lower.rs::declaration` handles `const`, multiple declarators and function/Promise/managed initializers outside `scalar_let_initializer`. It calls `factory_result`, `function_value`, `evaluate` and scalar/Promise adapters synchronously. Preserve per-declarator admission, argument effects and binding publication. |
| Concise/local function results | `closures::factory_result` evaluates arguments through `evaluate_factory_argument`; `function_values::local_function_result` retains synchronous argument and concise-result/body paths for compatibility callers. Preserve caller captures, fresh formals, signatures and result charges. |
| Runtime function results | `function_values::runtime_function_result` constructs its callee through `function_value` and evaluates callback/scalar/surplus arguments through synchronous helpers. Preserve the callee snapshot before arguments and the complete result signature. |
| Calculated/async callees | The non-slot arm of `evaluate::prepare_closure_call` calls `function_value`; awaited/start and remaining calculated-callee routes reach that helper outside the direct block-factory bridge. Their callee-construction phase is distinct from the migrated `ClosureCall` callback arguments. |
| Synchronous evaluator host | `evaluate` still handles yielded callback arguments through `callback_argument`, function effects through `function_value`, and `LowerBody` through `arrow_block_result`. Source-owned scheduling does not replace these compatibility entry points automatically. |
| Disposable definition checking | `closures::check_closure_at` and `check_factory_chain` call expression/function/body lowering on cloned checking state. Definitions are validated even when their runtime effects never execute; checking order, diagnostics and scratch identities must remain isolated. |
| Lifted closure-body materialization | `function_values::materialize_closure` lowers a cloned child through `evaluate`, `function_value` or `arrow_block_result`, then publishes real lifted function IR and `MakeClosure`. Its result ownership and publication differ from disposable checking state. |

A remaining `arrow_block_result → sequence` crossing can still enter another
source cursor, whose body may repeat one of the expression/function routes.
The source-owned argument/effect/callback continuations do not by themselves
eliminate these composed native cycles or the separately owned checking and
materialization activations.

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
| Factory activation | Function-result admission, caller binding/source-charging checkpoint and result-depth checkpoint; the callee route also owns its function-value charge until completion/unwind |
| `EvaluateFunctionValue` | Suspended evaluator, original Region handoff, caller statement depth, function-value admission and aggregate result-depth checkpoint; `FunctionValueResume` either completes an effect or supplies one callback value to the pending call |
| Source cursor | Pending source-body request, lexical scope, abrupt `Completion` and the typed continuation consuming the completed Region or expression result |

Every pending value and Region needs one explicit owner. A suspension cannot
leave a mutable caller-Region borrow inside another native call. Diagnostic
unwinding must restore nested contexts in reverse order and preserve the original
first error. Preserve the existing point at which a child Region, formal slot or
result value is allocated, as well as the source order of effects.

## Remaining L4/L5 work and evidence

1. **Remaining function and source circuits:** migrate concise returned
   factories, runtime/calculated/async callees and the source consumers listed
   above. Keep the converted factory/effect/callback arguments covered for
   callee-before-arguments order, snapshots, complete signatures and first errors.
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
