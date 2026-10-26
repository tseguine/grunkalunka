# ADR 0011: Separate semantic values from storage and lifetime protocols

Date: 2026-10-26

Status: proposed

## Context

This proposal defines a compiler/runtime boundary, not a new heap or collector.
The inspection baseline is `087700f0ecc791e0c7e7dc58a52ca9819a10398d`.
Reconcile newer implementation work before accepting or implementing it.

[ADR 0002](0002-tagged-primitive-values.md) selects a four-kind primitive wire
ABI, while the [IR](../ir.md) distinguishes values from mutable source cells.
[ADR 0010](0010-bounded-internal-root-frames.md) supplies a separate synchronous
root-frame protocol. The [heap source profile](../heap-literals.md) currently
uses its own AST path for String/JSON operations rather than the shared IR.
These are useful bounded implementations, not a single general JS heap contract.

The IR course correction and roadmap L4/JSON work need to connect these pieces
without making every frontend construct understand tags, local widths, root
IDs, or frame tokens. This ADR leaves structured regions versus CFGs open.
Existing entry profiles, diagnostics, source budgets and wire ABIs remain intact.

## Decision

Separate semantic value facts from a backend-owned representation and lifetime
plan. Evolve existing machinery where possible; this does not require new crates
or a second fully materialized IR.

The semantic layer records operation meaning, established value kinds, ordered
effects, value snapshots and source-cell identity. Proven Number operations stay
specialized. TypeScript annotations remain metadata, not unchecked runtime facts.
Logical widening into a supported value union is not itself a requirement to box.

Representation lowering selects scalar/pair/handle forms, physical call layouts
and root protection. Source lowering must not compute physical widths or root
frame operations merely to express a branch, assignment or return. The selected
backend profile must explicitly reject semantic kinds it cannot represent.

For the first synchronous String/JSON integration, reuse ADR 0010 behind this
boundary. Mark operations that may allocate, collect, transfer ownership or leave
trusted guest execution. Before each collection point, protect every live managed
value, including evaluated temporaries and caller values, not just named locals.
A conservative live superset is allowed initially, but retaining everything must
not silently turn bounded repeated-allocation programs into quota failures.

Keep source-cell IDs, semantic values, runtime root IDs, frame tokens and host
resource handles distinct. Copying a value is not implicitly transferring its
owner's release obligation. String equality must not become root-ID equality;
future object identity and shared captured-cell identity must not become copying.

Host-visible ownership uses explicit validated transport and copy/retain/release
rules. Preserve current owned UTF16 readback. No borrowed guest address survives
an invalidating operation. The existing clear/repin sequence admits no intervening
collection or reentry. Suspension needs activation-owned roots; the synchronous
frame stack alone does not establish that contract.

## Scope and non-goals

This covers semantic/physical separation and synchronous temporary-root planning.
It does not select NaN boxing, WasmGC, reference counting, a tracing collector,
object layouts, general Promise values, or a public managed-reference ABI.

Preserve ADR 0002's primitive boundary, ADR 0010's checked tokens and quotas,
existing scalar specialization, and the heap host's terminal-error behavior.
No new language support follows merely from adding an IR kind. A source extension
needs its own admission, execution and differential evidence. Current JSON data
trees are not an alias-preserving JavaScript object graph.

## Alternatives considered

- Option A: Keep independent representation logic in each frontend/profile.
  - Advantages: Minimal immediate disruption; existing probes remain simple.
  - Disadvantages: New control-flow and call features repeat ownership decisions.
  - Disposition: Retain as the comparison baseline, not the proposed destination.
- Option B: One universal boxed value and one complete heap immediately.
  - Advantages: A uniform physical interface.
  - Disadvantages: Premature collector/ABI commitment; may lose scalar paths.
  - Disposition: Deferred until workload and lifetime evidence justify it.
- Option C: Semantic values plus an explicit backend lifetime plan.
  - Advantages: Reuses current protocols while separating their responsibilities.
  - Disadvantages: Requires effect contracts, verification and migration adapters.
  - Disposition: Proposed; prefer the smallest implementation that proves the seam.

## Consequences

Frontend features should become less coupled to representation, but this benefit
is unproven. The compiler gains a reviewable place to reason about protection and
ownership; it also gains another contract that can be implemented incorrectly.

A small effect classification and ordered operations are sufficient initially;
this ADR does not require effect SSA or whole-program escape analysis. Do not move
allocation or collection across effects simply because an operation returns a
value. Keep reference and legacy paths explicit during comparison, then remove
superseded logic rather than maintaining parallel production implementations.

## Verification and evidence

**Observed:** the linked ADRs and profile documentation were inspected at the
baseline. They describe existing tests; those tests were not rerun for this draft.
No candidate lifetime plan, speedup or managed-heap integration is established.

**Required before acceptance:** demonstrate one source-to-Wasm vertical slice
through the candidate semantic boundary, not only hand-built IR. Start with the
existing String/JSON expression profile; add only the bounded storage or branch
case needed to exercise competing live values. Compare with native JS, Node-Wasm
and Wasmtime, retaining existing failure/admission distinctions.

Show a saved temporary surviving another allocation, aliases surviving release
of unrelated roots, correct caller/result transfer, and reclamation across more
allocations than the simultaneous-root quota. Test malformed/stale tokens, quota
failures and terminal traps separately. For new unsupported source cases, use
explicit IR/protocol tests without counting them as source compatibility.

Verify protection at collection points independently of the lowering that emits
it. Run relevant heap/composition tests and the canonical `node scripts/check.mjs`
when prerequisites are available; report unrun gates. Measure allocation/root
counts, artifact size and execution cost against the pinned baseline.

Revise or reject this proposal if it merely relocates special cases, requires a
full collector before the first slice, or increases quota failures without a
justified tradeoff. Resolve the contract before broad migration; acceptance need
not wait for every profile to migrate.

## Explicit boundary statement

This introduces a semantic-to-storage/lifetime seam using existing runtime
protocols. It does not implement a general JS heap or make synchronous roots safe
across suspension. Collector selection, graph identity and escaping/async root
ownership remain explicit future decisions.

## Related work and follow-up

Related: ADRs [0002](0002-tagged-primitive-values.md),
[0008](0008-strict-core-wasm-composition.md) and
[0010](0010-bounded-internal-root-frames.md); [captured cells](../captured-cells.md).
Roadmap: L4, JSON integration and the semantic-IR experiment in progress.

Coordinate with proposed [0012](0012-guest-runtime-and-host-capabilities.md) and
[0013](0013-async-job-and-activation-ownership.md), without making completion of
all three a prerequisite for the synchronous experiment. Record the selected
representation plan and measured tradeoffs before expanding source coverage.

## Summary

Express values and effects once; let backend lowering own their physical storage
and protection. Prove that separation with current String/JSON machinery before
choosing a general heap, collector or replacement IR.
