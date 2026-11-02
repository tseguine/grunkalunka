# Cross-phase source/expression lowering reentry

Date: 2026-11-02. Status: working implementation inventory, **not** a completed
cross-phase stack-independence claim. Complements [ADR 0020](adr/0020-source-lowering-control-kernel.md),
[engine steering](javascript-engine-steering.md#source-lowering-architecture-completed-base-and-remaining-work)
and [native-stack issue #281](https://github.com/tseguine/grunkalunka/issues/281).

## The remaining native reentry circuit

Source statements and lexical control-flow already share a heap-owned
`lower/control_cursor.rs::Cursor`. The expression evaluator owns its separate
`Work`/value/child-region stacks and runs synchronously.

The direct, source-dependent circuit inspected in `master` is:

1. `lower/control_cursor.rs::control_statement_iterative` dispatches
   `Next::Expression`, or delegates an admitted ordinary statement through
   `lower.rs::statement_inner`.
2. `lower/evaluate.rs::evaluate` schedules typed `Work` operations. A
   `Work::LocalCall` with a block-bodied arrow calls
   `lower/closures.rs::arrow_block_result` after evaluating ordered arguments
   and binding fresh formal slots. This branch is not suspended on its worklist.
3. `arrow_block_result` snapshots lowering modes, bindings, inline depth and
   closure context, constructs a function-body Region and calls
   `lower.rs::sequence` synchronously.
4. `sequence` enters a second `control_statement_iterative`. A statement
   inside the block can itself evaluate a local function call, returning to
   step 2, and the native Rust call stack grows with the composed source
   nesting. The existing `active_block_calls < 8` budget bounds this one
   path, but is an admission restriction rather than architectural stack
   independence.

There is a related circuit through
`lower/function_values.rs::function_value_leaf ->
local_function_result -> arrow_block_result`. Returned function constructors
also carry the separately enforced eight-function-result and sixteen-active-
function-value budgets. Effect-free signature, callback-origin, stored-kind
and Promise fact queries are separate analyses; they should not acquire
effects or reuse the source execution scheduler.

**These are identifiable source-call circuits, not a mechanically complete
function-level SCC inventory.** Indirect calls, errors and other helper edges
still require a graph audit before declaring the entire source lowering phase
acyclic. The async IR splitter and runtime Promise job queue have different
continuation ownership and are not part of this compiler circuit.

## Actual crossing state that must become heap-owned

| Current owner | Live information that a suspension must preserve |
| --- | --- |
| `evaluate.rs` | Original Oxc expression, requested `Mode`, pending `Work` tasks, typed values, source nesting charge and owned child Regions, including the current output Region |
| `Work::LocalCall` | Call node, source callee, argument index and ordered evaluated snapshots, physical result `Type`, pending await site and closure captures |
| `arrow_block_result` | Return type, loop/await flags, inline and statement depths, visible binding/closure checkpoints, expansion count, function source body and its completion contract |
| `control_cursor` | One nested source-body task, lexical scope, original body Region, source-directed abrupt `Completion` (normal/return/throw/break/continue), and the continuation that consumes the completed Region |

A successful solution needs a single explicit owner for each pending value or
Region. The statement scheduler cannot keep borrowing an evaluator's
`&mut Region` while the evaluator itself is suspended inside another
native call. A source-body request should be a typed continuation, with
its original function-context restoration and error precedence preserved.

## Proposed L4 implementation sequence

1. **Measure:** keep a 2 MiB source-compiled admitted/rejected block-factory
   ladder. The new test-only `evaluator_tests::active_depth` and
   `source_cursor_tests::EVALUATOR_CROSSINGS` count source cursor entries that
   occur *while the evaluator is active*. Print the depth/entry/crossing
   readings; do not treat a sampled entry count as native bytes or a universal
   worst-case stack bound.
2. **Own evaluator state:** introduce a resumable expression cursor that owns
   its original pending-work/value/child-Region state. Keep the existing typed
   `Work` vocabulary. Make the caller's output Region transferable at a
   suspension boundary rather than holding an outstanding mutable borrow.
   Legacy callers continue to work until their SCC is migrated.
3. **First whole-cycle vertical:** have a block-bodied, numeric
   `Work::LocalCall` yield a `LowerBody` request to the *shared* source
   scheduler, not invoke `arrow_block_result` on the Rust stack. Resume the
   evaluator with the completed Region, validate the same function-body
   completion and construct the same `Op::InlineCall` value.
   Preserve argument evaluation, captured cells, `source_charging` and
   the original eight-block admission limit during the migration.
4. **Expand across the SCC:** returned function values and callback arguments,
   expression-mode variants, async block bodies and nested function-result
   construction require the same return protocol. Remove the matching native
   call fallback for each fully migrated circuit; don't add one-way adapters
   that silently reenter the large evaluator frame on another syntax shape.
5. **Acceptance:** maintain byte-identical supported artifacts and exact first
   diagnostics where this is a pure refactor, cross-engine source/runtime tests,
   2 MiB admitted and just-over-budget cases on Linux/Windows, no interpreter
   or source-admission broadening, and an observed decrease toward zero in
   evaluator-to-source native reentries on the converted test family. Only
   then can the relevant source-dependent SCC be considered eliminated.

The observation test intentionally does **not** insist on positive native
reentry forever: that would fail once the intended architecture is implemented.
It checks the existing source admission and explicit stack budget while logging
the crossings, which later L4 units must drive down through actual ownership
changes rather than raising stack reserves.
