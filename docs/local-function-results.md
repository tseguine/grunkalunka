# Function values returned by known local calls

Known synchronous local calls can return existing function values, captured
function bindings, and compatible-signature conditional, comma or plain-assignment
results.
Their callers may bind, pass on, assign or immediately invoke the returned pair:

```js
export function run(a,b) {
  const identity = fn => fn;
  const choose = (flag,left,right) => flag ? left : right;
  let current = x => x+a;
  const saved = identity(current);
  current = x => x-b;
  return choose(a,saved,current)(b);
}
```

Classification follows original semantic symbols and actual argument kinds without
executing expressions. Scalar actuals explicitly shadow callback information from
other specializations, so `identity(a)` still takes the ordinary Number path.
Read-only callback-demand analysis follows concise and block returned constructors
through aliases and known calls, including constructors assigned to a formal parameter.
Erased TypeScript annotations do not determine a runtime kind.

Actual arguments execute once, left-to-right, in the caller's scope. Fresh formal
bindings are installed only after all arguments finish. The result snapshots the
code/environment pair; captured cell contents remain shared. Conditional and
assignment callees are evaluated before their ordered Number arguments. These
rules also hold when an argument suspends in the primitive async profile.

This reuses existing typed slots, closure cells, the private table and Number-leaf
argument frames. It adds no public ABI, primitive tag, IR operation or dependency;
block calls use the same InlineCall operation with a typed function result.
The older known-arrow factory specialization remains available. Disposable body
checking recognizes function results without publishing its type-only inputs.

Call-result classification and active local result specialization each admit at
most eight nested calls. Generated regions across scalar/function cursor crossings
share a 128-region nesting limit; source wrappers retain their separate depth
budget. Existing cursor, lifting, source-local, expansion, fuel and arena limits
also apply. Small-stack tests check both accepted and rejected boundaries.

Tests compare original JavaScript with Node Wasm and Wasmtime for identity,
selection, forwarding, parameter replacement, capture lifetime, argument effects,
immediate invocation, async jobs, numeric reuse and 1,000-argument leaf calls.
[Block bodies](block-function-results.md) also preserve these pairs through
statements, branches, loops and early returns, including callback identity and
selection under actual argument-kind scopes. Typed inline-call results extend the
existing IR operation; its verifier requires matching complete function signatures.

These finite cases establish this slice. Factories returning factories,
incompatible logical signatures, heap leaf results, async callback bodies and host function transport remain separate
work. Concise primitive-result closures now pass through these existing local
forwarding and function-return paths; see [runtime closures](runtime-closures.md).
[Numeric-leaf higher-order callback signatures](higher-order-callbacks.md)
now pass through concise and block identity, selection and known-factory paths too;
selected and assigned function values must share their complete signatures.

[Runtime-selected concise factories now return numeric-leaf pairs](runtime-function-results.md)
through typed indirect calls, with the same invocation-owned capture lifetime.
