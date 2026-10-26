# Runtime local function values

The synchronous Number and primitive-return profiles now support immutable local
function values selected at runtime. Constructors include same-arity conditional
branches, arrows, known factory results and existing local closure aliases.
Parentheses and erased TypeScript wrappers preserve their runtime meaning.
Scalar side effects can precede a constructor in a comma expression. Calls through
the resulting identifier or alias use exact-arity numeric arguments and return a
Number. Leaf arrows may have concise or numeric block bodies.

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

A `Closure(arity)` IR value is an `i32` code index plus an `i32` environment
address. It is separate from the four-primitive General domain and is never
packed into a Number or primitive host tag. A function-valued cell stores the
two fields at offsets 0 and 4 of its existing 16-byte cell. Conditional joins,
initialization and loads preserve both fields as one typed value.

The compiler lifts a leaf body into an internal function. Its first IR operation,
`ClosureEntry`, imports initialized captured-cell addresses into root-owned slots.
`MakeClosure` checks target kind, capture count, initialization, type and mutability.
`ClosureCall` checks function arity and Number arguments. Ordinary `Call` cannot
invoke a lifted body; the exported entry cannot use the closure ABI. Function
values cannot be packed as primitives or awaited in the current protocol.

The private Wasm table uses one physical signature:
`(environment: i32, arguments: i32) -> f64`. A packed argument frame contains the
ordered binary64 snapshots; the body copies its logical Number parameters before
execution. This supports zero through 1,000 numeric parameters without expanding
the physical Wasm signature. An environment contains ordered captured-cell
addresses. Imported pointers remain valid after creator return. Static callbacks
inside lifted bodies have their captures remapped to those imports. Source
helpers retain their original function indices; lifted bodies append afterward.
Only lifted bodies populate table slots; ordinary source-function positions are null.

Cells, environments and argument frames share the private 1 MiB invocation arena.
Frames round up to 16-byte alignment. Allocation checks the size, alignment,
inclusive capacity and memory-growth failure before advancing the cursor. The
memory and table are unexported. The next external invocation resets the cursor,
including after a fuel or allocator trap; internal calls preserve it. Frames are
retained until that boundary, so repeated calls consume the bounded arena.

Source locals/statements still count once. Constructors and indirect calls share
the 256 generated-expansion limit per function; at most 256 lifted bodies are
admitted per module, with capacity rechecked after nested construction. Eight
combined active closure/block lowering frames bound native recursion. A 2 MiB
stack probe accepts six nested block wrappers around the leaf and rejects seven;
source nesting, fuel and arena limits remain additional restrictions.

Tests compare original JS with Node Wasm and Wasmtime, including multiple captures,
aliases, nested runtime/static callbacks, helper indices, argument effects, all
captured slot kinds, 1,000 parameters, erased types, allocator boundaries and
recovery. Malformed IR tests check environment and calling-convention invariants.
Existing 919 rejection sources retain their exact diagnostics and hashes; the
former focused runtime-factory-selection rejection now has a positive execution
case. Canonical corpus counts remain unchanged.

Dynamic function values in the async profile remain a separate unit; rejection
also applies inside a synchronous local arrow of an async entry. Existing static
closure specialization across await remains available. Mutable function bindings,
passing functions as arguments, general function-valued results, nonnumeric leaf
return kinds, function truthiness/coercions, identity and properties, recursive function-value bindings
and escaping per-iteration environments remain open.
The exported Number/primitive/async signatures and dependency set are unchanged.
