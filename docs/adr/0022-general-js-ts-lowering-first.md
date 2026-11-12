# ADR 0022: General JS/TS lowering before exhaustive compliance or optimization

Date: 2026-11-12

Status: accepted

Maintainer decision: prioritize broad, workable JS/TS-to-Wasm lowering. This is a
target and implementation strategy, not a claim about the current compiler's
capabilities. The inspected baseline is `master`
[`6b60627`](https://github.com/tseguine/grunkalunka/commit/6b6062727ed767eeb5816107aaff3a4833f39a8e).

## Context

The maintained Oxc frontend, owned source-lowering cursor, verified typed Region
IR, Wasm emitter and WasmGC-backed runtime execute substantial but bounded
numeric, primitive, managed-value and async profiles. These have useful
specialized paths and regression coverage. The source-lowering migration
documented by [ADR 0020](0020-source-lowering-control-kernel.md) has removed
the duplicate effectful-source drivers; it is not the obstacle this decision
addresses.

Much of current source admission depends on proving a particular value kind,
receiver, function signature, source shape or purity property in advance.
When that proof is unavailable, ordinary valid JS/TS often fails compilation,
even where a more general runtime operation could execute it. Incremental
source-specific admissions have been productive, including the
[November 3 feature train](../javascript-feature-train.md), but cannot be the
primary strategy for approaching a general-purpose JS/TS compiler.

The desired product is a Wasm-first JS/TS application platform, not an
ever-growing language subset chosen by ease of static proof. The maintainer
therefore separates three goals:

1. **Lowering coverage:** accept essentially every valid JS/TS program in the
   selected language configuration and produce a valid, executable Wasm artifact
   with workable general runtime behavior.
2. **Semantic compliance:** progressively improve observable behavior toward
   ECMAScript and applicable TypeScript erasure/module semantics.
3. **Optimization:** progressively reduce runtime work, code size, memory use
   and overhead where equivalent behavior can be established.

These goals are related but **not sequential correctness guarantees**:
reasonable correctness and tests remain necessary throughout. The priority is
to remove *compiler rejection of valid source* before pursuing exhaustive
conformance or extensive new specializations.

## Decision

### 1. Broad lowering coverage is the primary compiler milestone

Build a general lowering path for valid JavaScript and TypeScript that does not
depend on source-specific positive proofs to decide whether ordinary operations
can be compiled. Where a type, property name, callee, receiver or control-flow
outcome is not statically known, emit the relevant general semantic operation
instead of rejecting the program merely because specialization is unavailable.

Aim for **near-total valid-source lowering coverage**, not a newly enumerated
collection of bounded language profiles. The objective is that a valid program
normally compiles and has a meaningful path to execution even when its precise
runtime kinds and behavior cannot be established ahead of time.

This is a direction for the general compiler entry point and runtime: the
present narrow experimental entry points and existing numeric/primitive ABIs
may coexist during migration. The eventual public entry, module and value
interfaces are not fixed by this ADR.

### 2. Workable execution, not acceptance theater

A program counts as successfully lowered only if the generated Wasm is valid,
can be instantiated in its declared environment, and implements a meaningful
execution path for its operations. Replacing unsupported syntax with an
unconditional trap, no-op, fabricated result or silent host-JavaScript
delegation does **not** constitute working lowering.

An initially incomplete general operation can differ from a fully compliant
ECMAScript implementation, provided the boundary is explicit, does not
silently invent authority, and gives useful behavior for a defined domain.
Record semantic limitations and failures separately from front-end/lowering
rejections. Do not intentionally miscompile a known supported case merely
to raise an admission count. Syntax errors, explicit resource limits and
unavailable/denied host capabilities remain legitimate distinct failures.

### 3. General semantic primitives precede new proof-dependent admissions

Extend the existing compiler/IR and guest runtime as needed to support general
operations and their composition, including:

- Values and coercion across JS runtime kinds, with an extensible representation
  for currently missing kinds such as Symbol and BigInt.
- Property Get/Set/Delete/Has, computed keys, receiver behavior and
  runtime-dependent error outcomes.
- First-class callable values, ordinary dynamic invocation, argument lists,
  receiver/`this` handling, construction and closures.
- Lexical environments, binding initialization and mutation, function and
  module boundaries, and control-flow completions.
- Iterators/generators, async functions, Promise values and suspension/job
  behavior.
- Source exceptions propagated through calls and finalizers, distinguishable
  from malformed IR, Wasm traps and host/capability failures.

These are **semantic families, not separate mandates to implement every
ECMAScript edge case before proceeding**. Implement broadly reusable,
composable mechanisms in reviewable increments. One general operation that
eliminates a category of static admission failures is preferred to many
isolated special-case recognizers, other things equal.

A general lowering may emit direct Wasm, private guest helper calls or a
combination. It need not be a bytecode interpreter. If genuinely dynamic
source creation (for example `eval` or the `Function` constructor) needs a
runtime compiler/interpreter, treat that as a bounded execution mechanism
with explicit capabilities and budgets rather than as a reason to reject
unrelated static source.

### 4. Specialization is optional and proves equivalence

Retain working specialized lowering, including unboxed Number/f64 paths,
known receivers, constant keys, typed helper calls and source-proven closure
operations. They remain valuable implementations of cases covered by the
general semantic model.

For future features, an inability to prove a specialization must normally
select the general path, **not a compiler error**. Proofs, TS annotations,
static origin analysis and speculative inference may enable optimization but
must not silently redefine JavaScript semantics. Whenever both paths are
available, compare their observable results and effects against each other
and, where practical, against native JavaScript. A general path is not
automatically spec-correct and is not a substitute for independent oracles.

The decision does not mandate two independently maintained implementations
of every operation or require an immediate new optimization pass. An existing
fast path can remain the direct implementation while general coverage is
added around it. Avoid duplicating complete language semantics merely to
separate the paths conceptually.

### 5. Maintain clear language and environment boundaries

TypeScript type-only syntax is erased as appropriate; no whole-program type
checker or reliance on claimed TS types is required for runtime correctness.
Define supported JS/TS syntax versions and module configurations explicitly.
A valid source file may still need resolvable imports, linked services and
capability grants at instantiation or execution; these are not evidence that
the language lowerer should have rejected its syntax.

Guest code owns language semantics. Do not obtain apparent coverage through
ambient Node/browser JavaScript, implicit filesystem/network privileges or
undeclared host calls. Retain WIT/Component Model at real service boundaries;
ordinary dynamic language operations may remain private within generated
Wasm and its guest runtime. Browser and Node environment completeness are
separate goals.

Preserve source evaluation order, observable side effects and identity to
the extent already established. When behavior is incomplete, characterize
the divergence and prioritize reusable fixes. Retain memory/fuel/source
budgets, controlled diagnostics, native-stack hygiene, WasmGC rooting,
capability isolation and the existing test/CI obligations.

### 6. Change the compiler work priority

This accepted maintainer direction **supersedes the November 3
impact-ordered feature train as the default priority for new compiler units**.
That train remains valuable historical implementation evidence and may supply
fixtures or useful bounded pieces. Its numbered feature-family order is no
longer the governing strategy. A1 use cases are valuable integration tests,
but do not determine which valid JS constructs merit compiler admission.

Prefer work that enlarges the reusable general-lowering surface, removes
entire causes of rejection or establishes necessary runtime semantics.
Existing coherent in-flight work may be finished and integrated without
being retroactively declared invalid. Keep dependent implementation PRs
in their maintainers' required stack structure; this architecture-document
PR is independent.

This ADR changes the compiler/runtime strategic objective. It does not
automatically assign tasks to other workstreams, authorize cross-stream
interface changes, or bypass the maintainer's coordination rules in
[AGENTS.md](../../AGENTS.md).

## Scope and non-goals

This ADR chooses **coverage-first architecture and priority**, not a final
runtime design or a declaration that arbitrary programs work today.

It does not require full Test262 compliance, perfect Proxy/accessor/iterator
edge semantics, complete browser or Node APIs, instant support for every
third-party dependency, production-level performance, complete security
isolation, or a public generic host ABI before the lowering-coverage goal can
advance. Those obligations remain visible but independently planned.

It does not require replacing Oxc, the owned source cursor, Region IR,
verifier, WasmGC, existing numeric fast paths or the accepted backend layout
policy. A second CFG, universal dynamic-call convention, generic object
model, runtime source compiler or particular GC representation needs a
concrete implementation justification and its own bounded decision where
appropriate.

The aspiration of "essentially every valid program" is a product goal, not
permission to claim 100% coverage from a small test corpus or to ignore
legitimate syntactic and environmental constraints.

## Alternatives considered

- **Continue static-proof-first feature admission.** Preserves narrow,
  well-tested execution contracts and immediate specialization, but adds
  increasingly complex inference merely to avoid general runtime behavior.
  Rejected as the long-term primary strategy; retained for fast paths.
- **Require full ECMAScript compliance before general lowering.** Gives a
  clear semantic standard but couples broad compilation to the long tail of
  specification edge cases. Rejected as a milestone dependency; independent
  compliance work remains essential.
- **Build an all-purpose interpreter first.** Can offer broad initial syntax
  coverage but imposes a second execution architecture and may lose direct
  compilation benefits. Not required; bounded interpreter/compiler mechanisms
  may be appropriate for truly dynamic source.
- **Rewrite the typed IR/compiler now.** Might simplify a future general
  implementation but would discard validated infrastructure without an
  established necessity. Rejected as a blanket prerequisite.
- **Delegate unsupported semantics to host JS.** Superficially broadens
  coverage but breaks portable guest ownership and makes behavior depend on
  the embedding host. Rejected as a default; host JS remains an oracle.

## Consequences

General operations will require more guest runtime machinery, runtime checks
and possibly larger/slower unoptimized artifacts. They make ordinary programs
compile without increasingly delicate whole-source facts and provide a common
semantic target for future optimizations.

The current tight primitive/result ABIs and exact typed closure signatures
will need a general execution interface alongside them. This ADR deliberately
does not select its representation. Ordinary source-language failures should
eventually use guest-visible abrupt completions; internal invariant violations
and host traps must stay distinct.

The compiler may temporarily accept a broad program while exposing documented
behavioral divergences. Those divergences are engineering debt, not evidence
of standard compliance. New implementations must not regress already verified
semantics or suppress failing tests to improve the reported coverage.

## Verification and evidence

**Decision evidence (2026-11-12):** the existing compiler, runtime, ADRs,
[feature train](../javascript-feature-train.md), [engine steering](../javascript-engine-steering.md)
and [known lowering gaps](../lowering-gaps.md) show successful bounded source
execution alongside positive-proof-driven admission. This ADR is documentation
and a maintainer-selected direction; no new general implementation or test is
claimed by recording it. The selected baseline was checked at
[`6b60627`](https://github.com/tseguine/grunkalunka/commit/6b6062727ed767eeb5816107aaff3a4833f39a8e).

Measure progress in **separate categories**:

1. **Validity and lowering:** parse/configuration errors; valid sources that
   emit and validate executable Wasm; valid sources rejected during lowering;
   valid sources that compile but reach an explicitly unimplemented runtime
   operation. Do not conflate an expected compiler rejection with success.
2. **Workability:** execute representative complete programs across values,
   calls, properties, closures, exceptions, iteration, modules and async.
   Check observable effects, identities, state, exception and job ordering,
   not just numeric return values. Include JS/TS source pairs where relevant.
3. **Compliance:** use native JS and selected original-harness Test262 cases
   to record matches and divergences independently from compilation coverage.
   An adapted fixture is not full-suite conformance evidence.
4. **Optimization:** compare generic/specialized execution on the same sources
   and inputs, including side effects and errors. Measure runtime, footprint
   and compilation costs independently of semantic coverage.

Expand the compatibility/rejection inventory into a source-coverage census
with attributable reasons. For admission progress, prefer previously rejected
*unchanged complete programs*, not a growing count of modified micro-fixtures.
Exercise common syntax families and compositions, then sample broader external
corpora; explicitly list exclusions. A program that only instantiates and
immediately traps is not a successful executable witness.

**Milestone exit direction:** the general compiler should reject valid,
configured JS/TS source only exceptionally; remaining exceptions must be
enumerated rather than embedded in an implicit static subset. Broad,
representative programs must actually execute through the general path.
Conformance deficits and performance deficits are tracked separately, without
blocking broad lowering merely for lack of exhaustive edge-case parity.

Maintain Wasmtime/Node-Wasm verification, true native-JS oracles where meaningful,
bounded small-stack probes, IR/Wasm verification, capability-denial tests and
exact-head CI requirements. No such new runs were performed for this ADR.

## Explicit boundary statement

This decision **introduces an accepted lowering-coverage-first engineering
objective and a general-runtime fallback expectation**. It does **not**
implement full JavaScript, choose a permanent universal ABI or declare
conformance, speed or current acceptance percentages. Future bounded decisions
must settle concrete dynamic value/call/completion representation and host
lifetime issues as implementation evidence accumulates.

## Related work and follow-up

- [ADR 0001](0001-explicit-evaluator-control.md),
  [ADR 0014](0014-backend-value-layout.md),
  [ADR 0019](0019-wasmgc-runtime.md) and
  [ADR 0020](0020-source-lowering-control-kernel.md) supply the retained
  evaluator, representation, rooting and owned-source architecture.
- [ADR 0021](0021-synchronous-finally-completions.md) records the existing
  partial completion path. General cross-call throws and dynamic operation
  errors remain needed.
- [ADRs 0012](0012-guest-runtime-and-host-capabilities.md) and
  [0018](0018-component-composed-environments.md) address *proposed*
  guest/host and environment composition; this decision does not accept
  their unselected APIs.
- The historical [feature train](../javascript-feature-train.md),
  [milestones](../milestones.md) and
  [engine steering](../javascript-engine-steering.md) remain evidence for
  what already works, not a new restriction on the general target.

## Summary

**Compile broadly first; improve compliance and optimize afterward.** The
normal path for valid JS/TS should become a workable general lowering to
Wasm rather than rejection for lack of a source proof. Existing efficient
specializations survive and later generalize as semantics-preserving
optimizations. Broad lowering coverage, standards compliance and performance
are distinct measurable outcomes.
