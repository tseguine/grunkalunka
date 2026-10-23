# Numeric region IR

The numeric compiler lowers admitted Oxc syntax into an internal typed, structured
region representation, verifies that representation, then emits WAT and encodes
Wasm. The public `compile(source, typescript)` API still returns `Artifact { wat,
wasm }`. The [source admission contract](compiler.md) remains authoritative.

`src/lower.rs` owns source admission and lexical symbol resolution.
`src/ir.rs` defines the representation and verifier. `src/emit.rs` emits structured
Wasm. These files live in `crates/grunkalunka-compiler/`.

## Source cells and computed values

A `SlotId` names a numeric source binding. Each slot records its owning lexical
region, mutability, parameter status and source span. Parameters begin initialized;
local declarations initialize their own region's slots once per lexical entry.
A loop body may enter its region repeatedly and initialize those cells on each
entry. Stores require an initialized, mutable cell.

A `ValueId` names one typed definition in the IR: `Number` or `Predicate`.
`Constant`, `Load`, `Unary`, `Binary`, `Compare` and `Truthy` produce values.
`Initialize` and `Store` consume numeric values and change source cells.
Operations execute in their recorded order. A load captures the cell's value at
that point; an assignment yields its right-hand value snapshot after storing it.

For example, the essential sequence for `a + (a = b)` is:

```text
v0: Number = Load slot_a
v1: Number = Load slot_b
Store slot_a, v1
v2: Number = Binary Add, v0, v1
Return v2
```

Reconstructing expression trees or delaying `Load slot_a` until the addition would
change this program's behavior. The emitter therefore materializes every computed
value and load immediately in a distinct typed Wasm temporary local. Source cells
and value temporaries occupy disjoint local indices. Physical temporary locals
execute again on each loop visit, while every definition remains unique in the IR.

## Structured regions and exits

Regions contain ordered operations and an explicit `Fallthrough`, `Yield(value)`
or `Return(value)` exit. `Block`, `If` and `While` preserve structured control flow.
Values are visible in their defining region and nested regions. A child or sibling
value cannot escape through an ordinary reference. A numeric conditional expression
uses an `If` result: both arms yield a number and the parent defines the joined
value. Statement branches have no result and may fall through or return.

`While` owns a condition region yielding a predicate and a statement body region.
The emitter executes the whole condition before each iteration and once more for
the final failed test, preserving condition side effects. Condition temporaries are
invisible to the body. A while never guarantees return because it may execute zero
times. A statement if guarantees return only when both arms do; a block carries its
child's return coverage. Every function must return a number on all structural paths.

Wasm emission uses direct `if`, `block` and `loop` instructions. Lexical-only IR
blocks need no extra Wasm control instruction. The final `unreachable` assists Wasm
validation after verified returning branches; the verifier must establish return
coverage before emission reaches it.

## Verification and inspection

Before emission, the verifier checks value and slot IDs, unique definitions and
region IDs, operand/result types, definition-before-use, enclosing-region value
visibility, lexical slot ownership, initialized reads/stores, immutable-cell writes,
exactly one local initialization per entry, and the exit contracts above. Violations
are compiler-internal errors carrying the relevant source byte span. Source syntax
and admission diagnostics are produced before IR verification.

The representation derives Rust `Debug`, allowing a compiler unit test or debugger
to inspect the complete function, slots, typed values, operations, regions and
exits without a new public API. Focused unit tests construct valid and invalid IR
directly. The existing differential corpus checks generated code against original
JavaScript in Node and executes it in Node-Wasm and Wasmtime.

Source-size, nesting, statement and source-local budgets count source constructs.
Generated regions, values and Wasm temporaries do not consume those admission
budgets. Temporary materialization increases artifact size and Wasm instruction
count; the runtime fuel allowance remains unchanged. This representation adds no
optimizer, arbitrary control-flow graph, SSA conversion or new admitted syntax.
