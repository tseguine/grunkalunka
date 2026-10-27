# Runtime-selected factory results

Synchronous concise or complete block factories selected at runtime return function
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

Block creators may return pairs from branches, nested blocks, loops and early
returns. Every structural path must return the same full function signature;
normal completion, bare returns and mixed primitive/function returns stay rejected.
The existing owned-return query excludes nested arrows' returns, while the typed
`InlineCall` boundary retains the creator's early returns without leaving its caller.
Local initializer aliases and captured mutable cells retain their lexical identity.

Logical signatures now include ordered parameter types and a result type. Numeric
leaf IDs 0 through 1,000 keep their prior meaning: Number inputs and Number result.
Richer IDs describe callback parameters or a function result. Pure kind queries
lazily intern compilation metadata without evaluating source effects; scratch
checking shares that registry but cannot publish its scratch IR. Result references
in the registry are acyclic. Runtime bodies return Number, General or a
Closure type. Every returned function accepts Number parameters or finite
Number/General-result callbacks, which may themselves accept further callbacks. It can return
Undefined, Null, Boolean or Number, including mixed primitive results from block
bodies, or another function satisfying the same contract. Selected or assigned
factory values must agree on the complete signature at every returned layer.
Source-known constructor groups now follow complete factory returns even when no
source call establishes the nested result signature. Corresponding primitive leaves
join through the existing General result interface; unused definitions, returned
aliases and complete block bodies receive the same result-kind checks as called
factories. This bounded query executes no source effects and preserves the
eight-frame signature limit and function/primitive signature boundaries.

Each IR function records its result type. The verifier checks it against the
entry profile, lifted-body contract, constructor signature and call result.
Number calls retain `(environment, arguments) -> f64`; factory calls use the
additional private `(environment, arguments) -> (i32, i32)` table call type.
Primitive leaf calls use the existing `(environment, arguments) -> (i32, f64)`
type. A shared iterative signature check guards lifting and IR results at every
returned layer. References strictly descend through earlier interned signature IDs;
cycles and dangling IDs stay rejected. Numeric-leaf parameter IDs remain
canonical, 0 through 1,000; richer callback IDs describe acyclic scalar-result
parameter graphs. Returned
callbacks and further closures retain captured callback cells and pair snapshots;
ordered arguments can suspend in the caller before their parameter cells are bound.
Runtime creator and returned-function calls evaluate surplus ordinary arguments
before invocation, preserving the earlier callee pair and declared-input snapshots.
Only declared inputs are passed; the extras retain their effects, validation and
async jobs. Missing inputs and lists above 1,000 supplied arguments remain guarded.
For example, `make(a)(b)()` can retain both creator cells through two returned pairs.
Frames and typed cells retain their existing layouts. The host ABI is unchanged.

Eight active static/runtime function-result lowering frames bound native cursor
reentry; a 2 MiB stack probe accepts eight nested runtime-result calls and rejects
the ninth. Generated-region, source-depth, 256-expansion, lifting, arena and fuel
bounds remain. Wide tests cover 1,000 creator arguments and 1,000 returned-leaf
arguments without enlarging physical Wasm parameter lists. The two 1,000-argument
limits are executed separately; combining both in one call can exceed the current
budget of 10,000 units of Wasmtime fuel, which remains enforced.

Tests compare native JavaScript, Node Wasm and Wasmtime for fresh cells, retained
captures, alias snapshots, effects, recursive rebinding, callback parameters,
returned callees, known block-result composition, and async catch/throw/await
interactions. Malformed IR tests check
result metadata and signature consistency; erased TypeScript annotations produce
identical bytes. Deliberately routing a factory call through the Number-return
physical type causes the behavioral test to fail with a Wasm type error.

General function host transport, unrepresented leaf
results, async factory/leaf bodies and Promise callbacks
remain outside this slice. Known local block-result specialization and runtime
block lifting share the existing typed completion machinery.
