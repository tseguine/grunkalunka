# Callback arguments to known local functions

Known synchronous local arrows and factories accept Number arguments or finite
scalar-result function values, including callbacks taking callbacks. The Number, primitive-return and primitive async
entry profiles share this source lowering. Constructors, runtime selections,
aliases and initialized mutable function bindings can supply a callback:

```js
export function run(a,b) {
  const apply = (fn,x) => fn(x);
  let f = x => x+a;
  return apply(f, (f = x => x-b, a)) + f(a);
}
```

Arguments finish left-to-right before the helper's parameter bindings replace
the caller's. Function arguments snapshot both code and environment fields;
capture contents remain shared cells. An argument may suspend or mutate the
original binding after an earlier argument saved its pair. Callback parameters
are fresh mutable slots, so assigning one does not replace the caller's binding.
Captured callback parameters retain their cells after a factory returns:

```js
const make = fn => x => fn(x);
const f = make(x => x+a);
```

The compiler derives a callback parameter's ordered parameter and result kinds
from calls, constructors, local aliases and forwarding to source-known local arrows. It follows original semantic
SymbolIds through conditional/comma/plain-assignment values and respects shadows.
Conflicting arities and arities above 1,000 are rejected. This is conservative
static demand analysis, not general JS type inference. Parameter names and erased
TypeScript annotations establish no runtime kind. A parameter not demanded as a
callback retains ordinary scalar checking, or accepts a function input when its
actual specialization can handle that input without primitive conversion.

Unused bodies still undergo disposable definition checking. Callback inputs there
carry a type only, with no runtime target, and that scratch IR is never published.
Every actual argument constructs or loads a complete pair; every actual body is
specialized with those initialized typed slots and passes the module verifier.
Call-result storage planning distinguishes Number-result callbacks from General
results (Undefined, Null, Boolean or Number), including intermediate locals and
forwarding functions. These facts cannot establish that a scalar value is callable.

Specialized helper parameters use existing `Closure(signature)` slots, initialization,
stores and captured cells. Actual callback calls use the existing private table
and environment/typed-argument frames. No new IR operation, host signature,
dependency or primitive tag is introduced. Logical lists still admit at most
1,000 parameters and arguments; Number source functions keep their prior ABI.

Source locals/statements count once, and constructor/call expansion budgets remain
separate. Callback-argument construction requires fewer than eight already active
function-value cursors, because body checking can reenter the scalar evaluator.
A 2 MiB stack probe accepts seven nested callback wrappers and rejects eight.
The existing 16-cursor comma-prefix bound, eight-frame lifting bound, source
depth/local/statement limits, 256 expansion/body limits, fuel and arena limits
remain additional restrictions.

Tests compare original JS with Node Wasm and Wasmtime for callback snapshots,
forwarding, aliases, shared writes, factories, lexical shadows, 1,000-argument lists,
async jobs and erased annotations. Boundary tests cover conflicting/invalid inputs,
unused invalid bodies, runtime signature exclusions, nesting and source budgets.

[Runtime-selected higher-order functions accept scalar-result callbacks](higher-order-callbacks.md),
with module-interned ordered parameter kinds rather than only an arity. Compatible
selected targets and initialized mutable functions preserve callback snapshots,
shared capture cells and the fixed two-pointer physical ABI. General primitive
results use the existing tag/payload call result. Ordinary General/heap parameters,
heap or function callback results, async callback bodies and Promise callbacks
remain separate gaps. This does not complete general first-class functions.

Concise known calls now also forward existing function pairs, including immediate
invocation and parameter replacement; see [local function results](local-function-results.md).
Known local [block bodies](block-function-results.md) also return compatible pairs;
runtime factories return finite function pairs via [typed indirect calls](runtime-function-results.md).
General recursive result signatures remain separate work.
