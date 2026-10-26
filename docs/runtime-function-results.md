# Runtime-selected concise factory results

Synchronous concise factories selected at runtime return numeric-leaf function
pairs in the Number, primitive-return and primitive async entry profiles:

```js
export function run(a, b) {
  const one = x => () => ++x;
  const two = x => () => x + b;
  const make = a ? one : two;
  const f = make(a), g = make(b);
  return f() * 100 + g() * 10 + f();
}
```

Each creator invocation allocates fresh captured parameter cells. Returned code
and environment fields are snapshots; the cells addressed by an environment stay
shared within that closure's lifetime. Cells belong to the enclosing invocation
arena, so returning from a factory does not invalidate them. Existing captured
function bindings can also be returned without constructing a new arrow.

Factory aliases, same-signature rebinding, conditionals, comma effects, recursive
rebinding through initialized cells and immediate returned calls preserve source
order. The callee pair is saved before arguments; arguments finish left-to-right
before the runtime creator runs. Runtime factories may also receive numeric-leaf
callback parameters. Their returned leaf can retain those fresh parameter cells.
Async callers preserve factories and returned pairs across awaits and catch jobs.

Logical signatures now include ordered parameter types and a result type. Numeric
leaf IDs 0 through 1,000 keep their prior meaning: Number inputs and Number result.
Richer IDs describe callback parameters or a function result. Pure kind queries
lazily intern compilation metadata without evaluating source effects; scratch
checking shares that registry but cannot publish its scratch IR. Result references
in the registry are acyclic. Runtime bodies currently return Number or a numeric
leaf Closure type. Selected or assigned factory values must agree on the complete
signature, including the returned leaf's arity.

Each IR function records its result type. The verifier checks it against the
entry profile, lifted-body contract, constructor signature and call result.
Number calls retain `(environment, arguments) -> f64`; factory calls use the
additional private `(environment, arguments) -> (i32, i32)` table call type.
Frames and typed cells retain their existing layouts. The host ABI is unchanged.

Eight active static/runtime function-result lowering frames bound native cursor
reentry; a 2 MiB stack probe accepts eight nested runtime-result calls and rejects
the ninth. Generated-region, source-depth, 256-expansion, lifting, arena and fuel
bounds remain. Wide tests cover 1,000 creator arguments and 1,000 returned-leaf
arguments without enlarging physical Wasm parameter lists.

Tests compare native JavaScript, Node Wasm and Wasmtime for fresh cells, retained
captures, alias snapshots, effects, recursive rebinding, callback parameters,
returned callees, known block-result composition, and async catch/throw/await
interactions. Malformed IR tests check
result metadata and signature consistency; erased TypeScript annotations produce
identical bytes. Deliberately routing a factory call through the Number-return
physical type causes the behavioral test to fail with a Wasm type error.

General function host transport, callbacks taking callbacks, factories returning
factories, nonnumeric leaf results, async factory/leaf bodies and Promise callbacks
remain outside this slice. This feature initially admits concise runtime factories;
known local block-result specialization is a separate source path.
