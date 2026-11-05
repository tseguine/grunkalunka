# Array iteration

Synchronous `for...of` admits one local `const` or `let` declaration over a
source-proven managed array. Its binding may be an identifier or a supported
object/array pattern, including mixed nesting and identifier-leaf defaults.
The loop works in ordinary source functions, helpers and supported async
activations. The iterable expression is evaluated
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

Each iteration initializes new lexical bindings. Captured `const` and `let`
leaves receive fresh cells; escaped closures keep the cells from their own
iterations, including later writes to an escaped `let`. A source binding that
shadows the RHS name is not initialized during RHS evaluation and retains the
existing controlled TDZ rejection. The loop bindings leave scope when the loop
finishes.

A destructuring head consumes the selected item through the same ordered pattern
cursor as an ordinary declaration. Nested receivers remain saved, defaults run
only for Undefined, and earlier leaves are initialized before later defaults.
The body begins after the entire head finishes. A supported await in a default
retains the selected item and nested iterator states; an abrupt default prevents
the body from running. Each nested array pattern has its own exhaustion state,
separate from the outer `for...of` cursor and from other patterns over the same
array. See [object bindings](object-binding-patterns.md) and
[array bindings](array-binding-patterns.md) for the shared pattern contract.

## Representation and admission

The selected item uses the managed value carrier. Equality, truthiness, managed
storage, identity transport and existing generic helper/capture positions work
without assuming that the item is a Number or a particular reference kind.
The original-source analysis also collects every possible element origin for
each reached array allocation. When those alternatives prove a String, record
or array, the binding can use the existing checked String operations, record
projections or nested array iteration. The carrier remains a managed value;
this proof does not add numeric unboxing. Type annotations do not supply proof.

The proof follows aliases, helper arguments and returns, captures, conditional
receivers and subsequent assignments to a mutable iteration binding. It unions
literal elements with observed index writes. Same-kind replacement of an
existing element can retain a proof; named properties do not contribute array
elements. An unmodified empty array contributes no yielded values, including
when it is one branch of a conditional receiver.

Holes, unknown or incompatible values, new indices, any length write, `pop`
and dynamic-key writes prevent a narrow element proof. This is deliberately
conservative: even contiguous append or safe truncation can lose a proof, while
generic equality, truthiness and identity transport remain available. Mutation
facts are accumulated across the whole source, including helpers and captures;
an earlier apparent proof cannot discard a later unknown alternative. Arrays
created by runtime operations such as `Object.keys` remain iterable without
proving the kind of their elements.

Analysis first reaches a fixed point, then marks unresolved value rules as
unknown and propagates those facts again. This prevents an unresolved cycle
from disappearing beside a known alternative. Proven records and nested arrays
can also use existing callable-member operations, subject to their established
signature and receiver checks; the loop does not introduce a new call ABI.

The receiver must be an array constructed within the admitted managed source
surface. That surface cannot install a custom iterator, change iterator or array
prototypes, install accessors or import foreign arrays. Existing own-array
length and element operations therefore preserve the intrinsic iterator's
observable behavior without allocating a runtime iterator object. Extending
this scope will require preserving user iteration hooks and iterator closing.

Each destructured item must have the object or array kind required by its head,
and every nested receiver needs its own positive source proof. Missing, nullable,
mixed-kind and unresolved alternatives retain their conservative rejection.
Defaults on a whole pattern, rest bindings and dynamic object keys remain outside
the shared pattern admission.

This slice rejects `for await...of`, `var`, assignment targets,
Strings, unproven iterable values and labels. Synchronous source
try/catch/finally retain their existing admission boundary.

The index slot and generated header regions do not count as source locals or
source statements. Every head leaf counts once against the existing
128-local limit; empty patterns consume no locals. Mixed patterns retain the
128-level binding-container limit. All source traversal remains on the heap-owned
cursor; the existing 64-level source statement nesting limit still applies. No new IR
operation, runtime import or application ABI is introduced.

## Executable evidence

`crates/grunkalunka-runtime/tests/managed-iteration.rs` compares native
JavaScript, Node-Wasm and Wasmtime for JS and erased TS. It covers sparse/live
arrays, original receiver identity, helpers, fresh captures, nested control,
scope and rejection boundaries, and source limits on a 2 MiB native stack.
Element-proof cases exercise String operations, record fields, nested arrays
and conservative mutation boundaries, including aliases and helper writes.
`crates/grunkalunka-runtime/tests/async-suspension/iteration.rs` checks saved
state, promise timing, rejection and finalizer behavior through the existing
async suite. These are ordinary Cargo tests; feature PRs record actual local
and hosted verification results separately.

The semantic references are ECMAScript's
[for-in/of evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-for-in-and-for-of-statements)
and [array iterator next](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-%arrayiteratorprototype%.next).
