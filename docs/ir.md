# Numeric region IR

The numeric compiler lowers admitted Oxc syntax into an internal typed, structured
region representation, verifies that representation, then emits WAT and encodes
Wasm. The public `compile(source, typescript)` API still returns `Artifact { wat,
wasm }`. The [source admission contract](compiler.md) remains authoritative.

The original Oxc AST, including parameter/return/local annotations and expression
wrapper types, remains available through compilation. Annotation syntax is
unvalidated metadata; it emits no runtime operations and is not copied into the
numeric IR. `Number`/`Predicate` are established by the ABI and runtime lowering,
not annotation claims. No type checker, early source stripping or persistent
metadata interface is introduced. Type-guided optimization remains separate work.

`src/lower.rs` owns source admission and lexical symbol resolution.
`src/ir.rs` defines the representation and verifier. `src/emit.rs` emits structured
Wasm. These files live in `crates/grunkalunka-compiler/`.

## Modules and direct numeric calls

A module holds an entry-first list of functions, indexed by `FunctionId`. Each
function records its numeric parameter count and owns independent source slots,
values, regions and loop-emission state. Only entry zero is exported as `run`,
with its original two-number/result-number ABI. Helpers may have zero through
1,000 parameters; every function returns a Number on all structural paths.

`Call` records a callee ID, ordered argument ValueIds and a numeric result. The
module verifier collects signatures before checking bodies, then verifies callee IDs,
exact arity and visible Number arguments before defining the result. Emission
loads captured arguments in order, emits a direct numeric-index Wasm call, then
materializes its result. No table, closure, new value type or host import is used.
Forward/self/mutual calls use the same operation. Source function bindings cannot
be written; shadowed references use semantic identity rather than spelling.

Module-wide statement/local budgets survive lowering each function. Added helper
declarations and every body/directive count once; generated calls/regions do not
add source statements. Single-function modules retain their original artifact
bytes. Internal calls share runtime fuel and the engine's existing stack guard.

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
`Call`, `Constant`, `Load`, `Unary`, `Binary`, `Compare`, `Truthy`, `Boolean` and `Not` produce values.
`Boolean` defines a `Predicate` and emits canonical `i32` zero/one; it never
produces a numeric source value. Source slots and function returns remain `Number`.
`Not` consumes and produces a `Predicate`, emitting `i32.eqz`. Numeric inputs
first pass through `Truthy`; negation evaluates its operand once.
`Unary::BitNot` consumes and produces a `Number`. Emission truncates its captured
input toward zero, reduces it modulo `2^32`, then uses saturating `f64`-to-`i64`
conversion and `i32.wrap_i64`. NaN and infinite inputs reduce to NaN and saturate
to zero. Complement uses `i32.xor` with -1 and signed conversion back to `f64`.
The inline sequence reads the same captured value twice; it never reevaluates the
source operand or mutates its snapshot. Its integer stack intermediates require
no new IR type, source cell, import or generated Wasm function.
`Binary::BitAnd`, `BitOr` and `BitXor` also consume and produce `Number` values.
Lowering captures the left operand before lowering the right; both execute eagerly.
Emission applies the same conversion helper to each captured value, uses `i32.and`,
`i32.or` or `i32.xor`, then converts the signed result to `f64`. The verifier's
existing numeric operand/result contract applies. Arithmetic binary emission and
the conversion helper remain unchanged.
`ShiftLeft`, `ShiftRight` and `ShiftRightUnsigned` are also Number-valued binary
operations. They use the same captured bit conversion and eager operand order.
The count's low five bits agree for `ToInt32` and `ToUint32`; Wasm shifts mask
them modulo 32. `i32.shl` and `i32.shr_s` return through signed `f64` conversion.
Only `i32.shr_u` returns through unsigned conversion, preserving values up to
`4294967295` instead of interpreting the high bit as a negative sign.
`Binary::Remainder` consumes two captured `Number` operands and produces a
`Number`; the ordinary verifier rejects predicate inputs/results. `%=` reuses the
existing target `Load`, RHS, `Binary`, `Store` and yielded snapshot ordering.
The emitter records helper demand while visiting every region of every source
function, including unused helpers. One private arithmetic function is appended
after all source functions; symbolic helper calls do not move source-call indices
or share source-function local namespaces. Without remainder the prior output is
unchanged. The helper is neither a source function nor a new IR value kind.

The original helper classifies special values, then reduces unsigned integer
significands. After the first remainder, `r < my < 2^53`; exponent difference is
at most 2045. Each nonzero loop advances `min(d, clz64(r))` bits, at least
`min(d, 11)`, bounding reduction by 186 iterations under these invariants. Shifts
cannot overflow unsigned 64-bit width, and zero is guarded before `clz` and
reconstruction. Reconstruction adds the aligned significand and exponent fields
to preserve the implicit-bit carry, then restores the dividend sign. This is a
conditional integer argument, not exhaustive binary64 execution or a measured
fuel-headroom guarantee. See the standard [Number remainder rule](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-remainder).

`Initialize` and `Store` consume numeric values and change source cells.
Unresolved `Infinity` and `NaN` reads use the existing numeric `Constant`
operation. They allocate no source cell and perform no host lookup. Resolved
symbols with those names continue through ordinary binding and TDZ checks.
Operations execute in their recorded order. A load captures the cell's value at
that point; an assignment yields its stored value snapshot. Compound assignments
load the previous target before evaluating the RHS, then store and yield their
computed result. Prefix updates yield the new snapshot; postfix updates yield the
old snapshot after storing the new value.
Bitwise/shift compound operators reuse this same `Load`/`Binary`/`Store` path,
including unsigned Number results from `>>>=`; they add no new IR operations.

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
or `Return(value)` exit; statement regions can also exit with `Break`. `Block`, `If`, `While` and `DoWhile` preserve structured control flow.
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

## Unlabelled loop exits

Lowering and verification independently compute possible `Normal`, `Return`,
`Break` and `Continue` completions with the same small algebra. Sequential composition preserves
prior abrupt alternatives and applies the suffix only to normal paths. Branches
union their outcomes. A fallthrough exit preserves that accumulated set: it does
not invent a normal path after all-abrupt branches. Functions must end with exactly
`Return`; runtime statements after guaranteed abrupt completion are rejected.

Explicit type aliases/interfaces, including direct named exports and default
interface exports recognized by source admission, emit no IR. They charge one
source statement each and retain ordinary depth checks, even after abrupt
completion. Sequencing their `Normal` summary through `completion.then(NORMAL)`
preserves the accumulated return/break/continue alternatives without introducing
normal completion. Type members add no values or source cells. The original Oxc
AST and semantic identities remain available; type names are not numeric bindings.

A loop consumes only its nearest body's breaks and continues. Pretest loops remain possibly
normal; do loops convert body breaks/continues into possible normal loop completion while preserving
possible returns. All explicit condition/yield regions and for steps must complete
normally, even under an outer loop body. Their verifier context denies inherited
break/continue through nested blocks and branches. A do body with return and
continue alternatives therefore still needs a following numeric return. The generated for-initializer `Block`
is an ordinary untagged lexical region; source lowering enforces its numeric
initializer restriction rather than claiming the verifier identifies it as a header.

The emitter activates paired exit/body-end targets only during each loop body and
restores the outer targets after nested loops. A break branches to the outer loop
exit, bypassing a for step or do condition. A continue branches to the generated
body-end block, then reaches the while condition, the for step followed by its
condition, or the do condition. These blocks allocate no source cells and consume
no source budgets. They can increase artifact size and instruction cost; fuel
remains 10,000 per call. Labels remain outside admission.

Oxc-recognized module/function directive prologues consume one source statement
each in the same cumulative budget as the numeric body. They emit no IR, values,
source cells or nesting levels: the explicit numeric-return ABI cannot observe
these literal completion values, and ESM source remains strict. General string
expressions are not added by this erasure.

An initial hashbang recognized by Oxc is discarded without any IR, values, source
cells, statement charge or nesting level. Its original UTF-8 bytes remain in the
source-size budget. Parsing uses unchanged source, so prefixed or misplaced
hashbangs keep their syntax errors; the numeric entry contract is unchanged.
