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

A declaration may initialize several numeric bindings. Lowering appends each
initializer and its `Initialize` in source order, exposing that binding only after
its initializer and before the next declarator. Symbol identity keeps a later
shadow from falling back to an outer binding.

A `ValueId` names one typed definition in the IR: `Number` or `Predicate`.
`Constant`, `Load`, `Unary`, `Binary`, `Compare`, `Truthy`, `Boolean` and `Not` produce values.
`Boolean` defines a `Predicate` and emits canonical `i32` zero/one; it never
produces a numeric source value. Source slots and function returns remain `Number`.
`Not` consumes and produces a `Predicate`, emitting `i32.eqz`. Numeric inputs
first pass through `Truthy`; negation evaluates its operand once.
`Initialize` and `Store` consume numeric values and change source cells.
Operations execute in their recorded order. A load captures the cell's value at
that point; an assignment yields its stored value snapshot. Compound assignments
load the previous target before evaluating the RHS, then store and yield their
computed result. Prefix updates yield the new snapshot; postfix updates yield the
old snapshot after storing the new value.

Numeric comma expressions need no additional IR operation. Their operands append
operations in source order, and the final operand supplies the result value. In a
predicate position the final operand supplies a predicate instead; earlier operands
remain numeric under the current admission.

Numeric `&&` and `||` reuse a numeric-result `If`. The parent captures the left
operand once and derives its truthiness. One child yields that original snapshot;
the other evaluates and yields the RHS. The operator selects their branch order.
The RHS is always checked during lowering, while its effects run only when selected.

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
or `Return(value)` exit. `Block`, `If`, `While` and `DoWhile` preserve structured control flow.
Values are visible in their defining region and nested regions. A child or sibling
value cannot escape through an ordinary reference. A value-producing conditional
uses an `If` result: both arms yield its declared type and the parent defines the joined
value. Statement branches have no result and may fall through or return.
The verifier checks the result ID before choosing the arm contract, requires both
arms to yield that exact type, and exposes the joined value only after both arms.
Emission selects `f64` or `i32` from the verified result type. Numeric ternaries
and logical value expressions join numbers. Logical conditions join predicates:
the left predicate is evaluated once and the RHS predicate only in its selected
branch. Source ternaries consumed as conditions also join predicates, converting
each arm in its own region. Number-value ternaries still require numeric arms;
Boolean locals and returns remain outside admission. Previously admitted numeric
ternaries in predicate positions may therefore emit different bytes, while their
results and selected effects must remain equivalent.

`While` owns a condition region yielding a predicate, a statement body region,
and an optional statement step region. Existing source `while` loops have no step.
The emitter executes the whole condition before each iteration and once more for
the final failed test, preserving condition side effects. Condition temporaries are
invisible to the body and step. A step executes after a normally completing body,
before the next test; a function return skips it. All three regions are siblings,
so their new values and source cells cannot leak into one another. The verifier
checks the step even when the body always returns.

A source `for` initializer lives in a surrounding lexical `Block`, whose header
bindings are visible to the condition, body and step but not after the loop.
Initialization runs once; body locals initialize on each entry. Omitted tests use
a generated true predicate. Pretest loops remain conservative for return coverage,
including `for` with an omitted test. A statement if guarantees return only when
both arms do; a block carries its
child's return coverage. Every function must return a number on all structural paths.

`DoWhile` owns sibling body and condition regions. Both inherit only outer values
and initialized cells; neither can use new values or cells from the other. The
emitter executes the body, evaluates the whole predicate condition, then branches
back when it is true. A function return skips the condition at runtime, but the
verifier always checks it. Because the body executes at least once, `DoWhile`
carries its body's guaranteed-return coverage. Other abrupt control forms remain
outside source admission.

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
count; the runtime fuel allowance remains unchanged. A `for` or `do` counts once, each
actual body statement/block/return counts once, and a header declaration counts
once as a statement, with one local charged per declarator. Generated header blocks,
loop regions and the omitted-test predicate add no source statements or source nesting. Header
expressions use the existing expression depth checks. There is no optimizer,
arbitrary control-flow graph or SSA conversion.
