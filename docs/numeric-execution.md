# Bounded numeric execution in Node

`runNumeric(wasm, a, b, { timeout = 5000, signal } = {})` from
`tools/runtime/numeric.mjs` executes one import-free numeric artifact and returns
its Number result. The numeric example uses this after compilation so a looping
edit cannot indefinitely occupy its main thread.

Each call copies the selected `Uint8Array` view (including Node Buffers), then
compiles, instantiates and calls `run` inside a fresh worker. Caller buffers are
not transferred or retained. Shared backing memory is rejected. Arguments must
already be Numbers; signed zero, infinities and NaN are preserved without coercion.
The artifact must have no imports and a function export named `run` returning
one Number. This is not a complete Wasm signature validator: an i32 result is
also a JavaScript Number. Use the compiler's numeric profile, not tagged returns.

`timeout` is a positive integer in milliseconds, at most 2,147,483,647. It covers
worker startup and guest compilation, initialization and invocation, not source
compilation or copying the input bytes. `signal` optionally supplies cancellation;
a pre-aborted request creates no worker. Every outcome waits for the worker to
terminate before the returned promise settles. No instance or guest state is
reused between calls.

Invalid arguments reject with TypeError or RangeError. `ExecutionError.code` is
`ERR_WASM_TIMEOUT` for deadline expiry, `ABORT_ERR` for cancellation, or
`ERR_WASM_EXECUTION` for worker/module/execution failures. Cancellation preserves
`signal.reason` as the cause; worker errors preserve their original cause. Messages
from the engine are not a stable interface. Worker construction failures propagate
normally. Timeout/cancellation race with successful completion; the first observed
outcome wins.

This is responsiveness for trusted local artifacts, not a hardened hostile-code
service. Workers share the process; guest memory, native allocations and aggregate
concurrency are not bounded here. Termination uses Node's asynchronous
[worker termination API](https://nodejs.org/api/worker_threads.html#workerterminate),
not a real-time deadline guarantee. Keep the parent event loop responsive. There
is no fuel meter, persistent instance, host capability grant or browser API here.

`node --test tests/build/execution.test.mjs` runs independent valid-Wasm fixtures
for Number transport, isolated calls, buffer snapshots, import denial, traps,
configuration, cancellation and infinite invocation/start-function timeouts.
Nontermination cases have an outer process watchdog. The example integration
suite also compiles an infinite source loop, observes failure, edits it to a
terminating program and runs again. Both suites run in the canonical checks.
