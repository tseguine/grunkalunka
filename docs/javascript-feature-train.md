# JavaScript feature train

## Direction (2026-11-03)

The maintainer has assigned the compiler/runtime stream to order language
features by practical impact and implement them in that order, with additional
weight for A1 application authoring. A1 is a priority, not the exclusive consumer.
Ship useful feature families in reviewable slices; complete ECMAScript coverage
of each family is not a prerequisite for moving to the next useful slice.

The starting revision is
[`0f34cd7`](https://github.com/tseguine/grunkalunka/commit/0f34cd774c28074048fce90f10e4b2dfc3114d53),
after the source-lowering architecture series through
[#468](https://github.com/tseguine/grunkalunka/pull/468). New features use the
existing owned source cursor, typed Region IR, managed runtime and completion
machinery. Extract shared facts or helpers when a feature needs them; another
architecture-wide migration is not a gate for this train.

## Ordered feature families

| Order | Feature family | Practical payoff and first useful slices |
| --- | --- | --- |
| 1 | Capability calls composed with ordinary functions | Let A1 handlers organize policy in named helpers and captured callbacks while invoking the existing explicit Rust/WIT service. First admit numeric named helpers in `compile_ui_import`, then closures and captured state, preserving the current service ABI. |
| 2 | Runtime collection access and updates | Select headings by runtime position, inspect lists and maintain keyed application state. Start with array `.at(Number)` using the existing sparse-array storage; then expand computed keys and updates as their runtime key semantics become available. |
| 3 | Practical Strings and common operators | Measure and construct headings, status labels and search text. Start with UTF-16 `.length`, character access and String concatenation; add useful conversions and methods in response to source programs. |
| 4 | Everyday bindings and iteration | Make record/list processing natural: object and array binding patterns, defaults and array `for...of`, followed by commonly needed control forms. Reuse the source cursor and existing per-iteration binding machinery. |
| 5 | Synchronous source errors | Let ordinary handlers recover from invalid operations. Extend the existing normal/throw seam into synchronous execution and add catchable failures as runtime operations need them. |
| 6 | Broader calls, receivers and object construction | Support ordinary methods, dynamic `this`, less restrictive callable signatures and reusable model objects. Constructors/classes follow concrete consumers. |
| 7 | Managed asynchronous payloads | Carry records and Strings as Promise settlements, rejection reasons and await results. Build on existing traced suspension and related-activation identity. |
| 8 | General modules and remaining compatibility | Grow explicit application module graphs and live bindings, then less frequently needed builtins and language features. Expand conformance coverage with the implemented surface. |

This is a working order rather than a requirement to exhaust a family before
starting the next. An observed A1 source blocker or a feature that unlocks several
other families can move a bounded slice forward. Each adjustment should name the
actual source use case and dependency. Prototype/accessor completeness, every
coercion edge case and full Test262 coverage do not gate unrelated useful slices.

The first collection slice deliberately offers a runtime position through
`array.at(index)`. General `array[index]` also has JavaScript property-key
conversion and named-property behavior, including negative and fractional
Numbers; those require their own implementation. This distinction keeps the
first slice useful without silently changing the meaning of ordinary syntax.

## First implementation batch

1. **A1 named-helper composition.** Account for the imported Wasm function when
   emitting direct source calls. Execute source helpers around and inside the
   actual service call against the Rust/WIT provider and Node-Wasm.
2. **A1 captured-handler composition.** Extend the same mapping to closure table
   targets and the entry wrapper while retaining logical function identities,
   captured-cell behavior and existing import-free artifacts.
3. **Runtime array positions.** Admit `.at` on known managed arrays, including
   negative positions, sparse misses and source-ordered argument effects.
4. **String length.** Admit `.length` on known managed Strings, counting UTF-16
   code units through helpers, captures and existing supported source contexts.

These are implementation goals at this document's introduction. The feature PRs
and their exact-head test results record completion; this list is not execution
evidence. String construction and more general collection updates are the next
extensions after this batch.

## A1 scope and existing evidence

The current minimal A1 selector already compiles. Its remaining application
acceptance includes connecting input to the source-imported service and the same
state/reflow/repaint path, readable glyphs and an equivalent native Rust client.
See [A1 acceptance](a1-note-preview.md#candidate-next-a1-acceptance-and-compiler-priorities)
and [the scalar service profile](a1-ui-source-import.md). The language train makes
that application code easier to author; it does not mark those integration gates
complete or reassign their implementation interfaces.

Recent merged work already supplies managed references across related async
activations ([#375](https://github.com/tseguine/grunkalunka/pull/375),
[#377](https://github.com/tseguine/grunkalunka/pull/377)), object spread
([#379](https://github.com/tseguine/grunkalunka/pull/379)) and a scoped runtime
`Object.keys` operation with async source-error routing
([#380](https://github.com/tseguine/grunkalunka/pull/380)).
[#374](https://github.com/tseguine/grunkalunka/pull/374) fixes the original numeric
captured `for-let` reproducer; the broader
[#334](https://github.com/tseguine/grunkalunka/issues/334) acceptance matrix remains
separate. Older dated overview sections are not the new implementation queue.

## PR stacking and validation

Use dependency-ordered PRs with each dependent branch based on the preceding
feature branch. Keep each PR useful and reviewable on its stated base, and state
the owning compiler/runtime stream, behavior added, remaining admission limits
and executed checks. The maintainer registers native stack membership when the
connector cannot; linked PR bases alone do not establish that registration.
The existing caretaker handles validation and integration. Local tests, hosted
checks, native stack membership and merge state remain distinct observations.

For each admitted slice, exercise actual JS/TS source and meaningful result,
effect-order or identity observations in the existing Node-Wasm/Wasmtime suites,
using native JavaScript as the semantic reference where applicable. Preserve
small-stack gates and existing resource diagnostics. Keep tests in canonical
suites with accurate dependency selection. Passing a narrow slice is sufficient
to move on; a known admitted-source miscompile needs correction or an explicit
scope boundary. Unimplemented valid syntax remains a tracked feature gap.

The broader [stack audit](https://github.com/tseguine/grunkalunka/issues/281),
[semantic-operations direction](https://github.com/tseguine/grunkalunka/issues/355)
and [managed async boundary](https://github.com/tseguine/grunkalunka/issues/356)
continue to inform implementation without making their entire scope a dependency
of every new feature.
