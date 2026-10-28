# ADR 0015: Keep checker hints separate from established runtime facts

Date: 2026-10-26

Status: proposed

## Context

This is a proposed contract for consuming checker information, not adoption of a
new frontend or implementation of an optimizer. Inspection baseline:
`57dc490447776c69092f8636c773e1e008b6b40e`.

The maintained compiler preserves annotations but does not trust them as runtime
facts. [Spike 004](../../spikes/004-tsrs-type-query/README.md) now queries pinned
tsrs revision `736a0e189939278c9878e5a3454f8126f55081fc` using an expression
selected by Oxc 0.155.0. It also observes a type on an AST with diagnostics. Its
fixture-specific span/text selectors do not establish general source or symbol
identity, and no queried type currently reaches production lowering.

[ADR 0014](0014-backend-value-layout.md) separates physical layout from verified
logical types. The next question is what can legitimately establish those types.
The existing Number entry ABI already explains many scalar paths; querying
`number` for them would not demonstrate an incremental TypeScript advantage.

## Decision

Treat declared and checker-inferred types as optional specialization hints.
Only an independently established semantic fact or a runtime guard may authorize
removing dynamic behavior. A successful checker query, or absence of checker
diagnostics, is not such evidence. Keep these categories distinct in the adapter
and any consuming analysis; a label saying "proven" without a derivation is not
sufficient. No public proof-object framework is required.

### Query identity and eligibility

Associate a query with an immutable source snapshot, file identity, JS/TS and
module mode, expression identity/range, checker revision and effective options.
Include ambient declarations, library inputs and resolution context when enabled.
A source-only hash is not a complete cache key. Reparse/requery after changes;
never attach facts by identifier spelling or by equal-looking expression text.

The adapter must verify the selected node kind and exact expression boundaries,
including any explicitly modeled wrapper/trivia differences. Offsets must use the
pinned API's documented units; Oxc and this tsrs revision use UTF8 byte offsets.
Repeated text, incompatible snapshots and ambiguous matches produce no hint.
Source-node identity is not lexical binding identity: map declarations and
value/type namespaces separately before consuming symbol-based information.

Initially, a file with syntactic or semantic diagnostics supplies no optimization
hints. Diagnostic queries may still display its recovered types. Missing hints
must not silently change the maintained compiler's admission or diagnostic order.
A separate user-requested type-checking command may enforce its own policy; adding
an optional optimization query must not accidentally introduce that policy.
Unsupported types or failed mappings mean "no hint", never a guessed Number.

Use checker APIs to classify candidate types. Human-readable type strings are
acceptable for this spike's assertions, not a machine-readable backend contract.
Checker-owned nodes/type IDs must not outlive their program/arena. Normalize or
copy any information that genuinely needs to escape that lifetime.

### Facts, guards and effects

Established facts come from reviewed transfer rules over actual values, enforced
entry/import contracts, or guards on the exact evaluated snapshots being used.
The existing two-Number ABI is an example; an annotation on a parameter is not.

A guard executes at the original evaluation point without invoking extra getters,
coercions or callbacks. Evaluate effectful operands once, preserve their order,
and use those same values on both specialized and fallback paths. Guard failure
continues through semantically equivalent admitted behavior, not a new type error.
If no faithful fallback exists for a kind, retain explicit source rejection until
that kind is implemented; a hint does not authorize widening admission.

Facts about a scalar snapshot differ from facts about a mutable cell or object
property. Writes, unknown calls, reentry and suspension invalidate affected
storage facts unless an effect/alias analysis proves otherwise. An unrelated
write need not invalidate an immutable scalar snapshot. `const`, `readonly`, or
a structural interface alone establishes neither immutable object contents nor a
fixed physical layout. A join retains only facts valid on every incoming path.

A Number fact includes NaN, infinities, negative zero and fractional values. It
is not an i32 range/integrality proof. Keep numerical identities, coercion order,
throwing behavior and resource/lifetime constraints separate from type hints.

## Scope and non-goals

This covers the trust and identity boundary feeding semantic analysis. It does
not replace structured IR, select Oxc versus tsrs as the sole frontend, introduce
unchecked TS semantics, implement guards/deoptimization, change a Wasm ABI, or
choose a heap/collector. It preserves current source limits and supported profiles.
The accompanying tests exercise the isolated query probe and native JS only;
object examples are not newly supported Grunkalunka source programs.

## Alternatives considered

- Option A: Trust checker types after a clean check.
  - Advantage: Immediate specialization with little adapter logic.
  - Disadvantage: Unsound typing and mutable aliases can contradict runtime kinds.
  - Disposition: Rejected for semantics-preserving compilation.
- Option B: Ignore checker information permanently.
  - Advantage: No extra frontend, identity mapping or query cost.
  - Disadvantage: May discard useful guidance for specialization or diagnostics.
  - Disposition: Retained as the measurement baseline and a possible outcome.
- Option C: Hints plus independent facts or guards.
  - Advantage: Investigates static guidance without redefining JavaScript behavior.
  - Disadvantage: Additional analysis, mapping and possibly guarded code size.
  - Disposition: Proposed, subject to a bounded cost/benefit experiment.
- Option D: Define a stricter compiled TypeScript dialect.
  - Advantage: Stronger language contracts could justify different representations.
  - Disadvantage: Changes the JS-compatible product contract.
  - Disposition: Outside this decision, not an implicit optimization shortcut.

## Consequences

This permits profitable specialization but guarantees no speedup. Type hints can
prioritize analysis or choose which guarded alternative to generate; they cannot
substitute for the checks that justify it. A lightweight analysis of equivalent
JavaScript may already establish everything needed. Additional parsing/checking
cost, memory, guard cost and code growth must be included in that comparison.

The existing spike remains outside the maintained workspace. No new production
dependency is approved. Its printed types and permissive fixture selector must
not be promoted unchanged as a verified metadata interface.

## Verification and evidence

**Inspected:** the pinned spike, its documented playback, production annotation
handling, backend-layout decision, and CI scope. Spike 004 remains an independent Cargo workspace; since PR #159 its focused
formatting and checker tests run through `node scripts/check.mjs` and participate
in dependency-selected PR verification rather than a standalone Actions job.

**Executable witnesses:** `tests/checker-hints.rs` in spike 004 retains the original
playback and tests Unicode/CRLF byte positions, duplicate-text ambiguity, an
assertion mismatch, a mutable-alias mismatch without `any` or assertions, and a
snapshot taken before the same mutation. The runtime oracle is independently
written JS executed by Node. These checks do not implement an identity validator
or validate a proposed guard emitter; they characterize why that boundary matters.

The three runtime witnesses were also checked locally with Microsoft TypeScript
5.8.3 and Node 22.16.0: each query reports `number` with no file syntax/semantic
diagnostics, while results are respectively string `402`, string `402` and Number
`42`. That local check used no library loading and is not a full-project type-check
or evidence about pinned tsrs. The PR's dedicated Rust/Node run must establish the
latter. No Rust execution, Wasm optimization or performance measurement is claimed
from document inspection; record actual CI outcomes in the PR.

Before accepting a production consumer, demonstrate one real query through a
validated source association to one specialization decision and executable Wasm.
Compare hint-disabled, hint-guided and deliberately misleading-hint variants of
the same supported program. Retain behavior in native JS, Node-Wasm and Wasmtime;
verify effects and fallback execution independently of generated-code inspection.
A path already specialized by the Number ABI is a control, not the claimed win.
Measure query/compile time, memory, emitted size and execution separately. A finding
of no useful advantage is a valid reason to defer integration.

## Explicit boundary statement

Checker results suggest what to investigate; established facts authorize what to
omit. This ADR does not make TypeScript sound or implement a production bridge.
Frontend selection, general source/binding association, and the first measured
specialization remain decisions requiring executable evidence.

## Related work and follow-up

Related: [0011](0011-semantic-values-and-lifetime-boundaries.md),
[0014](0014-backend-value-layout.md), and roadmap milestone T1. Start with the
existing spike rather than replacing either frontend. Resolve identity on one
expression family, then add one consumer; do not build an unused metadata layer.

Primary references, consulted 2026-10-26:
[TypeScript compatibility and soundness](https://www.typescriptlang.org/docs/handbook/type-compatibility.html),
[control-flow narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).
Pinned query APIs and their limitations are recorded in spike 004's source and
README; these references do not imply upstream endorsement of this architecture.

## Summary

Keep type hints, runtime facts and physical layouts separate. Prove the value of
one guarded or independently justified specialization before adopting a checker
as part of the production compiler pipeline.
