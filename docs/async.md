# Experimental primitive async profile

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
implicit result. Ordinary numeric/primitive entries and named helpers retain
their explicit completion requirements.

Plain module-level synchronous Number helpers can be called before or after
suspension, inside local closures and from known async bodies. They retain their
existing exact arity (up to 1,000 parameters), Number results and ordinary local
activations; calling a helper adds no reaction job. Helpers still require explicit
Number completion and cannot suspend or throw. The async `run` entry cannot be
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

Settled source results must reach an immediate await, optionally through a
synchronous result-forwarding call or a conditional/comma result. An unshadowed
`Promise.resolve` wrapper without surplus intrinsic arguments is also admitted.
Fulfilled branches explicitly promote their snapshots. Storing, discarding,
coercing, testing or passing potentially rejected snapshots as Promise inputs
remains guarded: these uses need unhandled-rejection tracking. Returning one from
an async body requires adoption, which remains open. Pending promises, general
payloads and identity also remain open.
Known directly awaited async calls retain their existing `AwaitCall` boundary.

Runtime-selected async bodies with their own await now use async private effects and
`AwaitClosureCall`. The backend saves caller state in invocation-local guest
frames, runs each child in the same state dispatcher and restores the caller on
primitive completion. Child awaits and the caller's await keep their separate
reaction boundaries, including uncaught rejection and nested/recursive IR calls.
This reuses the existing host protocol; it does not expose a pending Promise
handle. Direct awaited calls preserve the callee before actual arguments, eager
prefix effects, captures and typed inputs. Async constructor peers share the
effect; callback parameters and synchronous factories returning async functions
retain it. Comma prefixes execute before callee and arguments. Pending result
storage, detached calls, async-return adoption and mixed synchronous/asynchronous
interfaces remain guarded. See [private async IR frames](ir.md#async-private-calls-and-invocation-frames)
for representation, executable evidence and limits.

An await operand may now select a suspending call or an admitted primitive,
fulfilled or settled snapshot with `?:`, including nested comma prefixes. The
condition and prefixes execute once; only the selected branch runs. Each leaf
keeps its own child effects and the one caller reaction of the original await.
Unselected branches still pass source validation. Explicit source-site provenance
lets alternative continuations share that logical site without merging their
runtime jobs. Stored pending values, pending `&&`/`||`/`??` operands and adoption
remain guarded.

The IR/backend now connects stable Promise identities to coroutine frames and
the guest FIFO. `StartAsyncClosure` runs eagerly and returns an identity;
`AwaitPromise` registers a later continuation. Independent calls can overlap,
retain separate saved values, and be awaited later through their stored handles.
Both adapters consume individual enqueue notifications, so a root's settlement
does not cancel other ready activations. Wasmtime's `pending_job_count()` reports
outstanding callbacks separately from `state()`; Node schedules each notification
on the host microtask queue. [IR contracts and limits](ir.md#pending-promise-guest-support)
describe the protocol and arena lifetime. Source pending storage, detached calls,
adoption and unhandled-rejection reporting remain subsequent work.

Current source-generated modules retain the legacy primitive protocol described
below. IR modules using Promise identities additionally export `take_jobs`; their
queue-aware protocol is described in the linked IR contract above.

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
Symbols, BigInts, objects, Promise callbacks, synchronous/lifted throw, finally, catch
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
