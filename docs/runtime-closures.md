# Runtime local function values

The Number, primitive-return and primitive async profiles support initialized local
function values selected at runtime. Constructors include same-signature conditional
branches, logical `&&`/`||`/`??` selections, arrows, known factory results and existing
local closure aliases.
Parentheses and erased TypeScript wrappers preserve their runtime meaning.
Scalar side effects can precede a constructor in a comma expression. Calls through
the resulting identifier or alias use an exact logical signature with Number and
numeric-leaf callback arguments. Concise synchronous bodies can return Number,
Undefined, Null or Boolean, retaining the selected kind and payload. Runtime
factories still return numeric-leaf function pairs. Leaf arrows may have concise
or numeric block bodies; primitive-result block bodies remain a gap.

```js
export function run(a, b) {
  const make = x => () => ++x;
  const f = a ? make(a) : make(b);
  const alias = f;
  return f() * 10 + alias();
}
```

For inputs 4 and 2 this returns 56; for 0 and 2 it returns 34. Only the selected
constructor executes. Aliases retain the same code and environment. Captured
variables remain shared cells, including captures of another runtime function
value. Numeric call results can initialize or update existing scalar bindings;
later primitive writes still select General storage where required.

Logical selection requires matching complete function signatures, including callback
parameter kinds and the result kind. A represented function is truthy: `f && g`
evaluates `f` once and selects `g`; `f || g` keeps the captured value of `f` and skips
`g`. Functions are also non-nullish, so `f ?? g` keeps the same left snapshot and
skips the right operand. Evaluating an operand can construct a function, update a
binding or suspend;
testing its truthiness does not invoke its body. Skipped effects, awaits and
rejections stay skipped. Both operands are still checked against the supported
source profile. This also works for higher-order functions and function-returning
factories. Mixed primitive/function joins, function identity
and exporting a function through the primitive host ABI remain unsupported.

Coalescing requires compatible established operand kinds. Primitive binary `??`
also lets a constructor-free selector such as `(x,y)=>x??y` pass preliminary checking
with default numeric formals, then specialize using its actual function arguments.
Full function signatures still must match; TypeScript assertions establish no kinds.

`let` function bindings retain their initializer's logical signature and support
plain `=` assignment from the same constructors. Conditional/chained assignments
return the RHS function snapshot, so another local can retain it. Discarded function values
and assignments may occur in comma prefixes. Calls snapshot both callee fields
before evaluating ordered arguments, including when an argument reassigns the
binding or suspends. For example, with `a=4, b=2` this returns 695:

```js
export function run(a,b) {
  let f = x => a+x;
  const old = f;
  const before = f((f = x => b-x, 3));
  return before*100 + f(3)*10 + old(1);
}
```

Capturing a mutable function binding shares its cell; copying its value into an
alias snapshots the pair. Rebinding an already initialized function can form
self/mutual recursive calls through those cells, bounded by runtime fuel and the
arena. An initializer cannot capture its own unpublished binding. Mixed scalar/
function storage, signature changes, function compound/logical assignments and numeric
updates remain rejected. Assignment results cannot enter primitive/coercion positions.

A `Closure(signature)` IR value is an `i32` code index plus an `i32` environment
address. It is separate from the four-primitive General domain and is never
packed into a Number or primitive host tag. A function-valued cell stores the
two fields at offsets 0 and 4 of its existing 16-byte cell. Conditional joins,
initialization and loads preserve both fields as one typed value. Numeric-leaf
signature IDs equal their arity; richer IDs refer to module-local ordered
parameter lists and a result type. Callback signatures specify a fixed arity of
Number inputs.

The compiler lifts a synchronous body into an internal function. Its first IR
operation, `ClosureEntry`, imports initialized captured-cell addresses into
root-owned slots.
`MakeClosure` checks target kind, capture count, initialization, type and mutability.
`ClosureCall` checks the complete logical signature, argument types and result
type against the function's declared result metadata.
Ordinary `Call` cannot invoke a lifted body; the exported entry cannot use the
closure ABI. Function values cannot be packed as primitives or awaited in the current protocol.

The private Wasm table uses `(environment: i32, arguments: i32) -> f64` for
Number results, `(environment: i32, arguments: i32) -> (i32, f64)` for primitive
results, and `(environment: i32, arguments: i32) -> (i32, i32)` for
[runtime factory results](runtime-function-results.md). A packed argument frame contains the
ordered eight-byte snapshots: binary64 Numbers or two i32 callback fields. The
body copies its typed logical parameters before execution. This supports zero
through 1,000 parameters without expanding the physical Wasm signature. An
environment contains ordered captured-cell addresses. Imported pointers remain valid after creator return. Static callbacks
inside lifted bodies have their captures remapped to those imports. Source
helpers retain their original function indices; lifted bodies append afterward.
In the async profile the dispatcher and protocol entries have different physical
indices, so table entries refer to named lifted bodies rather than numeric IDs.
Only lifted bodies populate table slots; ordinary source-function positions are null.

Cells, environments and argument frames share the private 1 MiB invocation arena.
Frames round up to 16-byte alignment. Allocation checks the size, alignment,
inclusive capacity and memory-growth failure before advancing the cursor. The
memory and table are unexported. The next external invocation resets the cursor,
including after a fuel or allocator trap; internal calls preserve it. Async
activations instead own a fresh instance until settlement or failure and retain
their arena across jobs. Frames consume the bounded arena until that lifetime ends.

Source locals/statements still count once. Constructors and indirect calls share
the 256 generated-expansion limit per function; at most 256 lifted bodies are
admitted per module, with capacity rechecked after nested construction. Eight
combined active closure/block lowering frames bound native recursion. A 2 MiB
stack probe accepts six nested block wrappers around the leaf and rejects seven;
source nesting, fuel and arena limits remain additional restrictions. Crossings
between function constructors and scalar comma-prefix evaluation have a separate
16-active-cursor guard; a 2 MiB stack probe accepts 15 nested assignment prefixes
and rejects 16. These crossings carry the caller's original AST depth.

Tests compare original JS with Node Wasm and Wasmtime, including multiple captures,
aliases, nested runtime/static callbacks, helper indices, argument effects, all
captured slot kinds, 1,000 parameters, erased types, allocator boundaries and
recovery. Malformed IR tests check environment and calling-convention invariants.
Existing 919 rejection sources retain their exact diagnostics and hashes; the
former focused runtime-factory-selection rejection now has a positive execution
case. Canonical corpus counts remain unchanged.

The async frame retains code/environment pairs, scalar snapshots and capture-cell
pointers across suspension. Lifted bodies keep ordinary activation locals, while
only the source entry has resume states. Awaited conditions and factory/call
arguments preserve their original evaluation order. `await f()` can suspend on a
Number result; `await f` remains rejected because functions are not primitives.
Existing static closure specialization across await remains available.
[Known local helpers and factories accept callback arguments](local-callback-arguments.md)
with numeric leaf signatures, including captured callback cells after creator return.
[Runtime-selected functions also accept numeric-leaf callbacks](higher-order-callbacks.md).
Concise runtime factories now return numeric-leaf pairs. Callbacks taking
callbacks, factories returning factories, nonnumeric leaf
return kinds, function coercions, identity and properties, recursive initialization
and escaping per-iteration environments remain open.
The exported Number/primitive/async signatures and dependency set are unchanged.

Represented functions are now truthy in [Boolean contexts](function-truthiness.md).
The source expression executes once while the tested function's body stays dormant;
constructors, assignments, known returned values and admitted higher-order pairs use
the existing predicate path. This adds neither numeric coercion nor function transport.
