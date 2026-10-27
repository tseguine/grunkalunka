# Stored fulfilled primitive Promises

Initialized `const` and `let` locals can hold an unshadowed `Promise.resolve` over
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

A missing argument supplies intrinsic Undefined, regardless of any local binding
named `undefined`. Ordinary extra arguments run once, left to right, before the
call completes; their values are ignored. The first payload is already a snapshot,
so `const p = Promise.resolve(a, a = await b)` retains the original `a`. An extra
argument's rejection skips the remaining arguments and the constructor's completion.
The same ordering applies to `Promise.resolve(p, extra)` aliases. Up to 1,000
ordinary arguments are admitted, within the existing source and await-site limits.

Mutable cells accept another fulfilled Promise through plain `=`. Aliases keep
their earlier snapshot while captured bindings observe later writes. The store
occurs after the RHS completes, including all its argument effects and awaits;
a rejected RHS leaves the old cell intact. Assignment results can themselves be
aliased or awaited. Sequences ending in a Promise, conditional/logical selections
between represented Promises and logical assignments retain the Promise type.
Promises are always truthy and
non-nullish, even when their payload is false, zero, Null or Undefined: `&&=`
selects its RHS while `||=` and `??=` skip it and all its jobs.

Nested `Promise.resolve` adopts an already represented fulfilled value without
adding a job, preserving constructor and surplus-argument order. Discarded
Promise expressions still run their effects. A dedicated mode in the iterative
expression evaluator owns construction, assignment, selection and projection;
nested constructors and assignments do not recursively re-enter that evaluator.

`FulfilledPromise` is a distinct internal IR type. `FulfillPromise` wraps a visible
General completion snapshot; `PromiseValue` projects only that Promise type back
to General before the existing await operation. The verifier rejects scalar
substitution and Promise parameter/entry-result profiles. The backend uses the
existing sixteen-byte tag/f64 encoding for this virtual, nonescaping value,
including captured cells and suspended storage. No Promise object identity, new
host ABI, job scheduler or allocation allowance is introduced.

This is a bounded fulfilled-value slice. Bindings require a fulfilled initializer;
payloads are represented primitives or other admitted fulfilled values.
Rejected/unsettled Promises, thenables, `.then` callbacks, general object payloads,
function arguments/results carrying Promises and observable Promise identity
remain separate work. A Promise cannot be read as its payload in ordinary scalar
positions, returned directly, or used as a primitive rejection reason. Uninitialized
bindings and selections between Promise and primitive values remain separate.
Existing intrinsic shadowing and optional/generic/spread-call guards remain.
Type annotations and wrappers establish no runtime Promise or payload kind.

The maintained `async-suspension` and `runtime-closures` source tests compare
JS/TS, native JavaScript, Node-Wasm and Wasmtime values, effects and job order.
They cover all four primitive kinds, creation snapshots, aliases, repeated waits,
captures, coercions, rejection of an awaited primitive, and the 32/33 await-site
boundary. Argument tests also check absent payloads, ordered surplus effects,
rejection before the outer await, type erasure and 1,000/1,001 arguments.
Mutable-expression tests additionally compare assignment/alias/capture snapshots,
selection and truthiness, nested adoption and rejection before stores. Control
observations check the 127/128 nested-assignment and 128/129 constructor boundaries
with a maximum of one active evaluator entry.
Compiler IR tests separately check constructor/projection types and
parameter/entry-profile rejection. These are not general Promise conformance.
