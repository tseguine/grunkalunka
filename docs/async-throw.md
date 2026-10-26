# Primitive source throw in the async profile

`compile_async` accepts `throw expression` when the expression produces represented
undefined, null, Boolean or Number values. For example:

```js
export async function run(a,b) {
  try {
    throw a;
  } catch (reason) {
    throw reason + await b;
  }
}
```

Throw evaluates its operand once, in source order, then skips its sequential suffix.
A nearby lexical catch receives the original primitive pair immediately. No await
reaction job is added for this transfer. A throw inside a handler belongs to its
outer protected scope. If no handler exists, the returned Promise rejects directly.
A throw after an await uses that already-running continuation job; it adds no job
of its own. Await within the throw operand retains its ordinary reaction boundary
and can itself reject before the throw is reached.

Reason kinds, signed zero, finite Number bits, NaN and infinities survive transfer.
Catch bindings retain their mutation, lexical shadowing and captured-cell semantics.
Normal/return/throw/break/continue completion alternatives compose separately;
catch replaces the protected body's throw channel with its own alternatives.
Async structural paths may return or throw, while implicit fallthrough remains
excluded. Existing conservative loop/await path analysis still applies.

`Exit::Throw` carries a visible General reason. The verifier rejects scalar, missing or
out-of-scope definitions at this exit, and synchronous emission rejects throw IR.
The coroutine splitter emits a direct copy-and-jump to a handler, or terminal
protocol status 3 when uncaught. The Rust and JS adapters validate the primitive
encoding and settle status 3 immediately. Status 2 retains its separate meaning:
an uncaught rejected await requiring one queued reaction. The start/resume function
signatures and four primitive tags are unchanged; rebuild generated modules and
adapters together when consuming this experimental protocol.

Source throw consumes no await-site or job budget. Throw after the 32nd permitted
await can still settle the activation. A 33rd await, malformed protocol, fuel
exhaustion or Wasm trap stays a terminal host failure and cannot enter a source catch.
Source-size, statement, local, nesting, graph and fuel limits remain in force.

Synchronous/lifted throw, finally, Strings, objects, function reasons and general
Promise/thenable values remain outside this unit. This is a primitive exception
slice, not general JS exception or async conformance.

The maintained async suite compares native JS with Wasmtime and the JS adapter for
primitive results, Number bits, exact FIFO traces and job counts. Cases cover direct
and awaited throws, operand effects, nested handlers, loops, captured catch cells,
all represented reasons and rejection after 32 awaits. Boundary checks retain
synchronous-profile diagnostics, erased TypeScript behavior, canonical primitive
encodings on every status at start/resume, and terminal host fuel/job failures.

Combined cases also construct and return higher-order functions through local block
bodies, then use their results as throw reasons or invoke them from catch across
await. These retain callback signatures and catch captures without changing throw
reaction timing.
