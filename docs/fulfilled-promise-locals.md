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

Statically known synchronous local arrows can return these fulfilled values,
including concise bodies and blocks that return a fulfilled Promise on every
structural path. Number and represented primitive arguments retain their existing
rules. Results can be stored, aliased, selected, discarded or awaited; a synchronous
call adds no reaction job. Arguments finish before the body, captured bindings are
read when the call executes, and returned values keep their snapshots. Original
semantic binding identities and initializers supply bounded result-kind evidence;
the actual lowering still checks declaration order and every return path.

Runtime-selected, reassigned and returned synchronous closures, including supported
callback forwarding, can return the same fulfilled snapshots. Their private
signatures distinguish Promise results from primitive results despite sharing the
tag/f64 physical pair. Caller evaluation snapshots the callee before arguments;
the call adds no job, and awaiting its result adds the usual caller reaction.
Concise and complete block bodies use the same result-kind proof. Mixed
Promise/primitive result signatures remain rejected.

`FulfilledPromise` is a distinct internal IR type. `FulfillPromise` wraps a visible
General completion snapshot; `PromiseValue` projects only that Promise type back
to General before the existing await operation. The verifier rejects scalar
substitution and exported Promise parameter/entry-result profiles. The backend uses the
existing sixteen-byte tag/f64 encoding for this virtual, nonescaping value,
including captured cells and suspended storage. No Promise object identity, new
host ABI, job scheduler or allocation allowance is introduced.

The private IR also permits fulfilled Promise parameters on closure signatures,
including callback parameters and returned-function signatures. Each argument
occupies a sixteen-byte tag/f64 frame entry, using the shared caller/callee offset
policy. General and Promise entries retain distinct logical types despite their
matching physical layout. Parameter cells can be captured and mutated without
changing earlier loaded snapshots. Public entries and ordinary helpers still
require Number parameters. Source argument inference and binding do not yet
admit Promise parameters; the IR representation is a prerequisite for that work.

This is a bounded fulfilled-value slice. Bindings require a fulfilled initializer;
payloads are represented primitives or other admitted fulfilled values.
Rejected/unsettled Promises, thenables, `.then` callbacks, general object payloads,
Promise parameters and observable Promise identity
remain separate work. A Promise cannot be read as its payload in ordinary scalar
positions, returned directly from an entry point or async arrow, or used as a
primitive rejection reason. Async Promise-return adoption remains separate. Uninitialized
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
parameter/entry-profile rejection. Inline regions may return the Promise type;
their return values must match it, while `AwaitCall` still requires General.
Private closure definitions and indirect calls also check that distinct result
kind. Maintained source cases cover reassignment, returned functions, callbacks,
all primitive payload tags, callee/argument ordering, captures and 1,000/1,001
parameters. Malformed IR cases check logical result mismatches even though the
physical encodings coincide.
Private-parameter IR tests separately execute mixed Promise/Number/General/function
frames, Promise-consuming callbacks, captured parameter writes and the 1,000-argument
boundary in Node-Wasm. Fresh emitted fixtures were also replayed in Wasmtime with
an explicit test fuel budget; the default runtime budget is unchanged. These IR
fixtures do not demonstrate source-level Promise argument admission or new jobs.
These are not general Promise conformance.
