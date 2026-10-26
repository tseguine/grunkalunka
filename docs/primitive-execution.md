# Primitive results through the Node worker runner

`runPrimitive()` in `tools/runtime/primitive.mjs` executes the existing
[primitive-return-v1 ABI](adr/0002-tagged-primitive-values.md) and returns its
JavaScript value: `undefined`, `null`, Boolean or Number. Both inputs are still
Numbers. This is an execution adapter, not additional source-language support.

```js
import { compileSource } from './tools/build/compile.mjs';
import { runPrimitive } from './tools/runtime/primitive.mjs';

const { wasm } = await compileSource({
  compiler: `./target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`,
  language: 'ts',
  profile: 'primitive',
  source: 'export function run(a: number, b: number) { return a < b ? null : a; }',
});
console.log(await runPrimitive(wasm, 1, 2)); // null
console.log(await runPrimitive(wasm, 4, 2)); // 4
```

Each invocation snapshots the selected `Uint8Array`/Buffer view into a fresh
worker, compiles and instantiates there, then calls `run`. Caller storage remains
attached. No imports are granted, even unused ones. The existing canonical decoder
validates the returned pair before it becomes a successful result; unknown tags,
noncanonical nullish/Boolean payloads, and scalar results reject rather than coerce.
Signed zero, infinities and NaN-as-a-Number survive; NaN payload bits are not promised.
Strings, objects, asynchronous results and arbitrary runtime ABIs are not supported.

The options and `ExecutionError` class are shared with `runNumeric`: `timeout`
defaults to 5,000 milliseconds and must be an integer from 1 to 2,147,483,647;
`signal` is an optional `AbortSignal`. Already-aborted requests launch no worker.
Invalid configuration rejects with `TypeError`/`RangeError`. Execution, validation
and decoding failures use `ERR_WASM_EXECUTION`; deadline expiry uses
`ERR_WASM_TIMEOUT`; cancellation uses `ABORT_ERR` and retains the abort reason.
Every started worker is terminated before the returned promise settles, including
when initialization or the exported function never terminates.

The deadline is a parent-side wall-clock watchdog, not Wasmtime fuel. Buffer copying
and synchronous worker construction happen before its timer is installed; a blocked
parent event loop can delay it. There is no hard memory limit or full Wasm signature
inspection. Use trusted local artifacts, not arbitrary hostile modules. Every call
gets a new instance; state is not shared between calls. Neither the compiler nor the
numeric API's behavior is changed by selecting this separate entry point.

Run `node --test tests/build/execution.test.mjs tests/build/primitive-execution.test.mjs`
for both execution contracts. After building the native CLI, run
`node --test tests/build/primitive-integration.test.mjs` for actual JS/TS compilation
and execution. The canonical check script runs all three. The tests cover canonical
values, malformed pairs, wrong profiles, imports/traps, snapshot isolation, and
bounded termination/recovery. No runtime speed or memory-safety claim follows.
