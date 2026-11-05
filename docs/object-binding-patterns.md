# Object binding declarations

Local `const` and `let` declarations admit object binding patterns over
source-proven managed objects. This supports shorthand fields, renamed fields,
and identifier or String-literal property names, including literal-computed
String keys and nested object or array patterns. Identifier leaves may have lazy
defaults. The same declaration path applies in ordinary blocks, helpers,
async activations and traditional `for` initializers.

## Evaluation and representation

The original initializer executes once before any field is selected. Each
property is read in source order and its lexical binding is initialized before
the next property or declarator. Empty object patterns still evaluate their
initializer. Missing own properties produce Undefined through the existing
managed object operation.

Nested fields are selected once and retain their original receiver while
their leaves finish in source order. A default inside a nested pattern completes
before the next outer property is read. Replacing an outer property or rebinding
the original source does not redirect a nested selection already in progress. Empty nested
patterns still select and validate their receiver. Each nested object pattern
requires an all-object source proof; missing, nullable, array or unknown
alternatives do not satisfy that proof. Nested array patterns use their own
all-array proof and independent iterator state described in
[array bindings](array-binding-patterns.md).

A default runs only when that property's selected value is Undefined. Null,
false, zero, NaN and empty Strings keep their values. The check uses intrinsic
Undefined even if a source binding shadows the name `undefined`. Each default
completes before its leaf is published and before the next property is read, so
an earlier default can mutate a later field. Rebinding the source variable does
not change the saved RHS object used for later selections.

Defaults use the ordinary expression/source cursor, including source block
calls and supported awaits. Skipped defaults have no runtime effects; an abrupt
completion prevents later property reads and bindings. Earlier leaves are
visible in later defaults. Self references and references to later leaves keep
the current unsupported-TDZ diagnostic, even when a source default would be
skipped at runtime.

Selected values retain the managed carrier used by ordinary property reads.
String and container origin facts follow the property projection, including
alias writes and unknown alternatives. Those facts allow existing String,
object and array consumers when the source proves their receiver kind; a type
annotation does not supply that proof. Mutable bindings and captured bindings
use the same slots and cells as identifier declarations.

Default provenance distinguishes a source-proven Undefined value from an unknown
value. It removes only the proven Undefined alternative and includes the
fallback's alternatives. A missing property on a closed source record can
therefore use a String or container fallback with the corresponding consumers.
Explicit intrinsic `undefined` and `void` results carry the same narrow fact.
Unknown spread fields, nullable or other primitive alternatives, alias writes
and unresolved values retain their conservative proof boundaries. A default does
not erase those alternatives, and a shadowing binding named `undefined` follows
its actual source values.

The source-local budget counts each bound identifier across the complete pattern;
empty patterns do not consume locals. Pattern processing is iterative, with at
most 128 nested binding-container levels, and evaluation uses the existing source
cursor. This slice adds no runtime imports, public ABI, IR operation or native
recursion proportional to pattern size.

## Admission limits

This slice covers declarations with identifier leaves, their optional defaults
and nested object or array patterns. Defaults on whole nested patterns, rest
properties, destructuring assignment and parameter patterns remain separate
work. [Array declarations](array-binding-patterns.md) have their own source-proven
array admission. Dynamic property names and primitive
RHS coercion are also outside this slice.

Ordinary objects retain the existing intrinsic-prototype-key guard. Proven
null-prototype dictionaries may bind keys such as `constructor` and `__proto__`.
That proof is checked separately for every nested object receiver.
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
completion across suspension, including lazy default branches and original-RHS
snapshots. These tests join the existing Cargo suites and
impact selection; local receipts and hosted gates are recorded on the PRs.

The semantic reference is ECMAScript's
[PropertyBindingInitialization and KeyedBindingInitialization](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-destructuring-binding-patterns).
