# 004 — tsrs type query

Isolated research probe. Keep this package outside the main Cargo workspace; it
does not define a production frontend or stable cross-frontend interface.

## Question and result

Can a type query from a pinned tsrs checker be associated with an expression
selected from Oxc, and what additional mapping is needed compared with tsrs alone?

For these fixtures, yes: the Oxc byte span selects a tsrs binary-expression node
with the same end offset, and tsrs returns `number`. A separate tsrs-only AST
traversal finds the same fixture expressions and returns the same type and byte
range.
The probe also confirms that tsrs reports one semantic error for the deliberately
mismatched TS annotation, while the expression itself still has type `number`.
Both parsers reject the TS annotation syntax when the same fixture is treated as
JavaScript.

This is only a source-span association experiment. Matching ranges do not establish
shared AST nodes, symbol identities, equivalent parsing generally, or runtime
behavior. The tsrs-only expression selector is fixture-specific; it traverses the
tsrs AST and matches the expected expression text.

## Replay

From the repository root, after the locked dependencies have been fetched:

```sh
cargo fmt --manifest-path spikes/004-tsrs-type-query/Cargo.toml -- --check
cargo check --manifest-path spikes/004-tsrs-type-query/Cargo.toml --locked --offline
cargo run --manifest-path spikes/004-tsrs-type-query/Cargo.toml --locked --offline
```

`Cargo.lock` pins dependencies, including tsrs revision
`736a0e189939278c9878e5a3454f8126f55081fc` and Oxc 0.155.0. Dependency fetching
is a separate, online setup step; the replay commands above run offline. The
package has its own Cargo workspace and target directory, avoiding changes to the
main workspace or its build outputs.

Successful run:

```text
valid.js: tsrs syntax diagnostics=0, semantic diagnostics=0, Oxc+tsrs type=Some("number") span=(42, 50), tsrs-only type=Some("number") span=Some((42, 50))
valid.ts: tsrs syntax diagnostics=0, semantic diagnostics=0, Oxc+tsrs type=Some("number") span=(58, 66), tsrs-only type=Some("number") span=Some((58, 66))
invalid.ts: tsrs syntax diagnostics=0, semantic diagnostics=1, Oxc+tsrs type=Some("number") span=(32, 38), tsrs-only type=Some("number") span=Some((32, 38))
invalid.js: Oxc syntax diagnostics=2, tsrs syntax diagnostics=1, semantic diagnostics=0, tsrs-only expression type=Some("number"), byte span=Some((77, 83)); TS annotation rejected
```

## Method and limits

- Oxc parses the valid JS/TS fixtures, identifies the target binary-expression
  spans, and passes those offsets to tsrs. The probe uses tsrs's
  `get_node_at_position`, then walks ancestors to the binary node with the same
  end offset.
- The tsrs-only route walks tsrs's own AST and locates the fixture's expected
  binary-expression text. It does not use Oxc spans.
- The pinned tsrs `SourcePos` is a UTF-8 byte offset. Oxc spans are also byte
  offsets, so the original byte ranges are passed directly. An initial UTF-16
  conversion failed; it was removed after inspecting the pinned API definition.
- Syntactic and semantic diagnostics come from separate tsrs APIs. The invalid
  JavaScript assertion checks syntax diagnostics; the invalid TypeScript assertion
  checks an additional semantic diagnostic.
- Compiler options disable library loading and module resolution. The probe does
  not evaluate JavaScript or validate runtime values; a static type result is not
  proof of JavaScript runtime behavior.
- It does not compare Oxc and tsrs symbol IDs, declaration merging, value/type
  namespace behavior, generated nodes, transforms, or module graphs. The exact-span
  result is fixture evidence, not a general source-identity algorithm.
- The dependency's upstream licensing and attribution obligations remain relevant
  before any production adoption.

## Recommendation

Keep the bridge as an explicit adapter concern if subsequent experiments need both
frontends: this probe demonstrates a narrow span-based handoff, not shared identity.
Before adopting it, test symbol/declaration association and type/value namespace
cases, then compare behavior on broader fixtures. Do not merge the package into the
main workspace or treat its type results as a JS behavior guarantee.
