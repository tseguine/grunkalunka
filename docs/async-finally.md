# Async finally

The async profile admits try/finally and try/catch/finally in roots and private
async arrows, including known/runtime calls and captured closures.

```js
export async function run(a,b) {
  const p = Promise.resolve(a);
  try {
    return p;
  } finally {
    await b;
  }
}
```

The return expression is evaluated once and saved before cleanup. The finalizer
runs before async return adopts p. A normal finalizer preserves a pending normal,
return, throw, break or continue completion; an abrupt finalizer replaces it.
Break/continue retain the nearest lexical loop target. Nested function returns
belong to their own activation and do not exit a caller's protected scope.

A finalizer can await, catch its own rejection, return or throw a represented
primitive/intrinsic Error. Pending values retain their original bits and identity
while captured cells may change. A rejected awaited operand enters its lexical
handler/cleanup on its reaction job. Async-return rejection remains outside
lexical catches after cleanup. Overriding a returned Promise does not adopt or
handle that Promise; its detached rejection remains observable.

The IR holds one finalizer and typed private captures. The backend currently
copies cleanup continuations for each relevant exit kind; continuation identity
retains the pending kind across suspension. A source await is charged once even
when its cleanup code has several entries. The existing 32-site, 32-job,
4096-state, source-size, local and fuel limits remain. Nested cleanup may reach
the state bound sooner. Host fuel, traps, malformed protocol and exhausted job
budgets remain terminal and do not run source finally.

Differential tests compare native JS, Node-Wasm and Wasmtime results and FIFO
traces. The existing self-resolution Error fixture additionally checks both Wasm
adapters without claiming native self-cycle equivalence. Scope/type tests guard
the return/reason captures, and notifications distinguish raw discarded returns
from later adoption/handling. Five previous finally guard sources now execute
unchanged. General synchronous source exceptions, labels, catch destructuring,
heap/function return values and general Promise/thenable behavior remain open.
