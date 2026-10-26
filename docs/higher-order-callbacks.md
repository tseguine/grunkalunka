# Runtime-selected functions with callback parameters

Synchronous runtime function values accept Number arguments and numeric-leaf
callbacks, and return Number. [Concise runtime factories](runtime-function-results.md)
also return numeric-leaf function pairs. The Number, primitive-return and
primitive async entry profiles share this support:

```js
export function run(a, b) {
  const apply = (fn, x) => fn(x);
  const twice = (fn, x) => fn(fn(x));
  const h = a ? apply : twice;
  return h(x => x + b, a);
}
```

The compiler derives each callback parameter's fixed Number-argument arity from
source calls, aliases and known forwarding, as for specialized local calls.
Logical signatures describe every parameter in order. Functions selected into
one value or assigned into an initialized function slot must have the same full
signature: `(callback(1), Number)` differs from `(Number, Number)`, and callback
arities must agree. Erased TypeScript annotations establish no runtime kind.
Undemanded parameters retain Number signatures when lifted; passing an otherwise
unused callback to a runtime parameter is not inferred from its annotation.

The private `Closure(signature)` type holds a module-local signature ID. Numeric
leaf IDs retain their arity, 0 through 1,000; higher IDs intern parameter lists
containing Number and numeric-leaf Closure types plus a result type. The verifier
checks registry shapes, constructor targets, branch results, source cells and ordered arguments.
The host ABI stays the same. Number-result functions retain the private physical
`(environment, argument frame) -> f64` ABI; factories add a paired i32 result. Each logical argument occupies eight frame
bytes: an f64 Number or two i32 function fields. Captured callback parameters receive ordinary shared typed
cells, so rebinding a parameter is visible to closures capturing that parameter.

The callee and each earlier argument snapshot both code and environment before
later arguments execute. Captured contents remain shared. Rebinding the caller's
callback variable does not replace a saved argument, and rebinding a helper's
parameter does not replace its caller's binding. Synchronous callbacks called by
the async entry preserve these snapshots across intervening awaits.

Tests compare original JavaScript with Node Wasm and Wasmtime for selected and
mutable helpers, different callback environments, recursion, forwarding, callback
parameter mutation and capture, and argument ordering. Async tests also compare
native Promise outcomes and job behavior. Wide cases cover 1,000 logical helper
arguments and 1,000 Number arguments to the supplied callback. Signature mutations
exercise the verifier, and erased annotation tests require identical Wasm bytes.
Source, expansion, lifting, native-stack, arena and fuel bounds remain applicable.
The shared 128-region budget crosses runtime callback argument lowering too; a
nested-caller/returned-callback test rejects the combined deep case on a 2 MiB stack.

This slice does not admit callbacks taking callbacks, factories returning
factories, primitive or heap callback arguments/results, async callback bodies,
Promise callbacks, function host parameters/results, or general JS function
reflection and coercion. It does not complete first-class JavaScript functions.
