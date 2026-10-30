# Primitive results from named module helpers

Primitive and async compilation admit synchronous module helpers returning the
represented Undefined, Null, Boolean and Number kinds. Helpers keep their existing
Number parameters. For example:

```js
function choose(x) { return x ? null : false; }
export async function run(a,b) {
  const saved = choose(a);
  await b;
  return saved;
}
```

Result planning uses original semantic bindings and each body's own returns.
Helper signatures, local storage and callback facts converge together, so forward
references and recursive helper groups share the required result carrier. Nested
function returns do not classify the enclosing body. Erased TypeScript annotations
do not prove a runtime kind. Numeric-only helpers retain Number signatures; the
numeric compilation profile keeps its existing Number-only contract.

Calls reuse the existing General IR value and primitive return lanes. Validation
checks a call's actual callee result and still rejects incompatible result metadata.
There is no new operation, runtime adapter, public ABI or dependency. Arguments,
surplus effects, captures and saved return values retain their existing ordering.
Async callers can carry these results through suspension, closures and catches.

Helpers retain existing source parameter/argument and nesting limits. Missing
inputs, rebinding, optional/spread calls, primitive helper inputs, String/heap,
Promise/function results, bare returns and structural fallthrough remain outside
this unit. Ordinary synchronous helpers still cannot suspend or throw. Well-typed
IR is distinct from executable backend admission: the async emitter independently
rejects control operations in ordinary helper bodies.

Maintained native-JS/Node-Wasm/Wasmtime comparisons cover primitive tags, numeric
bits, local aliases, forwarding, recursion, callbacks, effects, async reactions and
JS/TS byte identity. Source guards include 1,000 parameters, nesting boundaries,
and result planning over a helper chain on the existing 2 MiB native stack budget.
