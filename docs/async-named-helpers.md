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

Helper inputs retain the existing module-helper Number contract. Primitive and
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
rebinding, optional calls, module globals and broader helper input domains remain
outside this unit. Native JS, Node-Wasm and Wasmtime tests cover declaration order,
values and reactions, forwarding, recursion, concurrent cells, evaluation order,
erased types and source/resource boundaries. Notification fixtures also compare
native rejection checkpoints and exact guest job counts.
