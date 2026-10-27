# Stored fulfilled primitive Promises

Immutable locals can hold an unshadowed, one-argument `Promise.resolve` over
Undefined, Null, Boolean or Number. The payload evaluates once at construction:

```js
export async function run(a, b) {
  const p = Promise.resolve(a);
  a = b;
  const alias = p;
  await b;
  return await alias;
}
```

The result is the original `a`, preserving signed zero and non-NaN Number bits. Construction adds no
reaction job. Each await still adds its ordinary job, including repeated awaits
of the same fulfilled local. Primitive tags and captured bindings survive
suspension. `Promise.resolve(p)` aliases an admitted fulfilled local rather than
adding a job. Known directly awaited async arrows can capture these locals.
Construction and aliasing can also occur in synchronous code; their argument
effects run even when the local is unused.

`FulfilledPromise` is a distinct internal IR type. `FulfillPromise` wraps a visible
General completion snapshot; `PromiseValue` projects only that Promise type back
to General before the existing await operation. The verifier rejects scalar
substitution and Promise parameter/entry-result profiles. The backend uses the
existing sixteen-byte tag/f64 encoding for this virtual, nonescaping value,
including captured cells and suspended storage. No Promise object identity, new
host ABI, job scheduler or allocation allowance is introduced.

This is a bounded fulfilled-value slice. Bindings require `const`; payloads are
represented primitives or aliases of admitted fulfilled locals. Mutable bindings,
rejected/unsettled Promises, thenables, callbacks, general object payloads,
function arguments/results carrying Promises and observable Promise identity
remain separate work. A Promise cannot be read as its payload in ordinary scalar
positions, returned directly, or used as a primitive rejection reason. Existing
intrinsic shadowing, optional/generic/spread-call and argument-count guards remain.
Type annotations and wrappers establish no runtime Promise or payload kind.

The maintained `async-suspension` and `runtime-closures` source tests compare
JS/TS, native JavaScript, Node-Wasm and Wasmtime values, effects and job order.
They cover all four primitive kinds, creation snapshots, aliases, repeated waits,
captures, coercions, rejection of an awaited primitive, and the 32/33 await-site
boundary. Compiler IR tests separately check constructor/projection types and
parameter/entry-profile rejection. These are not general Promise conformance.
