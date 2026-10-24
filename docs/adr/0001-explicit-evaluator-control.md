# ADR 0001: Explicit control state for expression lowering

Date: 2026-10-24

Status: Accepted; maintained expression evaluator implemented with explicit pending work.

## Context

The compiler lowers the original Oxc AST into a [typed region IR](../ir.md).
Number evaluation, Predicate evaluation, discarded evaluation and arithmetic
operand conversion form one mutually recursive component. Comma prefixes and
Boolean-to-Number bridges can re-enter evaluation indirectly.

A deferred Boolean-bitwise candidate twice exhausted the native stack on an
existing [554-byte depth fixture](../../tests/compatibility/numeric-bitwise-binary.mjs)
that should receive a bounded depth diagnostic. The
maintained baseline rejects that source normally. A later helper extraction
reduced one measured frame but increased the aggregate repeating call path by
832 bytes; it failed its measurement prerequisite and received no third CLI
replay. This identifies fragility when extending recursive lowering, rather than
establishing a native-stack failure in the maintained compiler or a complete
causal model of stack usage.

We need to extend language support without making source admission depend on
host stack reserves or incidental compiler-frame layout.

## Decision

Replace recursive expression-evaluator control with an explicit loop over typed
pending work and safely owned child-region state. Migrate the complete Number,
Predicate, Discard and arithmetic-operand component together. These are internal
lowering requests, not new JavaScript value types or public IR types.

Retain the current typed region IR, verifier, emitter and public compile API.
Implement the migration separately as maintained code; do not promote or copy the
disposable experiment. Every admitted evaluator node must use the new mechanism;
a recursive legacy fallback is not an acceptable completion boundary.

The representation for general JavaScript values remains a separate decision.

## Evidence and limits

A disposable experiment based on commit
`0222ed214accf5764d5eda552648b59caac66d39` used the real Oxc parser and semantic
analysis, lexical SymbolIds, original declaration scaffold, verifier and emitter.
It preserved exact private IR, WAT and Wasm for 29 selected programs and full
first diagnostics for 14 sources; six other shapes were explicitly outside its
slice. Original JavaScript and both baseline/candidate Wasm passed 14,848
comparisons over 7,424 argument pairs in Node. Five TS cases had explicitly
written JavaScript counterparts. Independent replay reproduced these results.

Selected controls included Number/Predicate depth 127/128/129 and retained TS
wrapper boundaries. Instrumentation observed one active candidate evaluator
entry, no legacy evaluator entries, and a maximum of 130 pending tasks in these
samples. These counts are not native-stack measurements or a general heap bound.

The experiment did not cover every admitted call, logical/comma expression,
update or compound assignment. It establishes feasibility for the selected
ownership and ordering cases, not a completed migration. It supplies no Wasmtime,
performance, cross-platform or out-of-memory equivalence result.

## Alternatives

| Option | Advantages | Disadvantages and disposition |
| --- | --- | --- |
| Keep recursive lowering; extract local helpers as needed | Familiar control flow, small local changes, no explicit pending-state machinery. | Frame growth remains coupled to syntax depth; a smaller individual frame need not reduce the repeating path. Retain only until the complete migration passes its gates; do not resume the failed local extraction. |
| Add an admission/depth preflight only | Can detect selected resource violations before later work. | A separate scan can change first-error precedence, and preserving that precedence requires modeling ordered admission. It does not remove recursion for admitted inputs. May complement later resource work, but does not replace this migration. |
| Raise host stacks, change build profiles or reduce source limits | Can provide a small implementation effort for a particular host or workload. | Stack/profile changes do not establish evaluator independence from native frames; reducing limits changes admission. These approaches add host/build assumptions or weaken the existing validation contract, so are not the selected remedy. |
| Flatten the AST or replace the IR broadly | Could make more pipeline walks iterative and support future analyses. | Expands the migration into source representation, verification and emission before that work is needed. Keep the existing IR for this change; assess other walks separately. |
| Explicit pending work with owned region state | Makes evaluation modes, snapshots and scheduling explicit; uses safe Rust and retains the existing backend. | Adds heap work state, temporary region ownership and more continuation variants to review. Selected, with complete coverage and preservation gates below. |

## Required migration invariants

- Preserve the [source contract](../compiler.md), all existing source budgets,
  SymbolId resolution, slot kinds and publication only after initialization.
- Preserve mode-specific depth checks and diagnostics. Discard/arithmetic routing
  classifies before choosing an evaluation mode; Predicate fallback enters Number
  at the same depth. Retained wrappers each charge their original depth.
- Preserve first-error phase, span and text, operator/target validation order,
  left-before-right evaluation, once-only effects and captured values. Compound
  assignments load their target before evaluating the RHS.
- Preserve ValueIds, RegionIds, spans, operation order and region creation points.
  A conditional completes its consequent before allocating its alternate region.
- Preserve exact current-baseline WAT/Wasm and retained rejection observations.
  Cover every admitted call/intrinsic, assignment/update, wrapper, unary/binary,
  logical, conditional and sequence path, including indirect Discard/bridge edges.
- Verify balanced, non-reentrant evaluator entry on success and error, with
  independent private-IR and runtime cases. Run the current depth controls and
  full offline checks; inspect the complete diff and independent review evidence.
- Introduce no unsafe ownership escape, RefCell workaround, new source cap,
  stack/profile change, dependency or public IR/ABI change for this migration.

## Maintained implementation

The production evaluator is maintained separately in
[`lower/evaluate.rs`](../../crates/grunkalunka-compiler/src/lower/evaluate.rs), with
original statement/declaration entrypoints forwarding requests. It covers the
complete evaluator component, including calls, assignments/updates, logical/comma
expressions, all selected bridges and wrappers. Its root region remains borrowed;
a LIFO vector owns unfinished children, and continuations move completed regions.
There is no recursive legacy evaluator fallback or copied experiment path.
Private characterization and entry observations complement the full existing
corpus artifact and diagnostic invariance checks; the IR/backend and source
admission remain unchanged.

## Consequences and remaining work

This decision adds explicit heap state and implementation complexity in exchange
for source evaluation control that does not recurse through evaluator requests.
The actual allocation overhead and compilation costs need measurement; no speed
or footprint improvement is assumed. Ordinary allocation failure equivalence is
not established.

Oxc parsing/semantic traversal, statement and loop lowering, recursive shape
classification, IR verification/emission, IR debug rendering and destruction
remain separate recursion concerns. Completing this migration will not establish
whole-frontend liveness. Deferred bitwise/shift candidates remain deferred until
a separately verified implementation supports reconsideration; their recorded
failures and exhausted local repair budget are preserved.
