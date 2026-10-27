# Block-bodied local functions returning functions

Known synchronous local arrows can return runtime function values from a block body:

```js
export function run(a, b) {
  const pick = flag => {
    if (flag) return x => x + a;
    return x => x - b;
  };
  const f = pick(a);
  return f(b);
}
```

All structural return paths must produce functions with the same logical signature,
including arity and parameter kinds. Returns inside nested blocks and the existing
`for`, `while`, and `do` loops use the same completion
rules as numeric block arrows. Pretest loops conservatively retain a normal exit,
so a return after the loop is still required. A nested arrow's own returns do not
participate in its creator's return contract.

The returned value is the existing code/environment pair. Arguments finish once,
left to right, before fresh parameter cells are bound. Surplus ordinary arguments
retain their effects and discard their values in caller scope; all declared inputs
are still required and supplied arguments are bounded to 1,000. Returned functions
retain captured lexical cells after their creator returns; different creator calls receive
independent local cells. Returning a mutable function binding snapshots its current
pair while preserving the captured cells shared by that pair. Local initializer
aliases are recognized through original semantic symbols, preserving shadowing.
Actual argument-kind scopes also support block callback identity and selection;
scalar specializations of the same source arrow retain numeric behavior. Read-only
constructor-origin analysis follows block returns when a formal is replaced by a
returned function constructor. A known constructor on another return path also
supplies the Number-leaf callback arity needed for definition checking.
Higher-order function pairs also retain their full signature IDs through block
constructors, identity and selection; equal parameter counts alone are insufficient.
Constructor-derived inference for returned formals remains limited to Number-leaf
callbacks, as in the existing callback-demand model.

`InlineCall` carries Number, primitive or function results, determined by its result
value's IR type. The verifier checks every return's complete type/signature; the
synchronous emitter uses an `i32 i32` block result for functions and `i32 f64`
for primitives. The async state
machine already copies typed values at inline returns, so returned pairs and their
captures also survive suspension in an async caller.

The source statement/local limits and existing eight-frame block/closure lowering
bound remain in force. On the small native stack test, seven simultaneously active
block factory bodies are admitted; an eighth needs a returned closure body frame and
is rejected. A creator call and each emitted returned constructor independently
consume the existing 256-expansion budget.

Function-valued results remain internal. Exported Number/primitive signatures stay
unchanged. [Runtime factories](runtime-function-results.md), including complete
block bodies, additionally return function pairs through typed indirect calls,
including further factories accepting Numbers and finite scalar-result callbacks; ordinary leaf closures
also support [primitive results](runtime-closures.md) from concise bodies and
block returns and Undefined normal completion. This does not add
function-valued host results, function coercion/equality, mixed function/primitive
returns or incompatible logical signatures, async local arrows, optional/spread local calls,
cyclic callback types, or async callback arguments.
TypeScript annotations remain erased.

Focused coverage lives in `grunkalunka-runtime`'s `runtime-closures`, `local-arrows`
and `async-suspension` suites, plus compiler IR mutation tests. The runtime cases
compare native JavaScript with Node WebAssembly and Wasmtime, including signed zero,
NaN, nested early returns, capture lifetime, mutation snapshots and argument effects.
Combined cases return higher-order functions across primitive rejected-await catches.
