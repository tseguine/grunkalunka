# Experimental numeric async profile

`grunkalunka_compiler::compile_async(source, typescript)` accepts one exported
`async function run(a,b)` with Number inputs, Number results and up to 32 await
sites across its structured regions. It preserves the original AST, semantic
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
yield copies its typed result, including both fields of a General value.

This follows the frame/state-index approach described by
[LLVM switched-resume coroutine lowering](https://llvm.org/docs/Coroutines.html#switched-resume-lowering).
It does not depend on LLVM. The frame conservatively retains every typed cell and
snapshot; spilling only values live across suspension is a later optimization.

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
32 jobs are admitted per activation, including repeated visits to one await site.
Generated graphs are limited to 4096 states. Traps, protocol errors and exhausted budgets
make that activation unusable. Other instances retain independent state. The JS
adapter also bounds jobs to 32 but does not provide Wasm instruction fuel. These
are cooperating generated-module adapters, not arbitrary-module execution services.

## Admission and remaining work

Await operands can be Numbers or direct one-argument calls to unshadowed
`Promise.resolve`/`Promise.reject` over a Number expression. Implicit await uses
intrinsic Promise behavior even if a local variable is named `Promise`.
Parentheses and erased operand wrappers retain depth charges. Numeric blocks,
`if`, conditional/logical expressions, `while`, `for` and `do...while` can suspend,
including loop conditions and updates. Early returns and unlabelled break/continue
preserve their existing structured semantics. Previously admitted local arrow
calls remain available.
Local block-arrow calls run synchronously with a separate return boundary. Await
can surround their numeric result; returning from the closure continues the caller,
including when the body returns from inside a loop. Async arrows remain excluded.

Source helper calls, async arrows, arbitrary Promise/thenable values, objects, callbacks,
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
conditional Number/Predicate/General yields, short-circuiting, loop conditions and
updates, nearest-loop control, early exits, lexical re-entry and the 32-await boundary.
Tests separately check 33-site rejection, intrinsic shadows, unsupported calls,
typed wrappers, malformed statuses, terminal traps, graph size, fuel and the 32/33
runtime-job boundary using one repeated await site.

The state machine follows [ECMAScript Await](https://tc39.es/ecma262/multipage/control-abstraction-objects.html#await).
Finite tests establish this slice; they do not establish full async conformance.
