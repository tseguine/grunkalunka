# ADR 0013: Separate async activation ownership from host job scheduling

Date: 2026-10-26

Status: proposed

## Context

This proposal defines the boundary between an async activation and its scheduler;
it does not implement general Promises or a browser event loop. The inspection
baseline is `087700f0ecc791e0c7e7dc58a52ca9819a10398d`. Reconcile newer work before
accepting or implementing it.

The [primitive async profile](../async.md) already emits resumable Wasm with
per-invocation saved state. Its JS adapter uses real Promises and microtasks;
the Rust adapter exposes pending/fulfilled/rejected state and one-job stepping.
The profile documents primitive await reactions, synchronous source throws,
lexical catch handling, shared captures and terminal resource/protocol failures.
These are bounded capabilities, not general Promise interoperability.

As the semantic IR evolves, compiler control flow must not depend on the layout
of a host queue, and scheduling must not assume that one Wasm instance per call
is the permanent storage model. Independent invocations also need a common
ordering contract: separately draining each invocation can preserve individual
results while changing their observable interleaving.

## Decision

Keep three responsibilities explicit: semantic suspension/completion, activation
storage, and host scheduling. Reuse the current state machine and adapters where
they satisfy this boundary; do not introduce a new coroutine implementation merely
to adopt these names.

### Semantic suspension and completion

The semantic layer records suspension points, resume values, normal/abrupt
completion and applicable handlers. It preserves evaluated snapshots and captured
cell identity. Physical spill slots, state numbers, Wasm globals and dispatch
instructions belong to lowering, not source semantics.

Distinguish a synchronous source throw from a rejected await reaction and both
from an engine trap or malformed resume protocol. A throw alone must not invent an
await job. Preserve the currently admitted reaction boundaries, including settled
primitive operands, and do not resume inline merely because a value is ready.

This is not a rule that every possible future await requires exactly one host
callback. General Promise adoption and thenable behavior need their own semantic
implementation and ordering evidence before admission.

### Activation and continuation ownership

An activation owns its saved state until completion or explicit host teardown.
A scheduled continuation has an owner identity and a particular pending resume
event. Delivering that event is at most once; executing it may produce a new event.
Reject duplicate, stale, foreign-owner and reentrant delivery before entering
guest code or changing its saved state. Choose an existing object identity or a
checked token scheme; this ADR does not require a globally allocated integer ID.

The queue retains the state needed for scheduled work. Terminal activations cannot
be revived by late jobs. Teardown invalidates queued deliveries and releases
host-owned references without invoking arbitrary guest cleanup after a trap.
Keep teardown distinct from a new JavaScript cancellation or rejection semantic;
a public cancellation API and its observable settlement behavior remain separate.

Start with the current one-instance-per-invocation storage. Any later shared
instance/frame-pool design must preserve invocation isolation and shared captured
cells deliberately. A value shared by closures is not copied merely because the
activation suspends. Managed heap values need activation-lifetime protection;
ADR 0010's synchronous LIFO root frames do not establish that protection alone.

### Scheduler and embedding contract

For one supported single-agent execution domain, coordinate admitted Promise-style
jobs through one logical enqueue order. It need not be one physical container,
but ready jobs must not be drained per activation in a way that changes that
order. Enqueue requests schedule later execution; they do not recursively enter
an unfinished start/resume operation.

In a JS embedding, use the host's actual microtask facility at the semantic enqueue
point. Do not batch a private queue behind one callback when doing so overtakes
other host microtasks. In a native embedding, provide explicit queue ownership
and a deterministic one-job test pump; the environment defines when checkpoints
run. Step/run-to-quiescence harness operations are not themselves a complete
browser or Node event-loop policy.

Keep native tasks, timers, I/O completion and Promise-style jobs distinct. A future
browser environment must implement its own required checkpoint behavior rather
than treating this bounded FIFO experiment as HTML event-loop conformance.

Existing per-entry fuel, await-site/job limits and graph budgets remain in force.
Track any new queue/activation resource limit separately. Budget exhaustion is an
explicit host failure under the current contract, not a catchable source throw or
a successful fairness yield. Do not silently interleave unrelated tasks to make a
long microtask sequence appear to meet its budget.

## Scope and non-goals

### Guest kernel implementation evidence

The backend now contains an invocation-local primitive Promise settlement and
reaction kernel, with executable Wasmtime and Node tests. Pending waiters become
ready FIFO records on settlement; late subscribers enqueue their own reaction.
An issued-count read reports each newly enqueued reaction separately from ready
queue depth. Native Promise traces with external microtask markers and reactions
that register more reactions verify that distinction. The kernel never invokes
host callbacks or drains itself inline.

Pending IR and adapter integration now consume this kernel. Logical Promise
identity is separate from General body completion. An eager start returns its
handle at first suspension or settlement; queued resumptions restore the specific
activation's frame, including sibling calls to the same compiled body. A separate
initial-return flag allows frame address zero and prevents a later continuation
from returning synchronously through a stale parent link.

The optional `take_jobs` export reports newly issued jobs. Node registers one
host callback for every issue before exposing root settlement at that boundary.
Wasmtime reports outstanding callbacks separately through `pending_job_count`;
an embedding subtracts the callbacks it has already registered before appending
new ones. Root settlement does not end an invocation while owned work remains.
Each resume consumes one guest job. Existing direct-await source paths and modules
without identity operations retain the previous protocol and output. Separate
source requests now create pending-capable identities, preserve them through
bindings/cells/getters and register later awaits. Discarding a call releases its
source reference but does not cancel the invocation's owned ready work.
The handled bit records subscription. First rejection without a handler emits an
ordered tracking event; the first subsequent subscription emits a handled event.
These are distinct from guest continuation jobs. `take_promise_event` consumes
events at each boundary before exposing root settlement. Node creates rejected
Promise proxies and attaches handlers as notified, so native host checkpoints
decide which rejections remain unhandled. Wasmtime exposes the ordered
`PromiseNotification` stream for an embedding's checkpoint policy. The primitive
reason and invocation-local ID survive root settlement; no guest execution state
is exposed through a Node proxy. Tracking uses bounded arena records and does not
change reaction-job accounting. Pending async returns now use `AdoptPromise`:
a queued registration subscribes to the returned identity, then a separate
reaction forwards its primitive settlement. Returning a Promise is therefore
distinct from awaiting it. Adopted rejection settles the owning physical
activation outside its lexical catches; errors during return-expression
evaluation still follow lexical handling. Source inference marks adopting
constructors as asynchronous even when they contain no explicit await.
The verifier forbids adoption in an inlined `AwaitCall` child without its own
activation, and in immediate snapshot settlement bodies. Frame restoration at
registration preserves handles across shared callee invocations. Implicit jobs
retain the runtime budget without charging explicit await-site provenance.
Snapshot adoption, arbitrary thenables and a browser event loop remain outside
this slice.

This covers ownership, delivery and ordering for the admitted primitive async
profile. Preserve its current external status/tag/payload ABI, diagnostics, source
limits and documented terminal-error behavior unless a separately justified change
updates compiler and adapters together.

It does not complete arbitrary Promise/thenable values, general synchronous exceptions, generators,
workers, cross-agent sharing, host I/O, JS cancellation,
a new GC, or a general reentrant host-call model.

The PoC sequence admitted stored/discarded pending calls before host event delivery.
That unit's detached-child counterexample is now a maintained native/Node-Wasm
regression: root fulfillment and the child's unhandled reason agree. Separate
tests cover same-checkpoint late handling, repeated awaits, primitive reason bits,
cross-instance event order and Wasmtime notification consumption. Publishing only
after an instance drains would reorder an early rejection behind a later rejection
from an instance with less unrelated work; boundary delivery avoids that error.
Other unsupported source remains explicit; this does not claim full Promise
compatibility or universal host checkpoint conformance.

## Alternatives considered

- Option A: Each activation privately runs all of its pending jobs.
  - Advantages: Simple isolated tests and local ownership.
  - Disadvantages: Can change ordering between activations and embedding jobs.
  - Disposition: Retain isolated stepping for tests, not the shared scheduling rule.
- Option B: Resume immediately whenever the operand is already available.
  - Advantages: Removes queue overhead for common inputs.
  - Disadvantages: Changes observable boundaries and enables accidental reentry.
  - Disposition: Rejected without proof of unobservable equivalence for a specific
    optimization; not a default lowering strategy.
- Option C: Own a complete Promise implementation and event loop immediately.
  - Advantages: One place for runtime scheduling and future interoperability.
  - Disadvantages: Expands this decision into object semantics and environment APIs.
  - Disposition: Deferred; retain current explicitly bounded adapters.
- Option D: Activation-owned state with an explicit embedding scheduler contract.
  - Advantages: Separates compiler layout, lifetime and observable job ordering.
  - Disadvantages: Requires delivery validation and multi-activation trace tests.
  - Disposition: Proposed; prefer evolving existing adapters over replacing them.

## Consequences

The compiler can change activation layout without simultaneously redesigning the
host queue. Scheduling, terminal states and ownership become independently
reviewable. This may add queue metadata and lifetime bookkeeping; reduced instance
count or faster execution is not promised.

A JS-hosted adapter remains an explicit compatibility implementation, not a hidden
generic language fallback. A future guest Promise implementation must preserve
this boundary and its cross-host tests, but is not a prerequisite here. Reusing a
native async executor does not by itself establish the required JS job ordering.

## Verification and evidence

**Observed:** the baseline async, catch and throw documentation describes existing
source-to-Wasm tests and host adapters. No test was rerun for this draft. The
external references below specify language/host ordering, not evidence that this
proposal has been implemented or validated.

**Required before acceptance:** run the existing async regression suite unchanged,
then add a shared scheduler witness with at least two interleaved activations.
Use identical modules in Node-Wasm and Wasmtime, and original native JS as the
language reference. Compare ordered markers, values and rejection reasons, not
only final results. Include synchronous-prefix effects, settled awaits, skipped
await branches, repeated loop awaits, caught/uncaught rejection, source throw,
saved operands and mutation of captured bindings.

The JS witness must interleave unrelated host microtasks with generated jobs so
that a batched private drain fails. The native witness must expose enqueue and
one-job steps deterministically, with no sleeps as ordering evidence. Keep any
additional tracing at the adapter/test boundary unless source admission explicitly
includes the required operation.

Exercise stale, duplicate, wrong-owner and reentrant resume attempts, teardown
with queued work, terminal traps and independent surviving activations. Verify
existing limits remain observable and separately test any new queue bounds.
An injected inline-resume or per-activation-drain defect must fail a trace test.

Run focused async/runtime checks and `node scripts/check.mjs` when prerequisites
are available; report unrun gates. Measure cold start/instantiation, per-job cost,
saved-state size and peak retained activation count independently. Reference
traces establish only the selected profile, not complete Promise or event-loop
compatibility.

Revise this proposal if the boundary forces a full Promise/heap rewrite before
one shared-queue witness, cannot compose with native host microtasks, or hides
unrecoverable guest state behind a nominally reusable activation.

## Explicit boundary statement

This introduces explicit ownership and delivery rules between compiled async
state and an embedding scheduler. It does not select a complete Promise object
model or a browser event loop. Managed roots across suspension, host checkpoint
policies, cancellation and general Promise interoperability remain decisions to
validate separately.

## Related work and follow-up

The optional true-parallel JS/TS task idea belongs to
[proposed ADR 0018](0018-component-composed-environments.md#workers-asynchronous-tasks-and-optional-parallel-js).
An `async` function is not automatically parallelizable: a future explicit
parallel-isolate API must preserve this ADR's single-agent Promise/job order,
resource ownership and no-unsafe-reentry requirements. Threading and shared
heap semantics remain undecided.


Related: [async profile](../async.md), [catch](../async-catch.md),
[throw](../async-throw.md), [captured cells](../captured-cells.md),
[ADR 0010](0010-bounded-internal-root-frames.md), and proposed
[0011](0011-semantic-values-and-lifetime-boundaries.md) and
[0012](0012-guest-runtime-and-host-capabilities.md).
Roadmap: L5, P1 and the first asynchronous application slice. None of the three
proposals requires all others to be implemented before its bounded experiment.

Primary semantic references, inspected 2026-10-26:
- [ECMAScript Await](https://tc39.es/ecma262/multipage/control-abstraction-objects.html#await)
  defines suspension and Promise reaction continuation semantics.
- [HostEnqueuePromiseJob](https://tc39.es/ecma262/multipage/executable-code-and-execution-contexts.html#sec-hostenqueuepromisejob)
  defines future job execution and the required enqueue order.
- [HTML microtask checkpoints](https://html.spec.whatwg.org/multipage/webappapis.html#perform-a-microtask-checkpoint)
  are a later browser integration contract, not implemented by this proposal.

Record the actual specification revision used by implementation tests when the
proposal is accepted; these living reference links are not immutable suite pins.

## Summary

Let compiled activations own saved language state and let explicit host adapters
schedule continuations without changing observable ordering. Prove shared-queue
behavior and terminal ownership with the current primitive profile before adding
general Promises, managed suspension roots or a broader environment event loop.

## Intrinsic self-resolution follow-up

The concrete mutable-capture self-resolution counterexample now rejects the owning
Promise immediately with a fresh TypeError identity, before scheduling ordinary
adoption registration. This is a catchable guest reason, distinct from terminal
fuel, trap or protocol errors. Existing rejection jobs deliver it to outer catches;
unhandled reasons use the same tracking FIFO. Error records belong to the invocation
arena. Node decodes one native object per ID, retaining the same object across root
outcomes and notifications, and creates separate objects for different invocations.
Wasmtime exposes a typed invocation-local descriptor. The opted-in error-kind
export validates the record; primitive-only modules keep their old protocol.
This closes self-resolution, not all object conversion: Error relational string
conversion has a separate executable counterexample and remains required work.

## Async finally implementation follow-up (2026-10-28)

The admitted async root/private-async profiles now use structured TryFinally
regions. Typed frame snapshots preserve raw return values and General throw
reasons, while continuation identity carries the pending exit kind. Normal
cleanup resumes that completion; abrupt cleanup replaces it. Async return
adoption follows all active finalizers, preserving Promise identity, handling
and job order during cleanup.

The source finalizer is lowered once. The splitter currently emits one cleanup
path per pending normal/return/throw/break/continue route, with at most five
entries at one scope. Reusing the same IR await across those entries counts
once toward source admission; executed reactions still count individually.
This simple static routing avoids a new dynamic completion record or dispatch
protocol. Nested cleanup can expand the state graph and remains subject to the
existing 4096-state bound; compaction is a later optimization if needed.

Native JS/Node-Wasm/Wasmtime tests cover completion overrides, nested handlers,
loop targets, snapshots, concurrent activations and adoption timing. Notification
tests cover overridden rejected returns and later handling. Host fuel/trap/job
failures remain terminal and cannot be recovered by a source finalizer. No
dependency, host ABI, kernel record, source-await/job/fuel limit or universal
exception model is changed.

## Named module async helper reuse (2026-10-28)

Plain named async declarations in the async identity profile use the existing
private async signature and AsyncClosureEntry marker with an empty capture list.
MakeClosure selects the original semantic symbol's function ID, and the normal
private async call route supplies fresh Promise and activation ownership. This
keeps declaration hoisting, eager startup, argument snapshots, source catches,
return adoption and recursive/concurrent captured-cell lifetimes on one path.

The alternative of treating declarations as synchronous General-returning calls
would lose Promise identity and body/caller reaction boundaries. A separate named
coroutine implementation would duplicate the existing frame and queue protocol.
The selected reuse changes no IR operation/type, physical frame, host ABI, job or
fuel policy. Named helper parameters infer Number/General primitive inputs and
Promise/AsyncValue identity or mixed inputs from actual calls and forwarding.
This reuses private closure input packing and captured cells; Number-only
proofs/signatures stay intact. Function, Error and heap input domains remain
separate. Erased types do not choose kinds. Passing a Promise argument alone
does not handle or adopt it: body await and return retain their existing ownership.
Promise-valued throws and intrinsic Promise.reject(p) now use the raw rejection
path described below.
Maintained native/Node-Wasm/Wasmtime and rejection-checkpoint tests exercise this
bounded extension; see [the contract and limits](../async-named-helpers.md).

## Host Promise identity transport (2026-10-29)

Queued modules expose a read-only promise_state snapshot for canonical guest
records. A raw Promise reference is permitted as a rejected value, preserving
identity without adoption. Node uses one real native Promise per invocation-local
ID for exposed reasons and rejection tracking; Wasmtime uses a typed descriptor
and explicit read-only queries. Snapshot reads do not mutate guest handling or
schedule work. Settled values and notification history remain checked, and host
protocol failures remain terminal. Primitive-only protocols stay closed. This
avoids a plain-object stand-in with different Promise behavior or duplicate host
rejection proxies. Broader thenable/object domains remain required follow-ups; see
[the protocol](../async-promise-objects.md).

## Source Promise-valued throw (2026-10-29)

Async throw selects the existing AsyncValue carrier for represented Promise or
Promise/primitive operands. The initial source provenance fixed point used a
conservative module-wide catch domain, reusing alias/capture/private-call facts
and preserving primitive-only modules. A concrete unrelated-catch arithmetic
failure motivated the lexical refinement below, which replaces that module-wide
catch classification. Type annotations do not choose the domain.

Async verification admits raw root and child rejection while lexical primitive
handlers and ordinary verifiers retain their closed domains. Completion emission
copies both lanes. Internal reaction data has a separate validator because an
already-caught reason travels on a normal continuation edge; Promise settlement
continues to reject raw fulfillment payloads. This avoids a new completion opcode
or host ABI while preserving rejection identity, cleanup precedence and job order.
Maintained native/Node-Wasm/Wasmtime tests cover source catches and root objects.
Intrinsic Promise.reject(p) now uses the same carrier as described below.

## Intrinsic Promise-valued rejection (2026-10-29)

Promise.reject uses the existing raw Throw in a SettlePromise body and materializes
the resulting rejected wrapper. Its payload may be a represented Promise or mixed
AsyncValue; fulfillment construction remains General-only. The module exception
summary includes these producers so catches retain their original raw reasons.
Direct awaited intrinsics materialize and await the rejected wrapper rather than
awaiting the reason. This reuses the identity queue and handling protocol and avoids
a new raw-await opcode or accidental reason adoption. Surplus argument evaluation
finishes before construction, using the earlier first-argument snapshot.

Native JS, Node-Wasm and Wasmtime tests cover the former exact exclusions, nested
and self-reasons, pending objects, alias/private forwarding, mixed tags, cleanup,
argument failure, notifications, erased types and full reaction traces. The same
host ABI and source limits remain unchanged. Lexical catch inference is refined below.

## Lexical source reason refinement (2026-10-29)

Refine the former module-wide catch classification using the existing monotone
source provenance query. Route represented raw throws/await rejections to the
nearest lexical handler and intervening finally buffer, stopping at function
boundaries. This avoids rejecting unrelated primitive catch arithmetic while
preserving rethrows, cleanup precedence, captured catch cells and raw identity.
The alternative of dynamically narrowing every mixed value would leave primitive
addition ambiguous with Promise string conversion; retaining the module summary
would preserve a known source admission gap. No new IR or runtime mechanism is
needed. Closed compositions of pristine resolve/reject calls now distinguish the
raw argument domain of reject from the input rejection domain followed by resolve.
Following produced-value paths avoids confusing an intermediate Promise with its
reason; nested argument failures retain their own lexical routes. Stored values
follow original-symbol initializers only when there are no resolved writes,
including writes in captured bodies. Merged type-only declarations are skipped
when finding that initializer. Deduplicating symbol/depth states avoids exponential
shared-alias traversal while retaining the existing conservative depth bound.
This is a read-only provenance query, not initializer re-execution or flow-sensitive
tracking of mutable cells; written bindings and opaque Promise inputs remain conservative.
Maintained JS/TS native-JS/Node-Wasm/Wasmtime tests cover these boundaries; existing
raw-reason and small-stack tests remain unchanged.

## Mixed intrinsic resolution (2026-10-29)

Promise.resolve selects between returning an existing represented Promise and
materializing a fulfilled General snapshot. This is identity normalization, not
async-return adoption. Reusing typed If/projection/materialization operations
avoids a new runtime opcode or host boundary, and resolving alone leaves rejection
handling unchanged. Direct await can consume the saved raw operand after surplus
arguments instead of allocating an unobservable primitive wrapper. Existing
argument ordering, abrupt completion, source-site and reaction rules still apply.

Constructor processing and new work scheduling use small private lowering helpers
to preserve the existing 2 MiB compiler-stack stress contract. The source profile
continues to exclude arbitrary thenables and unsupported heap/function/String
inputs. Differential value/identity/job/checkpoint tests and unchanged closure
stress tests verify this bounded extension; see [the contract](../async-promise-resolution.md).

## Mutable catch storage provenance (2026-10-29)

The first thrown value does not bound the later contents of a mutable catch cell.
Include assignment provenance when selecting the existing AsyncValue carrier, even
without a module-level raw rejection producer. This reuses current typed storage,
alias/capture flow and copy semantics instead of introducing another mutable box or
restricting assignments to the initial reason kind. Ordinary async return still
adopts a selected Promise; writing a cell neither adopts nor handles it.
Maintained source differentials cover initial primitive snapshots, captured later
writes, logical selection, argument failure and cleanup. Broader value domains and
precise rejection effects remain separate from this storage rule.

## Source-known synchronous call rejection provenance (2026-10-29)

Extend the read-only lexical reason query through all own return expressions of
source-known synchronous local call targets. Union every selected target's possible
rejection domain, preserving raw reject arguments and the earlier proof for calls
that cannot return a Promise. An async body can reject from its own awaits/throws
before returning, so return-expression inspection alone cannot narrow it. Bare
returns and normal completion are primitive Undefined and cannot reject; a complete
RETURN proof is unnecessary. The existing own-return collector excludes nested
function bodies. Synchronous source throw remains unsupported, and unknown
possible-Promise targets retain the conservative carrier.

This reuses the existing original-symbol target and complete-return queries rather
than cloning/inlining calls or adding runtime checks. Deduplicating body/depth
states bounds shared return graphs; existing depth and source guards remain.
Argument failures route independently, and runtime evaluation order, captures,
identity, reaction jobs and handling remain on their existing paths. Differential
source tests and two valid-execution mutations exercise the target-union and async
body boundaries. General mutable-cell and interprocedural precision remain open.

## Source-known async call rejection provenance (2026-10-29)

The earlier conservative async-body boundary is now refined by scanning each
target's own escaping await/throw sources alongside returned expressions. Throws
use the value domain; awaits and async returns use the rejection domain. A local
catch excludes only protected try-block effects, and nested definitions are not
executions. Handler/finalizer throws and returned rejected Promises still escape.
Unknown targets and control-flow overrides remain conservative. This preserves
the existing depth bound and body/depth memoization without a new effect IR or
runtime mechanism.

The existing awaited-call exception boundary also applies to its nested numeric
inline regions. The verifier previously rejected their throws even when handled
locally; it now accepts them under that boundary while retaining raw-handler type
checks and rejecting numeric throws without the boundary. A compiler regression
removes the boundary to verify that guard. Source differential tests cover raw
identity and primitive arithmetic together with full reaction traces. General
argument specialization and finally-sensitive effect elimination remain separate.

## Call-site primitive rejection provenance (2026-10-29)

Refine the existing source-known local call rejection query with primitive
actual-argument proofs for unwritten formals. Keep the shared physical signature
and global parameter facts: the refinement affects only the caller's possible
rejection domain. Propagate those proofs through unwritten aliases, forwarding
and captures, and include the context in both body and alias query caches.

Written formals and uncertain or Promise-valued actuals retain global facts.
Arguments are classified against the incoming context before any callee formal
is shadowed; the query executes no argument or body. Each query interns at most
128 primitive-formal sets, then uses the original module-wide context. This
preserves existing successful proofs when the new precision budget is exhausted.
Depth, source, frame, signature and runtime limits remain unchanged. Source
differentials and deliberate cache/write-guard mutations check raw identity and
primitive arithmetic together; broader Promise-domain and mutable-flow
specialization remain separate work.

The same call context applies to the own escaping await/throw query introduced
above for async targets. Explicit throws use the value proof, and awaits and
returns follow rejection provenance. Async counterparts retain raw first-call
inputs, primitive later calls, written formals and older captured activations;
typed sync and async twins emit identical bytes to their respective JavaScript.

Capture proofs require a stable constructor/alias chain for the called value.
Mutable storage, factory results and callback formals can select a closure from
an earlier activation of the same source body; those calls clear inherited
capture proofs before adding fresh actual-parameter proofs. A maintained
counterexample first stores a closure capturing a Promise, then invokes it while
the same outer source parameter holds a primitive in another activation. Direct,
aliased, selected, factory and callback cases preserve the earlier Promise's
identity. The initial blanket propagation failed that native-JS differential;
the activation guard and its deliberate removal test establish this boundary.
