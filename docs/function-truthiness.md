# Function values in Boolean contexts

Represented synchronous function values are truthy in `if`, ternary tests,
negation, loop conditions and Boolean-position `&&`/`||`. This includes existing
numeric-result and higher-order signatures, mutable bindings, aliases, assignments,
constructors, functions returned by known local calls, and admitted runtime-selected
concise factories and their numeric-leaf results.

```js
export function run(a, b) {
  let called = 0;
  const f = () => ++called;
  if (f) return called;
  return -1;
}
```

This returns zero: testing `f` does not invoke its body. Computing the tested
function still executes its source expression once, in order. A creator call runs
its arguments and creator body; assignments and comma prefixes retain their effects.
The body of the resulting function is not invoked to determine truthiness.
Short-circuit branches preserve skipped effects and awaits. Captured function
bindings and returned higher-order pairs retain their existing signatures.

Predicate lowering evaluates the function through the existing typed value cursor,
then emits `Boolean(true)`. Its code/environment pair remains internal; no primitive
Function tag, numeric conversion, IR operation or runtime ABI is introduced.
`!f` produces a Boolean in the primitive/async profiles. The numeric profile uses
its existing explicit bridge, for example `+!f`, to obtain Number zero.

The rule applies only when the underlying function value expression is admitted.
It does not extend function-valued logical results, numeric coercion, equality,
reflection, host function transport, function-valued awaits, unsupported function
result kinds or async callback bodies. A Boolean context such as `if(f && g)` can
normalize both selected operands to predicates without transporting a function as
a primitive or claiming general logical-result support.

Existing source, lifting, cursor, expansion, arena and fuel bounds remain in force.
Predicate-to-function crossings carry the caller's generated region context; nested
call-result regions share the 128-region limit. A 2 MiB native-stack test checks
nested constructor rejection and both sides of the aggregate region boundary.

Runtime tests compare original JavaScript, Node WebAssembly and Wasmtime for
truthiness, dormant bodies, effect order, negation, loops and short-circuiting.
Async tests also check creator arguments that await, skipped rejected awaits,
mutable pairs across jobs, captured catch bindings, and runtime factory predicates
with await and source throw/catch. Boundary tests retain the numeric/primitive
distinction and rejection of unsupported function operations.
