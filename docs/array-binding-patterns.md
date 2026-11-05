# Array binding declarations

Local `const` and `let` declarations admit array binding patterns over
source-proven managed arrays. Identifier leaves with optional lazy defaults,
elisions, empty patterns and nested object/array patterns work in blocks,
helpers, async activations and traditional `for` initializers. Objects can also
contain nested array patterns through the same binding cursor.

## Evaluation and representation

The initializer executes once before any selected value is published. Each
identifier receives its corresponding array position in source order. Sparse
holes and positions past the end produce Undefined. Empty and all-elision
patterns still evaluate their initializer. A default completes before its leaf
is published and before the next position is selected. Earlier leaves and
declarators are visible to later defaults; a saved selection keeps its identity
when a later expression mutates the source array.

Every nested selection retains its original object or array and finishes before
the next outer selection. Each nested receiver needs a source proof of the
required container kind, even for an empty pattern. Rebinding a source or
replacing a parent field does not redirect an already-selected inner receiver.

Defaults run only for Undefined, including sparse holes and exhausted steps.
Null, false, zero, NaN and empty Strings preserve their value. A source binding
named `undefined` does not change the intrinsic comparison. Skipped defaults
have no effects; a throwing or rejected default stops the remaining declaration.
Self references and references to later leaves retain the controlled unsupported
TDZ diagnostic even when that default would be skipped at runtime.

Before each active step, the iterator observes the original array's current
length. Growth is visible until a step reaches the end; after that, iteration
stays finished even if a later default grows the array. In particular, an empty
array is already exhausted before its first default runs, while an array with
one hole is still active after yielding that hole's Undefined value. Truncation
and regrowth before the next step differ from regrowth after exhaustion.
Source rebinding does not replace the saved array. The receiver, selected value
and activity state survive supported awaits in defaults. Every nested array
owns a separate iterator state, even when two patterns consume the same array
object. Exhausting an inner array does not finish its parent or a later sibling.
A default inside a descendant pattern can change an ancestor's live length;
the ancestor observes that change at its next step while retaining its own
exhaustion state.

This admission uses the intrinsic iterator of an array created within the
supported source surface. That surface cannot install a custom iterator, modify
the array prototype, add accessors or import a foreign array. Patterns without
defaults anywhere in their nested subtree have no source effects between steps
and keep the direct own-index read path. Arrays containing any descendant default
retain an explicit activity value and guarded live-length checks. Elisions do
not execute user hooks in this surface, so a check at the next bound index also
observes exhaustion within a run of holes.
This is not general iterable destructuring.

Every leaf uses the managed value carrier, including selected Numbers. Existing
equality, truthiness, storage and managed consumers apply; arithmetic does not
acquire unchecked numeric unboxing. Fixed-position source facts retain proven
String or container operations through helpers and captures. Alias writes,
truncation, `pop`, missing values and unknown alternatives remain conservative.
Type annotations do not prove either the RHS or an element's runtime kind.
Default origins are unioned with selected-element origins; a default cannot
erase unknown or possibly missing alternatives. A String fallback on a missing
element alone therefore does not justify a String-only consumer.

Mutable leaves use the existing lexical slots and captured cells. Traditional
`for` headers and declarations in loop bodies retain their existing iteration
cell behavior. The 128-source-local budget counts identifiers, not elisions;
pattern preparation and lowering are iterative. Object and array nesting share
one 128-container-level limit. No runtime imports, public ABI or IR operations
are added.

## Admission limits

Defaults on whole nested patterns, rest elements, destructuring assignment and
parameter patterns remain separate work. Strings, ordinary objects with numeric
fields, unknown parameters and nullable array alternatives do not satisfy the
array proof. The intrinsic-iterator boundary also excludes user-defined
iteration and observable iterator closing.

## Executable evidence

`crates/grunkalunka-runtime/tests/managed-array-bindings.rs` compares native
JavaScript, Node-Wasm and Wasmtime for JS and erased TS. It covers sparse
positions, lazy defaults, live growth, persistent exhaustion, evaluation order,
references, helpers, captures, traditional loop headers, rejection boundaries
and 2 MiB native-stack local-budget probes.
`crates/grunkalunka-runtime/tests/async-suspension/array_bindings.rs` covers
saved references, initializer/default suspension, iterator state and abrupt
completion in the existing async suite. Local receipts and hosted results are
recorded on the feature PRs.

The semantic references are ECMAScript's
[IteratorBindingInitialization](https://tc39.es/ecma262/multipage/syntax-directed-operations.html#sec-runtime-semantics-iteratorbindinginitialization)
and [CreateArrayIterator](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-createarrayiterator).
