# ADR 0012: Keep language semantics in the guest and authority at host boundaries

Date: 2026-10-26

Status: proposed

## Context

This proposal assigns runtime responsibilities; it does not define every host
API. The inspection baseline is `087700f0ecc791e0c7e7dc58a52ca9819a10398d`.
Reconcile newer work before accepting or implementing it.

The [roadmap](../roadmap.md) calls for portable Wasm artifacts, explicit
capabilities and browser/Node/desktop environments built on the platform.
[ADR 0008](0008-strict-core-wasm-composition.md) already composes a bounded JSON
runtime with generated code. The [capability experiment](../capabilities.md)
separates resource-handle validation from authorization, but does not connect the
source compiler to a public component/grant API.

As builtins and the first useful application grow, convenience host imports could
accidentally make a JavaScript engine or a particular embedding the real language
runtime. Conversely, forcing each internal operation through a component boundary
would conflate language implementation with authority and packaging.

## Decision

Use a guest-owned language implementation with explicit environment adapters.
Generated Wasm and its linked runtime helpers implement admitted language
operations whose behavior does not inherently require host services. Keep
pristine-intrinsic restrictions explicit until broader semantics are implemented.

Do not implement ordinary arithmetic, coercion, property behavior or JSON parsing
by silently delegating to ambient host JavaScript. A host-based implementation may
remain an oracle or an explicitly selected experimental comparator, not a hidden
fallback for a production profile. Proven scalar operations still lower directly;
there is no requirement to route them through runtime calls.

The embedding supplies external authority and environment integration: approved
I/O, clocks/entropy where required, UI/platform services, execution budgets and
job scheduling. Each linked interface declares ownership, errors, reentry and
authorization behavior. Target Wasm features and host capability grants are
separate configuration dimensions.

Use WIT/Component Model contracts at application/component service boundaries,
as the roadmap intends. Internal compiler/runtime calls within one trusted guest
may use a private core-Wasm ABI. WIT types describe interfaces, not a substitute
for operation semantics or host policy. Define the boundary first, rather than
assuming every function, closure or heap object is a component resource.

For the first integration, default to explicit imports and deny unrequested
authority. Check grant owner/lifetime and policy before protected work; canonical
handle validity alone does not authorize it. Revocation behavior must be defined
at use, not assumed from a valid handle or successful instantiation.

Cross-boundary values use explicit copies or validated resource ownership. A
coincident guest integer is not a host identity. No callback may reenter while a
runtime borrow, lock or root update is in an unsafe intermediate state. Preserve
the current synchronous guest's no-reentry rule until an extension proves a
specific alternative. Do not promise recovery from an interrupted Rust guest by
calling cleanup code whose invariants may already be broken.

## Scope and non-goals

This covers responsibility assignment and a bounded guest/host integration test.
It preserves current compiler profiles, ABIs, budgets and the strict composition
profile. It does not extend that linker to arbitrary modules or claim isolation
between generated code and runtime helpers sharing memory.

No browser, DOM, Node module system, filesystem API, full permission taxonomy or
public plugin ABI is selected here. Existing host-scheduled async adapters remain
valid. Moving their Promise compatibility machinery into the guest is not a
prerequisite; the long-term ownership boundary needs separate evidence.

## Alternatives considered

- Option A: Implement builtins through ambient host JS objects.
  - Advantages: Fast access to mature implementations in a JS embedding.
  - Disadvantages: Couples language behavior to the host and obscures authority.
  - Disposition: Retain for oracles or explicit experiments, not the default.
- Option B: Put every internal operation behind a WIT interface.
  - Advantages: Uniform interface descriptions and component tooling.
  - Disadvantages: Unnecessary boundaries and ownership translation; cost unproven.
  - Disposition: Rejected as a blanket rule; retain WIT at actual service boundaries.
- Option C: Guest language helpers plus explicit capability adapters.
  - Advantages: Portable semantic implementation and separately testable authority.
  - Disadvantages: Own the runtime semantics and maintain host adapters.
  - Disposition: Proposed; reuse current composition and capability evidence.

## Consequences

The same supported language operation should not change because the host provides
a different JS object or library. Required host services stay visible, and native
implementations can vary behind tested contracts. This does not establish engine
portability, security or small binaries without measurements.

The guest runtime and linking costs may increase. Keep helper inclusion deliberate
and measure footprint rather than requiring an elaborate linker optimization now.
Future service calls need behavioral contracts beyond their WIT signatures.
A native acceleration path would require an explicit contract, matching fallback
and differential evidence; it must not quietly become required host semantics.

## Verification and evidence

**Observed:** the linked roadmap, composition ADR and capability documentation
were inspected at the baseline. Their reported tests are existing evidence, not
new results from preparing this proposal. No combined source/component app was run.

**Required before acceptance:** exercise one existing source-lowered guest operation
through identical emitted bytes in Node-Wasm and Wasmtime, with no host JS semantic
implementation. Record its imports and repeat under separate instances.

Separately connect the smallest usable compiled computation to one explicit
capability using an adapter. Exercise absent, denied and granted authority; verify
that denied/revoked calls perform no protected work and foreign-owner tokens do
not alias. Retain the existing handwritten component tests as independent checks,
not substitutes for compiler integration. Do not require a complete component
compiler to run this adapter experiment.

Check ownership/drop, output-copy boundaries, reentry rejection and terminal traps.
Distinguish language errors from host protocol/resource failures. Run the relevant
composition, capability and ownership gates plus the canonical check when available.
Measure guest/runtime bytes and host-call cost separately from language execution.

Revise the boundary if the selected workload requires host internals or if the
adapter recreates the language runtime on each host. A larger boundary is acceptable
only with a documented semantic and cost rationale, not to bypass a failing test.

## Explicit boundary statement

This introduces an ownership rule for language behavior and external authority.
It does not make internal guest memory a security boundary or define complete
browser/Node services. Public host contracts, component packaging and callback
policies remain decisions to validate as those services become real.

## Related work and follow-up

Related: [ADR 0008](0008-strict-core-wasm-composition.md),
[capabilities](../capabilities.md), [async](../async.md), and proposed
[0011](0011-semantic-values-and-lifetime-boundaries.md).
Roadmap: R1, P1 and A1; later browser/Node environment work.

[Proposed ADR 0018](0018-component-composed-environments.md) expands the *candidate*
environment composition, provider/shim, cross-language WIT binding,
placement/isolation and interop-cost questions. It does not change ADR 0012's
guest-owned language-semantics boundary or accept a public host API.

The [WIT reference](https://component-model.bytecodealliance.org/design/wit.html)
describes interfaces and owned/borrowed resources; it does not supply this
project's authorization policy. Identify the first application service before
standardizing a larger host API. Coordinate scheduling with proposed
[0013](0013-async-job-and-activation-ownership.md).

## Summary

Own admitted language semantics in portable guest code; expose external services
through explicit, tested host contracts. Use component boundaries where they
represent real services, not as a compulsory wrapper around internal operations.
