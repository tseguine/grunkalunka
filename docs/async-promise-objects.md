# Host transport for Promise-valued reasons

Queued async modules opt in by exporting `promise_state(f64) -> (i32, i32, i32, f64)`.
The argument is an invocation-local Promise ID. The emitted reader validates its
range, alignment, allocation extent and record magic, then returns:

| Field | Meaning |
| --- | --- |
| state | 0 pending, 1 fulfilled, 2 rejected |
| handled | 0 or 1; guest handler state |
| tag, payload | Canonical completion value; both zero when pending |

The reader does not handle, adopt, allocate or enqueue anything. Promise references
use tag 5 with an aligned record ID. That tag is admitted only for rejected values
and rejection notifications. Fulfillment still resolves/adopts in the guest and
exposes the existing General kinds. A pending Promise, including one rejected with
itself later, can be a raw rejection reason without resolving the outer rejection.

Node decodes one real native Promise per guest ID per invocation. The same object
appears as a root reason, another Promise's reason and its rejection-event identity.
Its settlement is reflected at invocation boundaries, including jobs after root
settlement. Iterative decoding supports nested and cyclic reasons. Guest handling
suppresses the corresponding native unhandled report; external native handlers
also work normally. This does not mutate guest handling through the read API.

Wasmtime exposes `Value::Promise { id }`. Use the owning
`AsyncInstance::promise_state(id)` to read Pending, Fulfilled or Rejected state.
Queries preserve outstanding jobs and notifications. IDs have no meaning in a
different invocation. Primitive ValueInstance and numeric/primitive CLI APIs do
not admit this extension.

Both adapters validate snapshot kinds, canonical payloads, immutable settlement,
monotone handling and notification consistency. Rejection events cannot be replayed
after handling in this opt-in protocol. Query, trap, fuel and protocol failures are
terminal. Node also rejects exposed pending objects on terminal host failure rather
than leaving them hanging; these failures do not invent guest notification events.
Production job, fuel and arena limits are unchanged.

Maintained adapter tests compare native Promise branding, identity, outcomes,
reaction order and unhandled/handled checkpoints. Wasmtime tests check typed
queries, terminal failure and actual emitted record validation. Small protocol
peers isolate malformed roots, snapshots and events from source lowering.

The async source profile admits `throw` and intrinsic `Promise.reject` of
represented Promise identities and Promise/primitive selections, including named
and local async bodies. Neither operation adopts or handles the reason. Catch
bindings use the existing AsyncValue carrier for incoming raw reasons or later
Promise writes; finally buffers use it where source routes may deliver a raw
reason. A raw producer in one handler or nested function no longer widens
unrelated primitive catches. The fixed-point
query routes throws and awaited reasons to the nearest protected lexical catch,
through intervening finally buffers. Exceptions from a handler or cleanup bypass
that scope's own catch; nested function bodies reach callers through their Promise.
Direct and closed composed awaits retain General reasons when input provenance
proves no raw rejection. Promise.reject uses its argument value domain, while
Promise.resolve follows the rejection domain of its input. The query follows
conditional/logical value arms, final sequence expressions and simple-assignment
right-hand sides through erased wrappers. Nested argument failures still route
from their own throw/await nodes; source validation and limits are unchanged.
Later Promise writes still
widen catch cells, including captured cells, without losing initial primitive tags.
Stored values now follow their original-symbol initializers when the binding has
no resolved writes, including const aliases, unwritten let bindings and captures.
Type-only declarations do not hide the value initializer. Shared alias/depth states
are visited once per query; the existing depth bound keeps cycles conservative.
The query never re-executes an initializer. Written bindings, opaque inputs and
full interprocedural reason precision remain follow-up work.

Native JS, Node-Wasm and Wasmtime tests cover exact former local/named throw
exclusions, caught identity, aliases, captures, recursive forwarding, mutable
reason snapshots, rethrows, suspended cleanup and abrupt cleanup precedence.
Compiled-source host tests also cover pending, nested and self-rejecting reasons,
shared notification identities, external handling and reaction order. A separate
internal continuation-payload check permits caught raw values on a normal routing
edge; Promise fulfillment validation still forbids raw Promise payloads.

Intrinsic rejection snapshots its first argument before evaluating surplus
arguments. A failure while evaluating any argument prevents the outer rejection
constructor. Direct await of a raw rejection materializes and awaits its wrapper,
preserving the ordinary single rejection reaction without awaiting the reason.
Returning the wrapper from an async body adopts the wrapper, leaving its reason
unchanged. A pristine reject has no normal completion; numeric projections of
its directly awaited result are unreachable and never coerce the reason.

General synchronous throw, full interprocedural reason inference, thenables, String/heap
values and general Promise construction remain separate gaps.

## Mutable catch cells

A catch variable can start with an admitted primitive and later receive a
represented Promise, even when the module has no Promise-valued throw or rejection
producer. Its original-symbol assignment provenance selects existing mixed storage;
the incoming primitive retains its tag. Saved aliases remain snapshots, while
captured getters observe later writes to the same cell. Logical assignments retain
selection and skipped effects; a failed right-hand side leaves the old cell value.
Returning the selected Promise still uses ordinary async-return adoption.

Maintained JS/TS native-JS/Node-Wasm/Wasmtime tests cover primitive tags, identity,
captures, private helpers, suspension, failure and finally precedence. String, heap
and function values remain outside this catch-cell profile. No new IR operation,
host protocol, dependency or resource limit is introduced.

## Rejection domains of synchronous local calls

When a represented call may return a Promise, await-reason inference follows every
returned expression of its source-known synchronous local targets. This
keeps primitive intrinsic rejections usable in catch arithmetic through aliases,
selected callees, captures, factories and short forwarding chains. All targets
contribute: one raw Promise reason still requires the existing mixed carrier.

Calls already proved to return only primitives retain their earlier classification,
including implicit undefined completion. Bare returns and fallthrough contribute non-rejecting Undefined. Unknown
possible-Promise targets remain conservative. Async bodies additionally require
the escaping effects described below. Source
argument failures retain their own lexical routes. This query executes no call or
initializer. Current admission still forbids synchronous source throw and validates
unsupported bodies; the query adds no runtime inlining, reaction, handling or adoption. Repeated
body/depth states are visited once under the existing depth bound. Existing source,
signature, value-domain and resource guards remain in force.

Maintained JS/TS native-JS/Node-Wasm/Wasmtime tests compare primitive tags, raw
identity, argument/callee snapshots, cleanup and full reaction traces. Typed erasure
has an explicit byte-identical twin. Deliberately ignoring a selected target or an
async body's effects is caught by valid guest identity results. This is bounded
source-known provenance, not general flow-sensitive/interprocedural inference.

## Rejection domains of asynchronous local calls

Source-known async targets contribute their own escaping throws and awaits as well
as returned expressions. An explicit throw preserves its argument as a value;
await and async return follow a Promise's rejection domain. A fulfilled Promise
thrown as a reason therefore remains raw, while returning it cannot reject.
Every possible source target contributes, including a nested function that is
actually awaited. Merely defining a nested function contributes no escaping effect.

A catch protects only its try block. Its handler and cleanup may still throw to an
outer scope. Locally handled sources do not widen the caller's catch, while a
returned rejected Promise is adopted outside the local catch. This is conservative
source provenance: unknown targets, argument-wide facts and paths overridden by
finally may still widen the domain. It does not execute source code or change jobs,
handling, evaluation order, values or identity.

Numeric inline results within an awaited async call retain that call's exception
boundary in IR validation. Ordinary numeric calls still cannot throw. Maintained
JS/TS native-JS/Node-Wasm/Wasmtime tests exercise local handling, own and nested
effects, raw identity, snapshots, cleanup, primitive tags, typed byte identity and
existing argument/await limits.
