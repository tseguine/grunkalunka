# Numeric conversion of represented async values

The async identity profile accepts numeric-only consumers of represented builtin
Promise identities and selected General primitive/intrinsic Error values.

```js
export async function run(a,b) {
  const p = Promise.resolve(a);
  const coerce = x => +x;
  coerce(p);              // NaN; p is not awaited or handled
  return coerce(a?p:b);   // NaN or the selected primitive's numeric value
}
```

Admitted contexts are unary +, - and ~; subtraction, multiplication, division,
remainder, bitwise and shift operands; existing Math.abs/sqrt/floor/ceil/trunc
first arguments; and the RHS of the corresponding non-additive compounds on
primitive target cells. Each operand executes once in source order. Captured
snapshots survive later mutations, child startup and suspension. Surplus Math
arguments still execute after its first argument is captured. Failed RHS evaluation
does not perform a compound store, while earlier side effects remain observable.

Conversion discriminates the private carrier. A builtin Promise yields NaN;
General values use NumberFromGeneral: undefined and intrinsic Error yield NaN,
null and false yield positive zero, true yields one, and Numbers retain their
bits. This operation never settles, adopts, awaits or handles a Promise and adds
no job. Explicit later awaits retain their existing reaction and handling rules.

The source domain admits known builtin Promise identities without prototype
mutation or custom conversion. ECMAScript's
[ToNumber](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber)
and [OrdinaryToPrimitive](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-ordinarytoprimitive)
algorithms were checked on 2026-10-28. The NaN rule is the inference for this
closed builtin domain, additionally checked against native JS; it is not a rule
for arbitrary objects. Addition retains its guards. Represented relational operands use
[their separate contract](async-promise-relational.md).
Mixed-target compounds/updates, legacy non-identity snapshots, custom coercion,
String/heap/BigInt domains and unadmitted operators such as exponentiation remain
separate work. An explicitly converted Number such as (+p)+b uses ordinary
Number addition.

Six differential groups compare native JS, Node-Wasm and Wasmtime values and FIFO
traces, including mixed known/runtime/callback inputs/results, captured locals,
erased types and unchanged 32-source-await admission. A self-resolution Error
fixture additionally compares both Wasm adapters without native cycle equivalence.
Three notification fixtures distinguish conversion from explicit later handling;
the exact detached-child guard is executed through an owned native checkpoint.
No new IR operation/type, host protocol, kernel record, layout, dependency or
resource budget is introduced.
