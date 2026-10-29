# Named module async helpers

The async identity profile admits plain, unexported module-level async function
declarations alongside the single exported async `run(a,b)` entry. Helpers may
appear before or after their callers and recurse or call one another.

```js
export async function run(a,b) {
  const first = work(a), second = work(b);
  return +(await first) + (+(await second));
}
async function work(x) {
  const read = () => x;
  await x;
  x++;
  return read();
}
```

Each call starts its synchronous prefix immediately, allocates a fresh Promise
identity and owns its activation and captured cells. Arguments execute left to
right and are snapshotted before later arguments or suspension can mutate them.
An argument failure prevents startup; ordinary surplus arguments still execute.
Body awaits, caller awaits and return adoption retain their separate reactions.
Discarding a rejected child does not handle it; a later await or async return
adoption does. Root settlement and child rejection notifications remain separate.

Named helpers reuse the existing private async signature, closure entry, call,
frame and shared job-queue machinery with an empty capture list. Symbol identity
selects the body; erased TypeScript annotations do not select a different body or
coerce arguments. Existing callable aliases, same-signature selections and callback
forwarding can carry these named callables. This does not add general observable
function identity or properties.

Named async helper inputs now admit represented Undefined, Null, Boolean,
Number and Promise identities, including Promise/primitive mixtures. Actual
calls, aliases and known forwarding choose Number, General, Promise or
AsyncValue private parameters; type annotations supply no kind proof. Number-only
inputs keep their existing signatures, including forwarding to synchronous
Number helpers. Mixed inputs use the existing private argument layout and retain
primitive bits, mutable cells and captures across suspension. Primitive and
intrinsic TypeError results, bare return/fallthrough, rejection, lexical catch,
finally and represented Promise return adoption use the existing async profile.
Synchronous Number helpers remain synchronous and can start/discard an async
child. The exported entry keeps two Number inputs and cannot be called recursively
as a helper. Synchronous/numeric compilation entry points still reject async
declarations.

The existing 1,000 source-parameter/argument limit and module-wide 32 await-site
limit remain. Admission does not guarantee that every admitted program fits the
runtime's instruction, frame, arena or job budgets. Maintained tests execute
17- and 32-parameter helpers through the normal adapters. A 1,000-parameter
witness exhausts the default Wasmtime boundary fuel; it executes under an explicit
200,000-instruction probe budget and through the Node adapter. Production fuel
limits are unchanged.

Defaults, rest/destructured parameters, generators, `this`, `arguments`, binding
rebinding, optional calls, module globals and Error/function/String/heap
input domains remain outside this unit. Promise-valued throws now preserve raw
identity through named/local async rejection and lexical catches. Intrinsic
Promise.reject(p) preserves the same raw reason without adoption or handling.
Native JS, Node-Wasm and Wasmtime tests cover declaration order, values and
reactions, forwarding, recursion, concurrent cells, evaluation order,
erased types and source/resource boundaries. Notification fixtures also compare
native rejection checkpoints and exact guest job counts.

Actual primitive arguments propagate through named helper recursion and local
callable forwarding before signatures settle. A widened input does not convert
its value, change body/caller reaction boundaries or widen synchronous helper
inputs. Maintained native JS, Node-Wasm and Wasmtime comparisons include the
exact former null/false exclusions, mixed Number/General frames, concurrent
cells, alias/conditional selection, callback/factory forwarding, recursive
activations, argument failures and erased TypeScript. No IR, runtime/frame ABI,
dependency or resource limit changes are needed.

Named Promise inputs retain their original handle through direct calls, aliases,
selected named/local callables, callback/factory forwarding and recursive helper
activations. Plain parameter replacement can require the existing AsyncValue
carrier; captures and earlier arguments preserve their snapshots. The exact
former f(Promise.resolve(a)) exclusion now executes in JS and TS against native
JS, Node-Wasm and Wasmtime, including reaction ticks. Notification tests show
that passing an unused rejected argument neither handles it nor adopts it, and
that failure in a later argument prevents helper startup. Broader Promise-cell
updates/coercions, thenables and object rejection domains remain separate gaps.
