# Object binding declarations

Local `const` and `let` declarations admit flat object binding patterns over
source-proven managed objects. This supports shorthand fields, renamed fields,
and identifier or String-literal property names, including literal-computed
String keys. The same declaration path applies in ordinary blocks, helpers,
async activations and traditional `for` initializers.

## Evaluation and representation

The original initializer executes once before any field is selected. Each
property is read in source order and its lexical binding is initialized before
the next property or declarator. Empty object patterns still evaluate their
initializer. Missing own properties produce Undefined through the existing
managed object operation.

Selected values retain the managed carrier used by ordinary property reads.
String and container origin facts follow the property projection, including
alias writes and unknown alternatives. Those facts allow existing String,
object and array consumers when the source proves their receiver kind; a type
annotation does not supply that proof. Mutable bindings and captured bindings
use the same slots and cells as identifier declarations.

The source-local budget counts each bound identifier. Pattern processing is
iterative, and evaluation uses the existing source cursor. This slice adds no
runtime imports, public ABI, IR operation or native recursion proportional to
pattern size.

## Admission limits

This slice covers declarations with identifier leaves. Defaults, nested
patterns, rest properties, array patterns, destructuring assignment and
parameter patterns remain separate work. Dynamic property names and primitive
RHS coercion are also outside this slice.

Ordinary objects retain the existing intrinsic-prototype-key guard. Proven
null-prototype dictionaries may bind keys such as `constructor` and `__proto__`.
This does not introduce general prototype lookup or foreign object access.

A selected Number is still a managed field value. Equality, truthiness, storage
and existing managed consumers apply; this feature does not add unchecked
numeric unboxing. A function value can retain identity without every callable
signature being admissible.

## Executable evidence

`crates/grunkalunka-runtime/tests/managed-bindings.rs` compares native JavaScript,
Node-Wasm and Wasmtime behavior for admitted JS and erased TS source. It also
checks rejection boundaries and local-budget accounting.
`crates/grunkalunka-runtime/tests/async-suspension/binding_patterns.rs` checks
initializer effects, captured bindings, selected references and abrupt
completion across suspension. These tests join the existing Cargo suites and
impact selection; local receipts and hosted gates are recorded on the PRs.

The semantic reference is ECMAScript's
[PropertyBindingInitialization and KeyedBindingInitialization](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-destructuring-binding-patterns).
