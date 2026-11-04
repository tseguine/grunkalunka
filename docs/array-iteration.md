# Array iteration

Synchronous `for...of` admits one local `const` or `let` identifier over a
source-proven managed array. The loop works in ordinary source functions,
helpers and supported async activations. The iterable expression is evaluated
once; later reassignment of its source variable cannot redirect the loop.

## Iteration and control

Each step reads the original array's current length. Appending through admitted
index writes can extend the loop, while truncation or `pop` can shorten it.
Sparse holes yield Undefined. Named properties do not participate. Empty arrays
execute no iterations, but their initializer effects still occur.

The lowering uses the existing While IR and a private numeric index. It advances
that index before publishing the iteration binding and evaluating the source
body, so `continue` cannot repeat an item. Unlabelled `break`, `continue`, nested
loops and returns keep the existing completion behavior. Async throw, catch and
finally use the same completion machinery, including a finalizer that overrides
a pending loop completion. Supported awaits in the RHS or body retain the
saved array, cursor, selected values and captured cells.

Each iteration initializes a new lexical binding. Captured `const` and `let`
bindings receive fresh cells; escaped closures keep the cells from their own
iterations, including later writes to an escaped `let`. A source binding that
shadows the RHS name is not initialized during RHS evaluation and retains the
existing controlled TDZ rejection. The loop binding leaves scope when the loop
finishes.

## Representation and admission

The selected item uses the managed value carrier. Equality, truthiness, managed
storage, identity transport and existing generic helper/capture positions work
without assuming that the item is a Number or a particular reference kind.
This first slice conservatively leaves element origins unknown. String-only
methods, record projections and unchecked numeric unboxing require additional
proof or a separate runtime operation. Type annotations do not supply proof.

The receiver must be an array constructed within the admitted managed source
surface. That surface cannot install a custom iterator, change iterator or array
prototypes, install accessors or import foreign arrays. Existing own-array
length and element operations therefore preserve the intrinsic iterator's
observable behavior without allocating a runtime iterator object. Extending
this scope will require preserving user iteration hooks and iterator closing.

This slice rejects `for await...of`, `var`, assignment targets, destructuring
headers, Strings, unproven iterable values and labels. Synchronous source
try/catch/finally retain their existing admission boundary.

The index slot and generated header regions do not count as source locals or
source statements. The source head identifier counts once against the existing
128-local limit. All source traversal remains on the heap-owned cursor; the
existing 64-level source statement nesting limit still applies. No new IR
operation, runtime import or application ABI is introduced.

## Executable evidence

`crates/grunkalunka-runtime/tests/managed-iteration.rs` compares native
JavaScript, Node-Wasm and Wasmtime for JS and erased TS. It covers sparse/live
arrays, original receiver identity, helpers, fresh captures, nested control,
scope and rejection boundaries, and source limits on a 2 MiB native stack.
`crates/grunkalunka-runtime/tests/async-suspension/iteration.rs` checks saved
state, promise timing, rejection and finalizer behavior through the existing
async suite. These are ordinary Cargo tests; feature PRs record actual local
and hosted verification results separately.

The semantic references are ECMAScript's
[for-in/of evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-for-in-and-for-of-statements)
and [array iterator next](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%arrayiteratorprototype%.next).
