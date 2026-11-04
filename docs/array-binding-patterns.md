# Array binding declarations

Local `const` and `let` declarations admit flat array binding patterns over
source-proven managed arrays. Identifier leaves, elisions and empty patterns
work in blocks, helpers, async activations and traditional `for` initializers.

## Evaluation and representation

The initializer executes once before any selected value is published. Each
identifier receives its corresponding array position in source order. Sparse
holes and positions past the end produce Undefined. Empty and all-elision
patterns still evaluate their initializer. A later declarator can observe the
earlier bindings and mutate the same array without changing a saved selection.

This admission uses the intrinsic iterator of an array created within the
supported source surface. That surface cannot install a custom iterator, modify
the array prototype, add accessors or import a foreign array. With identifier
leaves and no defaults, there are no source effects between iterator steps, so
ordered own-index reads preserve the intrinsic iterator's observations. Elisions
do not need generated reads. This is not general iterable destructuring.

Every leaf uses the managed value carrier, including selected Numbers. Existing
equality, truthiness, storage and managed consumers apply; arithmetic does not
acquire unchecked numeric unboxing. Fixed-position source facts retain proven
String or container operations through helpers and captures. Alias writes,
truncation, `pop`, missing values and unknown alternatives remain conservative.
Type annotations do not prove either the RHS or an element's runtime kind.

Mutable leaves use the existing lexical slots and captured cells. Traditional
`for` headers and declarations in loop bodies retain their existing iteration
cell behavior. The 128-source-local budget counts identifiers, not elisions;
pattern preparation and lowering are iterative. No runtime imports, public ABI
or IR operations are added.

## Admission limits

Defaults, nested patterns, rest elements, destructuring assignment and parameter
patterns remain separate work. Strings, ordinary objects with numeric fields,
unknown parameters and nullable array alternatives do not satisfy the array
proof. The intrinsic-iterator boundary also excludes user-defined iteration and
observable iterator closing.

Lazy defaults will require reading live length between source effects and
retaining a finished state once iteration is exhausted. Independent indexed
reads are insufficient for that extension: growing an already exhausted array
must not restart its iterator.

## Executable evidence

`crates/grunkalunka-runtime/tests/managed-array-bindings.rs` compares native
JavaScript, Node-Wasm and Wasmtime for JS and erased TS. It covers sparse
positions, evaluation order, references, helpers, captures, traditional loop
headers, rejection boundaries and 2 MiB native-stack local-budget probes.
`crates/grunkalunka-runtime/tests/async-suspension/array_bindings.rs` covers
saved references, initializer suspension and abrupt completion in the existing
async suite. Local receipts and hosted results are recorded on the feature PRs.

The semantic references are ECMAScript's
[IteratorBindingInitialization](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-destructuring-binding-patterns-runtime-semantics-iteratorbindinginitialization)
and [CreateArrayIterator](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-createarrayiterator).
