# ADR 0020: Shared source-lowering cursor mechanics (proposal and first pilot)

Date: 2026-11-01

Status: proposed architecture for the remaining cross-phase work; its bounded
source-control pilot is integrated and verified (see checkpoint below).

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

## Cross-handler reentry seams to measure

The repeated direct-nesting optimizations do **not** yet establish a
stack-independent whole-program lowering entry point. In the inspected source,
these call paths can return to a previously entered subsystem:

| Candidate path (source files) | Why individual iterative loops are insufficient | Representative regression |
| --- | --- | --- |
| `sequence -> statement -> if_statement_iterative -> statement -> sequence` | A block inside an if body delegates back into sequence; alternating blocks/branches can reenter a suspended source cursor | Mixed nested block/if forms, boundary and over-depth, on a 2 MiB native thread stack |
| `statement -> for/while_cursor -> statement -> if/try/loop_cursor` | Same-kind loop chains are handled on a heap, but mixed-kind children reenter another large handler | Alternating for/while/if forms with breaks, continues, captured header bindings and exact output |
| `statement -> try_statement -> sequence -> statement -> try_statement` | Direct catch/finally chains have specialized iterative handling; intervening blocks/branches create a different path | Nested catch/finally with thrown and returned completion, precise handler/cleanup order |
| `statement -> evaluate -> function_value -> closure/body lowering -> sequence -> statement` | A value or call can enter a source function body and subsequently another expression evaluator | Factory/callback results with lexical captures, arithmetic suffixes and source-budget diagnostics |

These are **static call-path observations**, not a measured set of strongly
connected components or proof that any admitted case currently overflows.
Before a unified dispatcher, construct the composed call graph (including
helper callbacks and error paths), identify SCCs, and measure the accepted
depth/2 MiB case and just-over-budget rejection for each selected seam.

An eventual machine can represent a request, suspended parent continuation,
lexical context, owned output region and typed outcome explicitly. A handler
should either complete or schedule the next request; the engine resumes its
continuation without building another native Rust frame per source layer.
Its task vocabulary should be extended only for actual source transitions,
not copied wholesale from every existing local worklist.

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

## Integrated source-control checkpoint (2026-11-01)

The following bounded implementation stack was tested and integrated
into `master` through the combined #391 rebase merge. Predecessor PRs
were closed as superseded by the batch, not individually merged:

- [#381](https://github.com/tseguine/grunkalunka/pull/381): common source
  charge/admission checks, enter/child task vocabulary, owned Region handoff.
- [#382](https://github.com/tseguine/grunkalunka/pull/382) through
  [#385](https://github.com/tseguine/grunkalunka/pull/385): one heap
  scheduler for If/While/DoWhile/For, lexical blocks and raw sequences;
  existing statement families retain typed semantics and `Completion`.
- [#386](https://github.com/tseguine/grunkalunka/pull/386) and
  [#387](https://github.com/tseguine/grunkalunka/pull/387): catch and finally
  continuations enter the same machine; legacy direct-chain schedulers retire.
- [#388](https://github.com/tseguine/grunkalunka/pull/388) through
  [#390](https://github.com/tseguine/grunkalunka/pull/390): keep the scheduler
  small, place typed transitions in a separate module, give dynamic source
  modes an explicit context checkpoint, remove a now-obsolete singleton block
  path.
- [#391](https://github.com/tseguine/grunkalunka/pull/391): test-only
  active-cursor instrumentation and composed syntax probes for precisely the
  source-control boundaries that were previously distinct Rust frames.

**Verified scope:** #391's exact combined tip passed applicable hosted
Linux/Windows checks and the repository macOS verification gate before
rebase merge, and the caretaker confirmed the reviewed file contents in
`master`. The later [#394](https://github.com/tseguine/grunkalunka/pull/394)
expression refactor and [#396](https://github.com/tseguine/grunkalunka/pull/396)
owned source-cursor/for-loop extraction were also integrated with exact-head
checks and their own regression repairs. These establish the currently
implemented bounded source machine, not the full cross-phase architecture.
Full output/diagnostic parity, stack probes and runtime oracles remain gates
for *each future* code migration. The active remaining-unit plan is in
[engine steering](../javascript-engine-steering.md#source-lowering-architecture-completed-base-and-remaining-work).
The absence of synchronous recursion among these particular source-control
families is not yet evidence for the whole compiler. In particular:

1. `control_cursor -> statement_inner -> evaluate -> function_value_leaf ->
   local_function_result / arrow_block_result -> sequence -> control_cursor`
   can reenter the source machine through an *expression/function body* seam.
   A supported native stack budget is currently enforced for some inline and
   function-result depths. The next investigation should measure full
   cross-phase SCCs and their frame sizes, not only AST source depth.
2. Read-only signature, callback-origin, storage and Promise analyses use
   independent worklists; their results are fact queries, not compiler effect
   continuations. Unify only demonstrated duplicate semantics and preserve
   their no-source-effects contract.
3. The async IR splitter's continuation graph and runtime Promise job queue
   do not belong to the compiler source scheduler. Future traced-reference
   suspension must be designed and verified at its own lifetime boundary.

A future `LoweringMachine` API could expose a typed child-expression request
and a continuation receiving a typed value, allowing expression/factory leaves
to yield back to the same scheduler without Rust call-stack reentry. That
requires an explicit result-stack/region-ownership contract and must migrate
each actual SCC as a whole. It should not be claimed merely by replacing a
recursive call with another `Vec<Work>`.
