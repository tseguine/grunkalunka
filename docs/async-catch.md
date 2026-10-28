# Primitive rejected-await catch handlers

The async profile admits `try { ... } catch (reason) { ... }` and optional
catch bindings. For example:

```js
export async function run(a,b) {
  try {
    return await Promise.reject(a);
  } catch (reason) {
    return reason + await b;
  }
}
```

An executed rejected await enters its nearest lexical handler on the await's
reaction job. A normal body skips its handler. A falling-through handler continues
after the try statement; returns and nearest-loop break/continue retain their
existing meanings. A rejected await in a handler belongs to an enclosing try, if
present. With no enclosing handler it rejects the activation as before.

The catch binding is a mutable General cell identified by its original semantic
symbol. It preserves undefined, null, Boolean and Number reasons without coercion,
including signed zero, finite Number bits, NaN and infinities. Shadows do not alter
outer bindings. Re-entering a handler initializes a fresh captured cell, so closures
retaining previous catch bindings continue to see their own lexical instance.
TypeScript annotations and expression wrappers do not establish runtime kinds.

Lowering emits a structured `TryCatch` operation. Its General reason value is
visible only in the handler, and the verifier rejects reason leaks or references
to body-private snapshots. The coroutine splitter builds the handler under the
outer exception target, then installs that handler while splitting the protected
body. For a caught rejected await it saves a handler continuation and emits the
existing status 1 protocol: the adapter schedules the reaction and resumes with
the original tag/payload. This status therefore means a resumable await reaction,
including a caught rejection. Status 2 still denotes an uncaught rejected await.
The adapters and start/resume signatures are unchanged.

Catch parameters consume source-local budget. Both protected and handler awaits
consume the existing 32-site limit; executed reactions consume the 32-job limit.
Fuel, traps, invalid protocols and exhausted host job budgets remain terminal host
failures and cannot be caught by source code.

[Primitive source throw](async-throw.md) now shares these handlers, transferring
immediately without an await reaction. [Async finally](async-finally.md) now runs
cleanup after protected/caught completion, including suspended cleanup and abrupt
overrides. General synchronous source exceptions, catch destructuring and broader
Promise/thenable values remain excluded. Existing primitive operator limits remain.
It does not establish general exception or async conformance.

Validated awaited `Promise.resolve`/`Promise.reject` payloads participate in scalar
input analysis for known calls, runtime closures, factories and callbacks. A
callback argument such as `await Promise.resolve(null)` can establish a General
input without a prior call. A rejected argument skips the callback body; callee
and earlier argument effects still occur before the await. Payload kinds and
reaction-job order use the existing protocol. This does not admit Promise objects
outside await or relax the unshadowed intrinsic, arity, optional-call, spread or
type-argument guards.

The maintained `async-suspension` suite compares Wasmtime and the JS adapter with
native JS results, primitive kinds and Number bits, complete FIFO marker traces,
and interleaved activations. Catch cases cover all represented reasons, shadowing,
mutation, skipped suffixes, nested handlers, later awaits, loop control and captured
binding re-entry. Boundary tests check typed erasure, unsupported syntax, source
limits, job budgets and terminal Wasmtime fuel failures. Compiler IR tests reject
four malformed reason visibility/type cases.
