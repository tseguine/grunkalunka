# Primitive source throw in the async profile

`compile_async` accepts `throw expression` when the expression produces represented
undefined, null, Boolean or Number values, or represented Promise identities
and Promise/primitive selections. For example:

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
A nearby lexical catch receives the original value immediately; Promise reasons
retain identity without adoption or handling. No await reaction job is added for
this transfer. A throw inside a handler belongs to its
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

`Exit::Throw` carries a visible General or AsyncValue reason in the async profile.
The verifier rejects scalar, missing or out-of-scope definitions at this exit, and synchronous emission rejects throw IR.
The coroutine splitter emits a direct copy-and-jump to a handler, or terminal
protocol status 3 when uncaught. The Rust and JS adapters validate the completion
encoding, including opt-in Promise references, and settle status 3 immediately. Status 2 retains its separate meaning:
an uncaught rejected await requiring one queued reaction. The start/resume function
signatures and four primitive tags are unchanged; rebuild generated modules and
adapters together when consuming this experimental protocol.

Source throw consumes no await-site or job budget. Throw after the 32nd permitted
await can still settle the activation. A 33rd await, malformed protocol, fuel
exhaustion or Wasm trap stays a terminal host failure and cannot enter a source catch.
Source-size, statement, local, nesting, graph and fuel limits remain in force.

Synchronous/lifted throw, Strings, heap objects, function reasons and general
thenable values remain outside this unit. Intrinsic Promise.reject(p) also
preserves the raw reason without adoption or handling. This is a bounded exception
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

An [async finally](async-finally.md) now saves a pending General or raw throw reason
through cleanup, then rethrows to the enclosing lexical handler. A finalizer's
own abrupt completion overrides it. Fuel, trap and job failures remain terminal
host failures and do not trigger source cleanup.

Promise-valued throw coverage includes named/local async rejection, recursive
forwarding, aliases, captures, raw rethrows and cleanup overrides. Returning a
caught Promise still adopts it; throwing does not. Raw host reasons expose native
Promise objects in Node and invocation-local descriptors in Wasmtime. See
[the transport and source limits](async-promise-objects.md), including conservative
module-level catch inference.
