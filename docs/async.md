# Experimental numeric async profile

`grunkalunka_compiler::compile_async(source, typescript)` accepts one exported
`async function run(a,b)` with Number inputs, Number results and up to 32 await
points in its outer straight-line region. It preserves the original AST, semantic
binding identities and shared source limits. TypeScript annotations/wrappers do
not coerce values. Existing numeric, primitive and heap entry points keep their
contracts and reject async functions.

The exact current `gap-async-await` source runs in this profile:

```js
export async function run(a,b) { return a + await Promise.resolve(b); }
```

The numeric compatibility corpus still rejects that source: its result contract
is a Number rather than a Promise. This is one source-bound async admission,
not deletion of a numeric-profile gap or general Promise support.

## Suspension and host jobs

The compiler lowers await to a Number operation in the typed IR. The async emitter
partitions the outer region into continuations and stores source cells and evaluated
snapshots in private mutable Wasm globals. One instance represents one invocation,
preserving values across separate jobs. In `a + await Promise.resolve(a=b)`, the
original left value survives the mutation. Sequential/nested awaits each require
a separate continuation job.

The import-free module exports `start(f64,f64)` and `resume(f64)`, each returning
`(i32,f64)`: status 0 is fulfillment; status 1 suspends on a fulfilled Number;
status 2 suspends on a rejected Number. The synchronous prefix runs in `start`.
For statuses 1/2 the host must enqueue one FIFO microtask, including when the
operand is already settled. That job resumes status 1 or rejects the outer Promise
for status 2. A rejected await never executes following source operations.
Repeated start and resume after completion/rejection trap.

The maintained [JS adapter](../crates/grunkalunka-runtime/js/async.mjs) supplies
real Promises and `queueMicrotask`, creating a fresh instance per invocation.
It compiles the Wasm module once per factory. In Rust,
`grunkalunka_runtime::AsyncInstance::new(wasm,a,b)` runs the synchronous prefix;
`state()` reports Pending/Fulfilled/Rejected and `run_one_job()` processes at most
one await continuation. A Rust scheduler must arrange these steps in its shared
FIFO job queue. Drop releases the activation. There is no hidden global Rust job
queue or general Promise object representation in the Wasm guest.

Each Wasmtime initialization, start and continuation has 10,000 fuel; at most
32 jobs are admitted per activation. Traps, protocol errors and exhausted budgets
make that activation unusable. Other instances retain independent state. The JS
adapter also bounds jobs to 32 but does not provide Wasm instruction fuel. These
are cooperating generated-module adapters, not arbitrary-module execution services.

## Admission and remaining work

Await operands can be Numbers or direct one-argument calls to unshadowed
`Promise.resolve`/`Promise.reject` over a Number expression. Implicit await uses
intrinsic Promise behavior even if a local variable is named `Promise`.
Parentheses and erased operand wrappers retain depth charges. Non-suspending
numeric control flow and previously admitted local arrow calls remain available.

Await inside branches/loops/blocks, returns inside control flow, source helper
calls, async arrows, arbitrary Promise/thenable values, objects, callbacks,
try/catch/finally and Promise interoperability remain open. Rejected Number reasons
propagate uncaught; host resource/protocol errors are not mapped to JS exceptions.
The per-invocation globals are provisional activation storage, not a final shared
heap or escaping closure design.

## Verification

Run `cargo test --locked --offline -p grunkalunka-runtime --test async-suspension`.
The same modules run in Wasmtime and Node, compared with native JS async functions
for results, rejection reasons, exact Number bits and FIFO marker traces. Cases
include the unchanged original source, saved operands, multiple/nested awaits,
local arrows, independent concurrent invocations, signed zero, NaN/infinity,
numeric control flow and the 32-await boundary. Tests separately check 33-await
rejection, intrinsic shadows, unsupported control flow/calls, typed wrappers,
malformed statuses, terminal traps, fuel and the job budget.

The state machine follows [ECMAScript Await](https://tc39.es/ecma262/multipage/control-abstraction-objects.html#await).
Finite tests establish this slice; they do not establish full async conformance.
