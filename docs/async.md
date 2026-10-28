# Experimental primitive async profile

In this async source profile, represented Promise constructors produce stable
identities. `Promise.resolve` over supported values, `Promise.reject`, and
immediate or suspending async calls now share the `Promise` interface. Aliases,
captured bindings and homogeneous private parameters retain the handle;
`Promise.resolve(p)` returns the same represented Promise. Returning a Promise
from an async body adopts it. See [source constructor identities](#source-constructor-identities).
The snapshot descriptions below also explain the internal completion operations
and earlier optimizations; their former source escape restrictions are superseded.


`grunkalunka_compiler::compile_async(source, typescript)` accepts one exported
`async function run(a,b)` with Number inputs and undefined/null/Boolean/Number
fulfillment values or rejection reasons, and up to 32 await
sites across its structured regions. It preserves the original AST, semantic
binding identities and shared source limits. TypeScript annotations/wrappers do
not coerce values. Existing numeric, primitive and heap entry points keep their
contracts and reject async functions.

The root async function fulfills intrinsic `undefined` for a bare `return;` or
normal completion, including after awaits, loop exits and catches. Partial
return/throw paths keep their original values and rejection reasons. Falling off
the end adds no reaction job, and a binding named `undefined` cannot change the
implicit result. Ordinary numeric/primitive entries and synchronous named helpers
retain their explicit completion requirements. Named async helpers now share the
async completion rules; see [their contract](async-named-helpers.md).

Plain module-level synchronous Number helpers can be called before or after
suspension, inside local closures and from known async bodies. They retain their
existing exact arity (up to 1,000 parameters), Number results and ordinary local
activations; calling a helper adds no reaction job. Helpers still require explicit
Number completion and cannot suspend or throw. Plain named async helpers are
also admitted with Number inputs and private async activations as described in
[the named-helper contract](async-named-helpers.md). The async `run` entry cannot be
called as an ordinary source helper, including from another helper or closure.

The exact current `gap-async-await` source runs in this profile:

```js
export async function run(a,b) { return a + await Promise.resolve(b); }
```

The numeric compatibility corpus still rejects that source: its result contract
is a Number rather than a Promise. This is one source-bound async admission,
not deletion of a numeric-profile gap or general Promise support.

## Suspension and host jobs

Known async local arrows with primitive bodies can be called directly under
`await`. The body may be concise or use the existing block/implicit-return rules.
Both forms may await represented primitives and known async calls, including
rejected waits, source throws and lexical catches.
Block bodies preserve locals, captured cells, branches, loop exits and early,
bare or implicit returns across suspension. For example:

```js
export async function run(a,b) {
  const f = async x => x ? null : false;
  return await f(a);
}
```

Ordered arguments and the synchronous body prefix execute before suspension. Body
awaits and the caller's await remain separate reaction jobs: `async()=>await a`
under a caller await takes two jobs, matching native JavaScript. Const aliases,
direct async arrow calls and pristine
`Promise.resolve` wrapping preserve that behavior. Existing exact-arity,
Number/numeric-leaf callback argument and source/resource checks still apply.

General async function values and Promise object identity are not reified.
[Stored fulfilled primitive locals](fulfilled-promise-locals.md) have a distinct
bounded internal representation, including initialized mutable bindings, typed
assignments/selections and captured-cell writes across suspension. Aliases keep
their earlier values, failed RHS work skips the store, and object truthiness is
independent of the primitive payload. Async arrows with admitted non-suspending
primitive bodies can now be selected, reassigned, returned by factories and passed
as callbacks. Their arguments and body execute before returning a fulfilled
snapshot; the call adds no reaction, and the caller's await retains its job.
Results can be stored, aliased, forwarded, discarded or tested for object truthiness.
The body must actually lower to a primitive completion, including explicit or
fallthrough Undefined; annotations establish no kind. Own-scope await/throw in
runtime async closures, async-return adoption and general Promise/function
outcomes remain separate requirements. `await Promise.reject(f())` is rejected:
the reason would be the Promise object, rather than its eventual primitive value.
`await Promise.reject(await f())` uses a primitive reason and remains supported.

Known synchronous local arrows can return admitted fulfilled Promise values
through concise or complete block bodies. Calling them runs their arguments and
body synchronously; only the caller's await adds a reaction. Returned locals,
aliases and captured bindings keep their typed snapshots. Runtime-selected,
reassigned and returned synchronous closures and supported callbacks also carry
these results through a distinct logical signature kind, using the existing
tag/f64 physical return pair. Represented fulfilled Promise arguments now pass
through these synchronous calls and supported known directly awaited async local
calls. Actual argument facts establish the parameter kind; type syntax does not.
Arguments retain their creation/callee snapshots before later effects or awaits.
Mixed primitive/Promise parameter uses and async-return adoption remain separate
requirements.
The structured `AwaitCall` region separates callee completion from the caller's
reaction. A body throw settles the callee immediately, but the caller observes it
on its own reaction job; a rejected body await adds its body job first. Callee
catches run within that boundary, and caller catches run after the reaction.
Ordered arguments complete outside the boundary, so their failures cannot enter
callee catches. The return and rejection alternatives share one logical await
site under the existing 32-site limit. No Promise-object or host ABI was added.

`void` preserves operand suspension and rejection before producing intrinsic
Undefined. `void(await f())` retains the callee/caller jobs; it cannot turn a
rejection into fulfillment. An unawaited async call remains outside admission
even when its result is discarded with `void`.

Primitive identifier `??=` only enters RHS awaits when the old value is Null or
Undefined. The old value is captured before that work, and a successful RHS is
stored after its final reaction. Rejection skips the assignment store; earlier
RHS effects remain visible to a surrounding catch. Non-nullish values skip all
RHS jobs, including zero, false and NaN.

The compiler lowers await to a Number or General operation in the typed IR.
Known Number operands retain scalar specialization; other represented primitive
values retain their tag and payload across suspension. Await does not coerce a
value: `await true` stays Boolean, `(await true) === 1` is false and `+(await true)`
is 1 through the existing arithmetic conversion. Storage planning and lowering
share operand inspection, including unshadowed Promise intrinsics and wrappers.
The async emitter
partitions the verified regions into continuations and stores source cells and evaluated
snapshots in private mutable Wasm globals. One instance represents one invocation,
preserving values across separate jobs. In `a + await Promise.resolve(a=b)`, the
original left value survives the mutation. Sequential/nested awaits each require
a separate continuation job.

The splitter builds a control-flow graph with explicit branch, loop, yield, return
and suspension edges. A stored program counter selects a state using Wasm `br_table`.
Ordinary edges execute within the current job. Only an executed await returns to
the host; skipped branches and early exits add no jobs. `continue` visits the for
update or do/while condition, and `break` targets the nearest loop exit. A region's
yield copies its typed result, including both fields of a General or Closure value.

This follows the frame/state-index approach described by
[LLVM switched-resume coroutine lowering](https://llvm.org/docs/Coroutines.html#switched-resume-lowering).
It does not depend on LLVM. The frame conservatively retains every typed cell and
snapshot; spilling only values live across suspension is a later optimization.

Captured bindings additionally use [addressable guest cells](captured-cells.md).
The frame retains their pointers across jobs; parameter cells initialize only
on the fresh state. Each instance owns its arena for the invocation's lifetime.
Known [returned local closures](returned-closures.md) retain those cells across
jobs, including when an awaited argument initializes a factory parameter.
[Runtime-selected local functions](runtime-closures.md) also survive suspension.
The activation frame retains both code and environment fields, including captures
of another function value. Lifted bodies use ordinary Wasm locals and the existing
two-pointer numeric calling convention; they do not share the dispatcher's saved
slot offsets. The private table maps logical closure IDs to named lifted bodies,
because `advance`, `start` and `resume` occupy separate physical function indices.
Only the async entry suspends; lifted function bodies remain synchronous.
An admitted eager async closure wraps its checked General completion with
`FulfillPromise` before returning through the existing private pair ABI. A known
directly awaited async call retains its `AwaitCall` boundary, including previously
supported suspension, catches and rejection; this does not make those bodies
runtime closure values. Known eager calls inside `Promise.resolve` with extra
arguments use the fulfilled snapshot path so prefix and argument order remain
observable without inventing a pending Promise.

Runtime async arrows with primitive bodies and no own suspension can also throw.
Their distinct `SettledPromise` result closes synchronous primitive returns/throws
through `SettlePromise`. `AwaitSettled` performs the caller's single reaction and
lexical catch transfer using the same host protocol and canonical primitive tags.
Constructor groups join throwing async arrows with fulfilling async or synchronous
fulfilled-Promise peers. Selection, reassignment, returned factories and supported
callback forwarding preserve code, input and captured-cell snapshots.

Closed completion snapshots now materialize into stable identities when their
source results escape the direct-await optimization. Rejected results can be
stored, discarded, tested for truthiness, passed through homogeneous Promise
parameters and adopted on async return. Rejection notifications retain the
existing host-checkpoint policy.
[Numeric-only Promise coercion](async-number-coercion.md) now works in the identity
profile. String-producing operations, general payloads and thenables
remain separate work.
Known directly awaited async calls retain their existing `AwaitCall` boundary.

Runtime-selected async bodies with their own await now use async private effects and
`AwaitClosureCall`. The backend saves caller state in invocation-local guest
frames, runs each child in the same state dispatcher and restores the caller on
primitive completion. Child awaits and the caller's await keep their separate
reaction boundaries, including uncaught rejection and nested/recursive IR calls.
That directly-awaited form retains the legacy host protocol and a General
completion. Direct awaited calls preserve the callee before actual arguments, eager
prefix effects, captures and typed inputs. Async constructor peers share the
effect; callback parameters and synchronous factories returning async functions
retain it. Comma prefixes execute before callee and arguments. These direct
await paths keep their existing output. Separate pending-value requests now
support storage and detached execution; async-return adoption and mixed
synchronous/asynchronous interfaces remain guarded. See [private async IR frames](ir.md#async-private-calls-and-invocation-frames)
for representation, executable evidence and limits.

An await operand may now select a suspending call or an admitted primitive,
fulfilled or settled snapshot with `?:`, including nested comma prefixes. The
condition and prefixes execute once; only the selected branch runs. Each leaf
keeps its own child effects and the one caller reaction of the original await.
Unselected branches still pass source validation. Explicit source-site provenance
lets alternative continuations share that logical site without merging their
runtime jobs. Homogeneous pending-value selections and storage now use identity
records. Mixed snapshot/identity storage and adoption remain guarded.

The IR/backend now connects stable Promise identities to coroutine frames and
the guest FIFO. `StartAsyncClosure` runs eagerly and returns an identity;
`AwaitPromise` registers a later continuation. Independent calls can overlap,
retain separate saved values, and be awaited later through their stored handles.
Both adapters consume individual enqueue notifications, so a root's settlement
does not cancel other ready activations. Wasmtime's `pending_job_count()` reports
outstanding callbacks separately from `state()`; Node schedules each notification
on the host microtask queue. [IR contracts and limits](ir.md#pending-promise-guest-support)
describe the protocol and arena lifetime. Source now admits pending-capable
async calls in bindings, aliases, reassignment, captured getters, homogeneous
conditional/logical results, object truthiness and discarded positions. Later
awaits preserve primitive completion and lexical rejection handling.
`Promise.resolve(existingPending, extra1, extra2)` preserves the identity and evaluates
extras after the argument, including their effects and suspension. This does not
implement general Promise adoption. Typed annotations remain erased.

Source using these identities exports `take_jobs` and `take_promise_event`; other
source modules retain the legacy primitive protocol described below. Detached
child rejection and subsequent handling are delivered in occurrence order, before
root settlement is exposed at each boundary. Node bridges these notifications to
native rejected Promise proxies, leaving checkpoint reporting to the host;
Wasmtime exposes `take_promise_notifications()` for an embedding's checkpoint
policy. Awaiting a child later in the same checkpoint prevents a false unhandled
event. See [pending source values](runtime-closures.md#pending-source-values).
Completed and pending source Promises now share identities; general payloads and
thenable resolution remain open.

Pending identities can now be passed through homogeneous source parameters in
known calls, runtime-selected functions, higher-order callbacks and factories.
Positive facts follow original async constructors, bindings, aliases and call
arguments to a fixed point; annotations supply no runtime evidence. Every actual
argument and body must still match the inferred private signature. A parameter
retains the passed handle across suspension, captures and later caller binding
replacement. Synchronous helpers may return that same identity; async returns
now adopt identities as described below. Completed and pending source Promises
share one private interface. Represented Promise/General mixtures now use the
private tagged inputs described below. Tests execute 32-parameter calls and separately check the
existing 1,000-parameter admission bound; wide admission is not a production-fuel
guarantee.

### Pending async-return adoption

An async root or private function can now return a represented pending identity,
including from a concise arrow without an explicit `await`. It adopts that
Promise's eventual primitive completion. `AdoptPromise` schedules a registration
job, then uses a separate reaction job to forward fulfillment or rejection; this
keeps native job order rather than rewriting `return p` as `return await p`.
The returning activation saves its frame before registration and restores it
when that job runs, retaining the original handle across sibling calls.

An adopted rejection bypasses the returning function's lexical catches. A throw
while evaluating the returned expression still enters its lexical catch. An
outer caller's explicit await may catch the adopting function's rejection.
Implicit adoption jobs consume the existing 32-job runtime budget, not additional
source await sites. The 32 explicit-site, 4,096-state, arena and fuel limits remain.
Tests compare native JS, Node-Wasm and Wasmtime results and full microtask traces,
including later registration, shared callees, primitive reasons and host error
budgets. Tracking suppresses the adopted child's handled rejection and reports
an unhandled adopting activation once.

This covers represented pending identities only. Fulfilled/settled snapshot
adoption, mixed identity/primitive expression joins, arbitrary thenables and
escaping Promise objects remain gaps. A branch may return a pending identity
while a separate return statement completes with a primitive.

The import-free module exports `start(f64,f64)` and `resume(i32,f64)`, each returning
`(i32,i32,f64)` for status, primitive tag and payload. Status 0 is fulfillment;
status 1 suspends on a resumable primitive await reaction, including a caught
rejection; status 2 suspends on an uncaught rejected primitive. Status 3 is direct
rejection from source throw and settles immediately without a queued reaction.
The synchronous prefix runs in `start`.
For statuses 1/2 the host must enqueue one FIFO microtask, including when the
operand is already settled. That job resumes status 1 or rejects the outer Promise
for status 2. A rejected await skips its protected suffix and enters the nearest
lexical catch handler, if present; otherwise it rejects the activation.
Repeated start and resume after completion/rejection trap.

| Tag | Kind | Payload |
| --- | --- | --- |
| 0 | undefined | positive zero |
| 1 | null | positive zero |
| 2 | Boolean | positive zero or 1 |
| 3 | Number | binary64, including signed zero, NaN and infinities |

Resume receives the validated tag/payload pair unchanged. Both host adapters
reject noncanonical primitive encodings; Rust shares the primitive decoder with
`ValueInstance`. `AsyncState::Fulfilled(Value)` and `Rejected(Value)` expose the
same four runtime kinds. This replaces the earlier Number-only experimental async
ABI; rebuild compiler, adapters and generated modules together.

The maintained [JS adapter](../crates/grunkalunka-runtime/js/async.mjs) supplies
real Promises and `queueMicrotask`, creating a fresh instance per invocation.
It compiles the Wasm module once per factory. In Rust,
`grunkalunka_runtime::AsyncInstance::new(wasm,a,b)` runs the synchronous prefix;
`state()` reports Pending/Fulfilled/Rejected and `run_one_job()` processes at most
one await continuation. A Rust scheduler must arrange these steps in its shared
FIFO job queue. Drop releases the activation. There is no hidden global Rust job
queue or general Promise object representation in the Wasm guest.

Each Wasmtime initialization, start and continuation has 10,000 fuel; at most
32 jobs are admitted per activation, including repeated visits to one await site.
Generated graphs are limited to 4096 states. Traps, protocol errors and exhausted budgets
make that activation unusable. Other instances retain independent state. The JS
adapter also bounds jobs to 32 but does not provide Wasm instruction fuel. These
are cooperating generated-module adapters, not arbitrary-module execution services.

## Admission and remaining work

Await operands can be the four represented primitive kinds, admitted stored
fulfilled Promise locals, or direct calls to unshadowed
`Promise.resolve`/`Promise.reject` over primitive expressions. Missing arguments
supply intrinsic Undefined. Up to 1,000 ordinary arguments are admitted: the first
value is saved, then surplus arguments evaluate left to right and their values are
ignored before the outer reaction. An await in a surplus argument retains its own
job; its rejection skips later arguments and the outer await. Stored fulfilled
constructors use the same rules. `Promise.resolve(asyncCall(), extra)` still needs
pending-Promise storage; explicitly awaiting the first call supplies a represented
primitive and permits the extra arguments. Implicit await uses
intrinsic Promise behavior even if a local variable is named `Promise`.
Parentheses and erased operand wrappers retain depth charges. Numeric blocks,
`if`, conditional/logical expressions, `while`, `for` and `do...while` can suspend,
including loop conditions and updates. Early returns and unlabelled break/continue
preserve their existing structured semantics. Previously admitted local arrow
calls remain available.
Immutable runtime-selected functions can be constructed before or after await,
including awaited conditions and factory arguments. Calls may use awaited Number
arguments or have their Number result awaited. Awaiting a function value itself
remains outside the primitive protocol.
Initialized `let` function bindings can also be reassigned with `=`, `&&=`, `||=` and
`??=` at the same full logical signature. Logical assignment keeps RHS waits and
rejections inside its selected branch: functions select the RHS for `&&=` and skip
it for `||=` and `??=`. The assignment store occurs only after RHS success.
A callee snapshot survives suspension while an argument changes the
binding. Captured function cells share those writes; aliases retain their pairs.
[Known local callback arguments](local-callback-arguments.md) can precede or follow
awaited arguments, and factories can retain those callbacks across later jobs.
Local block-arrow calls run synchronously with a separate return boundary. Await
can surround their numeric result; returning from the closure continues the caller,
including when the body returns from inside a loop. Known async arrows use the
separate immediately awaited call boundary described above.

[Primitive rejected-await catch handlers](async-catch.md) now admit lexical
`try/catch`, optional bindings, nested handlers, mutable/captured reasons and later
awaits. Normal bodies skip handlers, and falling-through handlers continue after
try. Rejected awaits within a handler belong to its enclosing protected scope.
[Primitive source throw](async-throw.md) transfers to a lexical handler immediately
or directly rejects the activation when uncaught. It adds no await reaction job;
its operand can still suspend through explicit await. Async structural paths may
return, throw or fulfill Undefined through normal completion. Throws inside a
known async callee settle that callee; observing them through the caller's await
still requires its reaction job.

General async function values, arbitrary Promise/thenable values, Strings,
Symbols, BigInts, objects, Promise callbacks, synchronous/lifted throw, catch
destructuring and Promise interoperability remain open. Runtime-selected functions
accept Number and numeric-leaf callback parameters and return Number or a
numeric-leaf pair from a concise factory; see [higher-order callbacks](higher-order-callbacks.md)
and [runtime function results](runtime-function-results.md). Callback bodies still take
Numbers and return Number. Existing operator admission limits still apply. Rejected primitive reasons
propagate through the nearest lexical handler or remain uncaught; host
resource/protocol errors are not mapped to JS exceptions.
The per-invocation globals are provisional activation storage, not a final shared
heap or escaping closure design.

## Verification

Run `cargo test --locked --offline -p grunkalunka-runtime --test async-suspension`.
The same modules run in Wasmtime and Node, compared with native JS async functions
for results, rejection reasons, exact Number bits and FIFO marker traces. Cases
include the unchanged original source, saved operands, multiple/nested awaits,
local arrows, independent concurrent invocations, signed zero, NaN/infinity,
conditional Number/Predicate/General yields, short-circuiting, loop conditions and
updates, nearest-loop control, early exits, lexical re-entry and the 32-await boundary.
Tests separately check 33-site rejection, intrinsic shadows, unsupported calls,
typed wrappers, malformed statuses, terminal traps, graph size, fuel and the 32/33
runtime-job boundary using one repeated await site. Primitive cases check retained
kinds, strict/loose equality, numeric conversion, typed metadata, shared cells and
canonical-tag rejection at both initialization and continuation boundaries.
Runtime-function cases check selected environments, shared mutations, alias pairs,
nested lifted/static calls, captured slot kinds, terminal fuel/job failures and
independent interleaved activations. A 1,000-argument call repeats over 32 jobs;
arena cursor checks in both engines observe retained frames and four-page growth.
Catch cases additionally compare primitive reason identity, lexical shadows,
nested rejection, later awaits, loop exits and captured catch binding re-entry.
Source throw cases check immediate transfer/rejection, effects, nested rethrows,
loop/capture behavior and settlement after the 32nd await. Host fuel/job failures
remain terminal even inside a protected body.

The state machine follows [ECMAScript Await](https://tc39.es/ecma262/multipage/control-abstraction-objects.html#await).
Finite tests establish this slice; they do not establish full async conformance.

Concise known synchronous helpers can return existing function pairs before or
after suspension; returned callees and callback arguments retain their snapshots.
This adds no async callback body or Promise value; see
[local function results](local-function-results.md).

### Intrinsic self-resolution TypeError objects

Returning an activation's own Promise now rejects it with a fresh guest TypeError
object before a thenable-registration job is queued. An outer await can catch,
retain and rethrow that reason. Repeated awaits and forwarding preserve the same
identity; separate self-resolutions create separate objects. This closes the
previous mutable-capture case that left a root pending with no ready jobs.

Adoption modules opt into `error_kind(f64) -> i32`. Async tag 4 carries a validated,
invocation-local Error-record ID; kind 1 is the intrinsic self-resolution TypeError.
The Node adapter caches a native TypeError per ID, shared between root outcomes
and rejection notifications. Separate invocations have separate caches. Wasmtime
exposes `Value::TypeError { id }`; IDs have meaning only within their invocation.
The primitive `ValueInstance` protocol still admits only tags 0–3. Malformed
extension payloads, unsupported kinds and guest validation traps terminate an
async invocation, while a valid guest TypeError remains catchable by source.

The new object is truthy and non-nullish, supports strict/loose identity comparison,
and converts to NaN in explicit numeric coercion. Comparisons never treat its ID
as a Number. Catch cells, closures, private calls and notifications retain its ID.
Tests compare Error class and identity rather than engine-specific messages or
stack traces; previous primitive comparisons remain exact.

All four relational operators now retain General operands until comparison. Two
intrinsic Errors have equal primitive strings, even when their object identities
differ: `<` and `>` are false, and `<=` and `>=` are true. The saved `e <= e`
counterexample now passes. Mixed Error/non-string primitive pairs compare through
NaN and return false; explicit `+e <= +e` also remains false. Captured operands,
private calls and suspension retain source evaluation order and snapshots.

This does not implement arbitrary Error constructors/properties, general object
semantics or String values. The comparison shortcut relies on the current single
intrinsic Error kind and fixed message; new kinds or custom coercion require an
extension. String-producing conversions, snapshot adoption and other previously
recorded JS/TS gaps remain open.

### Represented Promise identity equality

Stored identity values from suspending or adopting async calls now support
`===`, `!==`, `==` and `!=`. Aliases compare equal; separate activations compare
unequal even when their completions match. Comparisons preserve left-to-right
snapshots through reassignments, private callbacks and suspension. Comparing with
an awaited primitive compares the object with its completion, not two completions.

Both operands keep their eager effects, but comparison adds no await reaction or
job and does not mark a rejection handled. Tests retain the one child job and
unhandled notification of a discarded rejecting child even when its Promise is
compared. A measured opaque ID supplied as an ordinary Number never aliases the
Promise. TypeScript annotations remain erased and the existing await-site and
runtime-job limits remain unchanged.

The current mixed equality route covers General non-string primitives and the
intrinsic Error extension. It does not add General Strings, custom ToPrimitive,
function-object equality or identity for immediate fulfilled/settled snapshots.
Snapshot materialization remains required for those source Promise domains; see
[the logical contract](ir.md#represented-promise-identity-equality).

### Error payloads in immediate settled snapshots

Runtime-selected immediate async closures now retain intrinsic Error payloads in
both fulfillment and rejection. The old private rejection bit collided with Error
kind 4, corrupting the reason and trapping during canonical payload validation.
The backend now reserves bit 31 for rejection and preserves the General kind in
the low bits. This changes private packed snapshot encodings, not their lane or
frame widths, public statuses, canonical value tags or microtask order.

Maintained source tests cover the original runtime-selected throwing counterexample,
mixed fulfilling/throwing peers, captured reasons, forwarding, repeated rethrow,
numeric coercion and all previous primitive payload kinds. A separate typed IR
fixture checks direct payload/status projection of fulfilled and rejected Errors.
The already-working direct inline throw remains a control. Snapshot conversion to
stable Promise identities and other recorded Promise gaps remain separate work.

### Snapshot identities: IR foundation

The IR now has `MaterializePromise` to allocate a fresh identity from a completed
fulfilled or settled snapshot. It preserves rejection status and intrinsic Error
payloads through the existing Promise kernel, without adding jobs or handling the
rejection. Repeated execution creates distinct identities; aliases retain one.
IR execution tests cover await, adoption, equality, synchronous private creators
and native rejection checkpoints. See [the IR contract](ir.md#completion-snapshot-materialization).

### Source constructor identities

Constructor normalization now allocates once at creation in the async source
profile, never on an alias read. Immediate primitive async bodies keep compact
closed completion operations and materialize the result; bodies that suspend or
adopt use coroutine frames. Synchronous factories and callbacks can return the
same handle. Fulfilled and rejected primitive constructors preserve payloads,
status, eager effects and left-to-right argument evaluation, including ignored
surplus arguments that suspend. Construction adds no reaction or handling;
await and async-return adoption retain their distinct job sequences.

The completed-record kernel helper specializes fresh allocation and settlement,
retaining canonical payload validation, the arena cap and rejection events.
Immutable General/Fulfilled SSA carriers can share physical locals; mutable slots
remain separate. Modules with no coroutine functions keep root globals across
reactions without redundant frame spills. Modules with coroutine children retain
spills and restores. The default 10,000 fuel and 32-job limits are unchanged;
the existing 140-constructor and 1,000-argument stress cases still execute.

Maintained source tests compare values and full microtask traces in native JS,
Node-Wasm and Wasmtime. Former source guards also execute through native
checkpoint reporting and default-budget Wasmtime, including detached rejections.
General Strings, arbitrary objects/functions/Promises as payloads or rejection
reasons and custom thenables remain open. Private mixed returns are described below.
Ordinary numeric/value profiles retain their narrower contracts.

### Selected async return values

Async conditional, comma and logical returns can select a represented Promise
or an admitted General primitive/intrinsic Error. Conditions and comma prefixes run
once in source order; only the selected Promise is adopted, with the existing
registration and reaction jobs. A selected primitive returns directly. Rejected
adoption bypasses the returning activation's catch; throws or rejected awaits
while evaluating the expression retain lexical handling.

Known and runtime async constructors receive the adoption effect when any return
leaf needs it, including through supported async callbacks and captures. Native
JS, Node-Wasm and Wasmtime tests compare values and full microtask traces, with
unchanged source-await, runtime-job and fuel limits. Logical returns test the
original left value once: a Promise remains truthy/non-nullish even when its
completion is zero, null or undefined. Adoption stays inside the selected return
branch, so discarding a Promise with `&&` neither handles it nor adds a job.
Rejection notification tests compare detached failures with native checkpoints.

Homogeneous logical left operands keep the existing direct evaluation route.
Nested left expressions now use the selection-local carrier described below.
Promise/scalar private inputs now use the tagged entries described below.
Synchronous helper starts use the callable prefix entries described below.

### Nested Promise/primitive selections

Nested left operands such as `(a ? p : b) || b` and `(a && p) || b` now
retain their selected kind until the outer operator tests it. An internal
`AsyncValue` carrier holds a General value or a represented Promise without
adopting it. Only the final selected return is resolved. Primitive and Error
payloads, Promise identity, short-circuit effects and rejection handling retain
the existing semantics and budgets.

This carrier does not extend the exported completion protocol. Const/let locals
and represented mixed private inputs/results use it as described below.
Numeric-only coercion and [mixed relational operands](async-promise-relational.md)
now work in this represented domain. Mixed addition remains separate work.
A helper can resolve its selected async return before the caller compares the
resulting primitive/Error value.

### Starting async children from synchronous helpers

A synchronous helper may call a represented async function and receive its
Promise after the child's synchronous prefix. The helper does not await that
Promise or add a reaction merely by forwarding it. Known, runtime-selected and
nested helpers preserve captures, argument order, child identity and FIFO jobs.
Ordinary source helpers can start a child, discard its handle and return a Number.
Detached rejection and later handling remain observable through guest notifications
and native Node checkpoints.

When such a helper exists, the backend provides callable private async entries
which allocate a child activation and Promise, run its prefix through the existing
dispatcher, then restore the caller's frame and continuation. Coroutine callers
spill and restore persistent snapshots across ordinary calls, including re-entry
of one coroutine body before its outer activation suspends. Normal helper locals
remain on the Wasm stack. Modules without these helper starts retain their existing
emission path. No exported protocol, record width, fuel, job or source-await limit
changes. Actual awaits still require an async source function; typed synchronous
helpers still cannot contain suspension. General Promise payloads and thenables
remain separate gaps. Private mixed returns now use the raw carrier below.

### Equality of selected Promise/primitive operands

The four equality operators can compare operands such as `(a ? p : b) === p`
and `(a && p) != (b ? q : null)`. Conditional, comma and logical operands retain
their selected raw kind. Each operand executes once, left before right; the left
snapshot survives RHS mutation, child startup and suspension. Equality does not
adopt, await, handle or otherwise schedule a reaction for a selected Promise.

When both selected values are Promises, equality compares their identities. When
both are General values, existing strict or loose primitive/intrinsic Error
equality applies, including Boolean/Number coercion, NaN and signed zero. A
Promise is unequal to every currently admitted non-string primitive and intrinsic
Error; the inequality operators invert that result. The source General profile
still excludes Strings, so this does not implement Promise/string loose equality,
custom coercion or arbitrary objects. Mixed addition remains a gap; numeric-only
coercion and represented relational operands use the contracts below. Existing
homogeneous comparison paths and all source/runtime budgets remain unchanged.

### Immutable selected-value locals

In the async identity profile, a const initializer may select a represented
Promise or General primitive/intrinsic Error. The local and const aliases retain
that raw tagged snapshot, including across suspension and captured cells. Reading,
discarding, comparing or testing it does not adopt, await or handle a Promise.
Logical selection tests the stored kind before choosing an operand. The same
await cursor also handles direct logical selections of represented Promise and
primitive operands, without requiring an intermediate local.

An explicit await consumes the selected primitive with its ordinary reaction or
awaits the selected Promise; rejection reaches the lexical catch. Returning the
local from an async activation adopts only a selected Promise, whose rejection
bypasses that activation's catch. Primitive returns add no reaction. Static effect
analysis tracks "may contain a Promise" separately from homogeneous Promise
signature facts, preserving the required physical activation for captured returns.

Native JS, Node-Wasm and Wasmtime comparisons cover aliases, captures, original
binding mutation, primitive bits/kinds, Error identity, logical/comma/conditional
consumers and job traces. Notification tests cover discarded rejected values and
later handling. No new layout, exported completion, private signature, fuel, job or
source-await budget is introduced. Mutable represented mixed locals now use the
same carrier as described below. Mixed addition, mixed-target numeric writes,
general Promise payloads, Strings and
thenables remain open.

### Mutable selected-value locals

A let local may now change between represented Promise identities and General
primitive/intrinsic Error values. Planning reads the original initializer and all
simple/logical writes before choosing its existing AsyncValue slot. Uninitialized
locals start as intrinsic undefined. Mixed provenance propagates through aliases
after positive Promise facts converge; may-Promise provenance alone is not a
homogeneous private parameter/result proof.

Simple assignment evaluates the RHS once, stores its raw snapshot, then exposes
that same value to the enclosing expression. Logical assignments test the captured
old kind: Promise objects are truthy and non-nullish, and General values retain
their ordinary Boolean/nullish rules. Skipped RHS expressions neither execute nor
schedule reactions. A failing RHS performs no outer assignment store; its
earlier side effects remain. Aliases keep old snapshots while captured closures
read the current cell.

Await and async return consume assignment results through the same lexical-await
or return-adoption routes as immutable locals. Storage, equality and logical tests
do not handle a rejected Promise. Differential traces and notification tests cover
tag changes, aliases/captures, RHS suspension, selected errors, late handling and
source limits. No backend layout, exported/private ABI, arena, fuel, job or
source-budget changes are needed. Mixed-target numeric compounds/updates, mixed
addition, unrepresented heap values and broader Promise/thenable behavior remain open.

### Mixed private inputs

In the async identity profile, an inferred private parameter may receive a
represented Promise or General primitive/intrinsic Error. Original actual
arguments and forwarding relations join the initializer/write provenance graph.
Mixed formals use AsyncValue; annotations still provide no runtime evidence.

Known calls lift their evaluated inputs without resolving them. Runtime calls,
callbacks and factories use the existing tag/f64 layout in sixteen-byte argument
entries. Argument order, callee snapshots, surplus effects, captures and parameter
reassignment retain the raw kind, bits and identity. Normal calls copy the typed
argument frame; coroutine starts copy typed inputs into activation frames.
Ingress adds no adoption, await, handling or reaction.

The callee may compare or test the input, select it logically, await it or return
it from an async activation using the existing consumers. Native JS, Node-Wasm
and Wasmtime tests cover mixed-width normal frames, simultaneous activations,
17/32-parameter runtime async calls, primitive/Error kinds, factory cells and
erased types. Rejection notifications distinguish passing from later awaiting.
Homogeneous argument layouts and source limits remain; wide execution remains
subject to ordinary fuel and arena limits.

Private results now use the same carrier below. Exported entry inputs remain two
Numbers and exported completions remain General. Function/heap-value mixtures and
mixed addition/relational coercion remain separate work.

### Ordinary private mixed returns

Synchronous private arrows may return either a represented Promise identity or a
General primitive/intrinsic Error. Original return and call provenance propagates
the raw carrier through known/runtime-selected calls, callbacks, factories,
forwarding and captured storage. Concise and block returns use the same pair;
bare returns and normal fallthrough produce intrinsic undefined.

Calls preserve kind, bits and identity, including ordered/surplus argument effects
and callee snapshots. Returning or discarding a rejected Promise neither handles
it nor adds a reaction. An explicit await discriminates the returned variant;
return from an async activation adopts only a selected Promise. Equality,
truthiness and logical selection keep their existing raw-value behavior.

Native JS, Node-Wasm and Wasmtime comparisons cover early returns, aliases/cells,
reassignment, nested calls, rejection timing and erased annotations. Separate
notification tests cover discarded results and later handling. No host protocol,
kernel record, dependency or source budget changes. This does not admit arbitrary
heap values, strings, thenables, mixed addition/relations or synchronous source
exceptions.

### Finally completion scopes

[Async finally](async-finally.md) now preserves pending normal, return, throw and
nearest-loop break/continue completions through cleanup, including cleanup awaits.
An abrupt finalizer overrides the pending completion. A returned Promise remains
raw until all active finalizers finish; only then does async return adopt it.
Nested catches, callbacks and private activations retain their lexical boundaries.
Terminal host failures do not enter cleanup. Existing value and resource limits
remain; this does not add general synchronous source exceptions.

### Numeric conversion of represented Promise/General values

The async identity profile now admits unary +, - and ~, non-additive arithmetic,
bitwise and shift operands, existing unary Math first arguments, and non-additive
compound RHS values on primitive target cells. A represented builtin Promise
converts to NaN without settlement, adoption, handling or jobs. Selected General
values use the existing primitive/intrinsic Error conversion. Source evaluation,
snapshots, surplus arguments, suspension and abrupt completion retain their order.
See [the bounded contract and evidence](async-number-coercion.md). Addition,
mixed-target compounds/updates, legacy snapshot profiles and custom object
conversion remain separate work.

### Relational consumers of represented Promise/General values

All four relational operators now accept represented Promise/General operands.
Operands execute once, left before right, and retain raw snapshots across RHS
mutation, calls and suspension. Builtin Promise primitive strings compare equal
regardless of identity. Against the sole represented intrinsic TypeError string,
the Promise string orders after it. Promise/non-string primitive pairs compare
through NaN and yield false. General/General behavior remains unchanged.
No comparison adopts, handles or schedules a Promise. Custom conversion and
broader object/String domains remain excluded; see [the precise contract](async-promise-relational.md).

## Promise intrinsic type arguments

TypeScript call-site types on pristine Promise.resolve/reject are now erased:
`await Promise.resolve<Missing>(a)` has the same runtime behavior and Wasm bytes
as `await Promise.resolve(a)`. Metadata can contain unresolved names, type queries,
arbitrary type-argument counts or misleading primitive claims; it adds no runtime
binding, lookup, conversion, Promise identity or reaction. This is erasure, not
type checking or a validation of those claims.

The shared intrinsic validator applies this rule to directly awaited, stored,
returned and privately forwarded constructors. Ordinary value arguments still
execute left to right, including surplus arguments and their abrupt completion.
Resolve identity, reject reasons, adoption, catches/finally and notifications keep
the existing contracts. Tests compare typed guest bytes with JS twins and execute
those twins against native JS, Node-Wasm and Wasmtime; notification fixtures also
compare native rejection checkpoints. Four exact former TS guards now execute.

Source bytes, await sites, argument limits, optional/spread calls, value shadowing,
custom/unknown intrinsics and payload domains retain their guards. Source binding
checks still distinguish type-only names from actual value references. Generic
calls through the separate direct numeric module-call validator remain open.
Known local and runtime closure calls use
[the contract below](#closure-call-site-type-arguments); runtime factory-result
calls follow [their contract](runtime-closures.md#runtime-factory-call-site-type-arguments).
Supported unary Math intrinsics follow
[their contract](compiler.md#math-intrinsic-type-arguments). The separate
[heap profile](heap-literals.md) erases JSON call-site types within its existing
String-operand and String-result bounds.
No new IR, type checker, runtime operation, dependency, ABI or budget is introduced.

## Closure call-site type arguments

Known local arrow calls and represented runtime closure calls erase TypeScript
call-site types. This includes async closures, named async helpers, callable
aliases/selections, callbacks and known local factories. For example,
`const f=async x=>await x; return await f<Missing>(a);` emits the same guest bytes
as its JavaScript twin. Unresolved names, type queries, multiple arguments and
misleading primitive claims create no runtime binding, lookup, kind or conversion.

The existing callee is evaluated and saved before ordinary arguments. Declared
inputs and surplus effects retain their order, snapshots, failures and async jobs;
replacing the callable during argument evaluation cannot change the saved callee.
Optional/spread calls, missing inputs, unsupported values and source/argument
limits keep their guards. The separate direct numeric module-call validator
still rejects explicit call-site types; runtime factories follow their contract.

Four maintained async groups compare exact JS/TS Wasm bytes and native JS,
Node-Wasm and Wasmtime outcomes. A synchronous numeric/primitive group checks the
same shared admission rules. The exact older `f<never>(a)` Promise and Math guards
now have positive byte-identity and execution tests. No IR, runtime ABI, frame layout,
dependency, type checker or resource-limit change is introduced.
