# ADR 0020: Shared source-lowering cursor mechanics (proposal and first pilot)

Date: 2026-11-01

Status: proposed; first bounded source-control migration is under review.

## Context

The maintained compiler lowers original Oxc syntax and semantic identities into
verified typed region IR. [ADR 0001](0001-explicit-evaluator-control.md) made
expression evaluation iterative, and
[ADR 0016](0016-shared-function-selection-cursor.md) unified function-valued
selection with that evaluator. Those decisions and the existing IR remain in
force. [ADR 0019](0019-wasmgc-runtime.md) owns the accepted GC direction.

An iterative function can still call another iterative function that re-enters
the first, rebuilding a source-dependent native call stack. The current source
statement code has separate work loops for sequences, `if`, `for`,
`while`/`do-while`, try/catch, try/finally and concise function bodies. They
carry much of the same mechanics but legitimately differ in source semantics.
The native-stack acceptance invariant is tracked by
[stack hygiene #281](https://github.com/tseguine/grunkalunka/issues/281).
This proposal addresses structural maintenance and composability, not just a
currently reproduced native stack failure.

## Survey of recurring and upcoming patterns

| Pattern / observed sites | Shared invariant | New-language pressure / migration candidate |
| --- | --- | --- |
| `lower.rs::sequence`, `if_statements.rs`, `for_statements.rs`, `loop_statements.rs`, `finalizers.rs`, `finally_chains.rs` | Descend, suspend parent, resume child and preserve original source evaluation order | Labels, richer loops, switch, destructuring and nested exception handling need a composable statement continuation contract |
| `evaluate.rs` and its managed/own-keys subcursors; `function_values.rs`, `callbacks.rs`, `closures.rs` | Expression result delivered once, in the requested value mode, after side effects | Dynamic Get/Set/Call, optional chaining and more value kinds should not each invent a recursive evaluator |
| `Lowering` fields `statement_depth`, `in_loop_body`, `return_type`, `source_charging`, `inline_depth`; branch/loop/finalizer handlers | Dynamic context is restored on completion and errors; branch-local bindings must not leak | Reentrant functions, lexical loops, catch bindings, async suspension and future control constructs need typed, frame-owned context |
| `Completion` algebra and `Region.exit`; `try_statement` and loop handlers | Normal, return, throw, break and continue channels compose in source order; finally can override a pending completion | Catchable source-level TypeErrors from runtime-dependent operations must use the same completion path, not a host trap (see [#355](https://github.com/tseguine/grunkalunka/issues/355)) |
| Source-depth checks in sequence, singleton blocks, if/for/while; statement count and expansion quotas | Check ordering, first diagnostic, source charge and generated-region charge are distinct | Each new handler must honor the same limits without duplicating their policy; native stack and guest stack/fuel are independent |
| Saved output `Region`, child-region ownership and `Op::Block`/branch/loop construction | Parent operations are neither lost nor emitted twice; child regions remain owned across suspension | More conditional regions and compound operations need a consistent handoff convention without changing the typed IR |
| `storage.rs`, `callbacks.rs`, `function_values.rs`, `managed_calls.rs`, Promise/source-kind queries | Read-only source facts do not emit effects; fixed-point and union facts are conservative | Runtime-dependent values, joined input kinds and callback origins will need incremental/worklist analyses; *do not* force them into an effectful lowering scheduler |
| Async splitter and backend continuation graph; closure/cell ownership | Source lowering, IR transformations and runtime Promise scheduling are separate phases with different state and identity | Traced GC references across suspension ([#356](https://github.com/tseguine/grunkalunka/issues/356)) require an explicit lifetime design, not reusing a compiler cursor as a runtime scheduler |

The repetitive part is **ownership, scheduling, admission and context**. The
different parts are **language-specific transitions and result semantics**.
An abstraction that merges both prematurely would be less maintainable than
the existing handlers.

## Proposed decision

Adopt a **composable, heap-owned lowering cursor** as a direction, developed in
small parity-preserving steps:

1. Begin by sharing source-control request shapes (`enter same-kind node` vs
   `lower child statement`), source-admission policy and ownership transfer
   of the current `Region`. Preserve existing statement-specific frames and
   completion assembly. This pilot is represented by
   `lower/source_control.rs` and its users in the if/for/while/do handlers.
2. Make lexical scopes, saved dynamic modes and restoration points explicit
   continuation-frame data. Prefer typed context boundaries over directly
   mutating and repairing global `Lowering` flags on many code paths.
3. Explore a single statement-control dispatcher **only after** mixed-kind
   call-graph/SCC analysis demonstrates its need. A lowerer handler should be
   able to schedule work and yield rather than synchronously call another
   dispatcher. Try/catch/finally and inline-closure reentry are essential cases.
4. Reconcile the existing expression evaluator's request/result loop with
   statement control through typed suspend/resume boundaries. Reuse ADR 0001
   and 0016 mechanisms; do not implement a competing evaluator.
5. Keep effect-free facts and backend worklists separately organized unless
   concrete duplication warrants shared infrastructure. Introduce a
   source-operation normal/throw seam when executing the first runtime-dependent
   property/call feature, consistent with existing exception propagation.

These are proposed migration stages, not permission to widen language admission
or replace the IR in advance of evidence. The first stage is specifically a
mechanism extraction, **not** proof that all cross-handler recursion is gone.

## Scope and invariants

- The original Oxc AST, SymbolIds, accepted numeric fast paths, typed region
  IR, verifier, emitter, WasmGC choice and private ABI remain unchanged.
- The source statement-depth limit of 64, block/arrow expansion limit of 128,
  statement/local budgets and first-diagnostic ordering are unchanged. Source
  charging and generated-region ownership must remain distinguishable.
- Root source statements already charged by `statement_inner` must not be
  charged twice when a nested-control cursor starts.
- Entry to a child preserves lexical visibility, loop targets, return type,
  pending completion channels and left-to-right effects.
- Do not add a recursive fallback for an admitted shape. A source-dependent
  cross-subsystem reentry is a reason to migrate the entire SCC, not to add
  another local depth check or increase the Rust thread stack.
- Dynamic Get/Set/Call, general source exceptions, async-GC rooting, arbitrary
  parallelism and a new CFG/SSA IR are non-goals of this pilot.
- This compiler work scheduler is **not** the runtime JavaScript event loop.

## Alternatives

1. Continue one-off iterative rewrites. Small changes, but repeated context
   guards and cross-handler reentry remain easy to overlook. Not preferred for
   mechanisms common to several handlers.
2. Immediately replace all source lowering with one universal interpreter-like
   dispatcher. Could remove cross-handler recursion at once but risks
   destabilizing diagnostics, output identity, specializations and cost without
   evidence. Deferred.
3. Generate handlers from generic AST visitor macros. Reduces boilerplate but
   obscures nonlocal evaluation order and context ownership; does not itself
   remove native recursion. Rejected as the organizing principle.
4. Flatten all syntax into a new CFG/SSA before lowering. Could help later
   optimizations; not justified by a shared continuation problem alone and
   would conflict with proven IR paths. Deferred.
5. **Chosen pilot:** share exact small mechanics and observe where next feature
   work still requires hand-managed continuation state.

## Consequences and evidence

The pilot removes the copied control-task enum, source-admission check and
initial region detachment from three source-control handlers, as well as
duplicated admission checks in block and sequence entry. It leaves special
frame data and completion assembly local, avoiding an artificial “one frame fits
all” abstraction. It adds a small `source_control` module.

The immediate test gate is preserved compiling and emitted bytes for supported
sources, unchanged source rejection diagnostics, and 2 MiB native-stack
admission/rejection probes on Windows/Linux. Existing maintained tests include
nested-if, nested-loop, nested-for and source-block boundaries. The pilot is
**not yet validated** by a local Cargo build or `cargo fmt` in this session;
GitHub CI must run on its exact PR head, with any failures corrected before
merge. Future larger migrations require a before/after source matrix, native
stack measurements for composed mixed-kind nesting, and compiler performance
and peak-memory comparisons; passing one static code inspection is insufficient.

## Explicit boundary

This introduces common compiler **source-control mechanics**, not a completed
universal lowering state machine. Statement-specific continuations and the
existing expression evaluator remain separate today. The next decision is
whether a unified statement dispatcher is worth the integration cost after a
mixed-kind SCC survey and a bounded cross-handler vertical slice.

## Related work

[Native-stack hygiene #281](https://github.com/tseguine/grunkalunka/issues/281),
[for-let closure correctness #334](https://github.com/tseguine/grunkalunka/issues/334),
[source semantic operation steering #355](https://github.com/tseguine/grunkalunka/issues/355),
[GC-aware suspension steering #356](https://github.com/tseguine/grunkalunka/issues/356),
[JavaScript engine steering](../javascript-engine-steering.md), and
[IR course correction](../ir-course-correction.md).
