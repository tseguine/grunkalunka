# Typed region IR

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

The verifier keeps pending regions, lexical visibility and child-completion
continuations on an explicit stack. Its checks retain source order without native
recursion through region nesting. CLI regressions exercise the default executable
stack at the 64-statement limit, including a body with 128 nested expressions;
65 statements retain their depth diagnostic. This does not remove recursion from
parsing, source classification, emission or destruction.

## Modules and direct numeric calls

A module holds an entry-first list of functions, indexed by `FunctionId`. Each
function records its numeric parameter count and owns independent source slots,
values, regions and loop-emission state. Only entry zero is exported as `run`,
with its original two-number/result-number ABI in the numeric profile. Helpers may have zero through
1,000 parameters; the numeric profile returns a Number on all structural paths.
The primitive return profile below changes only the entry result signature.

`Call` records a callee ID, ordered argument ValueIds and a numeric result. The
module verifier collects signatures before checking bodies, then verifies callee IDs,
exact arity and visible Number arguments before defining the result. Emission
loads captured arguments in order, emits a direct numeric-index Wasm call, then
materializes its result. This operation uses no table, closure, new value type or host import.
Forward/self/mutual calls use the same operation. Source function bindings cannot
be written; shadowed references use semantic identity rather than spelling.

Module-wide statement/local budgets survive lowering each function. Added helper
declarations and every body/directive count once; generated calls/regions do not
add source statements. Single-function modules without captures retain their original artifact
bytes. Internal calls share runtime fuel and the engine's existing stack guard.

## Source cells and computed values

A `SlotId` names a fixed-type cell (`Number`, `Predicate`, `General` or `Closure(signature)`). Each slot records
its type, owning lexical region, mutability, parameter status and source span.
Source lowering constructs scalar `Number` slots, immutable `Predicate` const slots,
and immutable/mutable `General` local slots.
Initialized runtime function bindings additionally use `Closure(signature)` slots and
values. `ClosureEntry` imports captured cells, `MakeClosure` constructs a typed
code/environment pair and `ClosureCall` invokes it through the private table ABI.
The verifier checks initialized capture type/mutability, ordered parameter kinds
and calling convention. Numeric-leaf signature IDs equal arity; higher-order IDs
intern ordered Number/numeric-leaf callback parameters and a result type in the
module. The compilation registry is shared across pure queries and scratch checks;
only verified initialized IR reaches the emitter. Each function also records its
result type; lifted runtime results are Number or a numeric-leaf Closure. Logical
lifted parameters use those fixed types; emission copies Number or paired callback fields
from eight-byte argument frame entries. See [runtime closures](runtime-closures.md)
and [higher-order callbacks](higher-order-callbacks.md).
Mutable function slots use the existing typed `Store` operation, which checks
mutability and exact logical signature. Assignment expressions preserve the RHS `ValueId`
snapshot; capture imports preserve the source slot's mutability.
Known local helper specialization can initialize generated parameter cells from
function pairs as well as Numbers. These specialized cells are not physical Wasm
parameters. `ClosureCall` uses typed logical parameters through a fixed two-pointer
physical ABI, with f64 Number or paired i32 function results. Ordinary source
entry/helper parameters retain Number slots and ABI.
See [runtime function results](runtime-function-results.md).
Parameters begin initialized; local declarations initialize their own region's
slots once per lexical entry.
A loop body may enter its region repeatedly and initialize those cells on each
entry. Stores require an initialized, mutable cell.

`CaptureCells` marks initialized, lexically visible slots that need stable
addresses. It produces no value and executes no source effect. The emitter boxes
those slots at their existing initialization points; the marker itself emits no
instruction. See [captured cell storage](captured-cells.md). Source function zero
retains its internal call index; modules with captures export a reset wrapper.

A declaration may initialize several numeric bindings. Lowering appends each
initializer and its `Initialize` in source order, exposing that binding only after
its initializer and before the next declarator. Symbol identity keeps a later
shadow from falling back to an outer binding.

A `ValueId` names one typed definition in the IR: `Number`, `Predicate` or `General`.
`Call`, `Constant`, `Load`, `Unary`, `Binary`, `Compare`, `Truthy`, `Boolean` and `Not` produce values.
`Boolean` defines a `Predicate` and emits canonical `i32` zero/one; it never
produces a numeric source value. Default-profile function returns remain `Number`.
`Not` consumes and produces a `Predicate`, emitting `i32.eqz`. Numeric inputs
first pass through `Truthy`; negation evaluates its operand once.
`Unary::Abs` consumes a captured Number and defines a Number through `f64.abs`.
`Sqrt`, `Floor`, `Ceil` and `Trunc` use the same numeric operand/result contract
and emit their corresponding f64 opcode directly. They preserve input signed zero;
negative nonzero sqrt inputs yield NaN. No integer conversion or helper is added.
These operations represent only the numeric profile's pristine Math intrinsics: the lowerer
checks exact syntax, arity and unresolved value identity before emitting them. They
add no host import, helper function, source cell or value type. Existing generic
Unary verification enforces numeric operand/result visibility and definition order.

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

`NumberFromPredicate { input, result }` requires a visible `Predicate` input and
uniquely defines a `Number` result. It emits `f64.convert_i32_u` on the existing
canonical zero/one predicate, implementing Boolean-to-Number only for selected
primitive-Boolean operands of numeric unary `+`, `-`, `~` and binary
`+`, `-`, `*`, `/`, `%`, and two proven Boolean operands of strict predicate equality.
Numeric operations consume and return Number; equality compares those Numbers and
defines a Predicate. Unary plus
copies its Number; unary minus uses f64 negation and preserves the -0 result of
`-false`. These bridges add no Boolean parameter, return or general coercion.
A pure bounded AST shape
classifier selects this path; original operand lowering performs effects once and
counts all retained source wrappers at the original operand depth.
Unary `+`, `-` and `~` additionally select initialized immutable Predicate slots by
the original reference's SymbolId and fixed slot kind. This separate bounded
eligibility check preserves wrappers/depth. The same unchanged method now also
selects the five arithmetic operand callers, without changing the free classifier.
The operand still lowers once through predicate `Load` and the same explicit
conversion; all operators and ABI results remain Number-typed.

`Initialize` and `Store` require an input matching the slot type; `Load` defines
a result of that type. Nonparameter `Number`/`Predicate` slots emit f64/i32 locals.
Both occupy one scalar local, preserving all slot/value index arithmetic. Stores
perform no conversion. Source lowering initializes Number cells and immutable
Predicate const cells; stores still target mutable Number cells.
Unresolved `Infinity` and `NaN` reads use the existing numeric `Constant`
operation. They allocate no source cell and perform no host lookup. Resolved
symbols with those names continue through ordinary binding and TDZ checks.
Operations execute in their recorded order. A load captures the cell's value at
that point; an assignment yields its stored value snapshot. Compound assignments
load the previous target before evaluating the RHS, then store and yield their
computed result. Prefix updates yield the new snapshot; postfix updates yield the
old snapshot after storing the new value.
TypeScript-wrapped source write targets resolve to the same original identifier
reference and numeric slot before these operations are emitted. Each retained type
wrapper and inner parenthesis charges source expression depth; discarded outer
target parentheses are unavailable after Oxc parsing. No wrapper operation or
coercion enters this IR. Symbol identity, const/TDZ checks and whole-module function
write rejection are unchanged, including in unselected or unused source functions.
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
Mutable Boolean locals and Boolean returns remain outside admission. Previously admitted numeric
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
child's return coverage. Every numeric function must return a number on all
structural paths.

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
`Throw`, `Break` and `Continue` completions with the same small algebra.
Sequential composition preserves prior abrupt alternatives and applies the suffix only to normal paths. Branches
union their outcomes. A fallthrough exit preserves that accumulated set: it does
not invent a normal path after all-abrupt branches. Numeric functions must end with
exactly `Return`; async entry paths may return or throw. Runtime statements after
guaranteed abrupt completion are rejected.

Explicit type aliases/interfaces, including direct named exports and default
interface exports recognized by source admission, emit no IR. They charge one
source statement each and retain ordinary depth checks, even after abrupt
completion. Sequencing their `Normal` summary through `completion.then(NORMAL)`
preserves the accumulated return/break/continue alternatives without introducing
normal completion. Type members add no values or source cells. The original Oxc
AST and semantic identities remain available; type names are not numeric bindings.

A loop consumes only its nearest body's breaks and continues. Pretest loops remain possibly
normal; do loops convert body breaks/continues into possible normal loop completion while preserving
possible returns and throws. All explicit condition/yield regions and for steps
must complete normally, even under an outer loop body. Their verifier context denies inherited
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

Discarded expression statements, non-final comma operands and expression-form for
initializers/updates may evaluate Numbers or Booleans. The lowerer inspects retained
result-producing syntax before emitting and preserves original node depths. Boolean
shapes use existing predicate operations; Number-only trees use unchanged numeric
lowering. Mixed logical/conditional results may normalize to predicates because the
value is discarded and Number/Boolean truthiness has no user hooks. Mutable source cells,
arguments and returns remain Number-only; no IR or emitter operation is added.

## Boolean source cells

Immutable Boolean initializers now use the existing fixed `Predicate` slots and
canonical `i32` loads. The source binding is published only after initialization;
aliases resolve through SymbolId and the slot type. Number slots keep their old
lowering. No new IR operation, Wasm layout or call signature is introduced.
See [the predicate-position source contract](compiler.md#immutable-boolean-predicate-locals)
for the intentionally narrower source reads, initializer forms and shared local budget.

## Arithmetic operand conversion

The source arithmetic helper reuses `NumberFromPredicate` only for recognized
inline Boolean operands and initialized immutable Predicate bindings of binary
`+`, `-`, `*`, `/` and `%`. It preserves original depth, effects and captured
left-before-right values. Number operands use unchanged expression lowering. No
IR, verifier or emitter operation is added; the binding-aware eligibility body is
unchanged and no call/storage type is widened. See [the source boundary](compiler.md#inline-boolean-arithmetic-operands).

The arithmetic binding extension is source eligibility only: no IR operation,
slot kind, verifier rule, emitter, call ABI, index layout or conversion changes.
All Number operand paths and their emitted artifacts remain the baseline.

## Discarded Predicate binding loads

The discard selector additionally recognizes initialized immutable Predicate
identifier leaves through existing semantic eligibility. It lowers the original
operand once with the existing Predicate Load and leaves that value unused, just
as for other admitted discarded predicates. No NumberFromPredicate conversion,
new IR operation, verifier/emitter change or source/result type profile is needed.
The free discard classifier and its logical/conditional/sequence rules are unchanged.

## Strict Boolean predicate comparison

Same-kind source `===`/`!==` converts each proven Boolean operand with the existing
`NumberFromPredicate` operation, left before right, then uses unchanged Number
`Compare::Equal`/`NotEqual`. Canonical zero/one conversion preserves Boolean
identity. The verifier still requires Number comparison operands and a Predicate
result. Number paths, value capture, IR types, emitter operations and ABI are
unchanged; this adds no dynamic kind dispatch or mixed-kind equality policy.

## Numeric logical assignment

Source `&&=`/`||=` reuses existing Number `Load`, Predicate `Truthy`, Number-result
`If`, `Store` and `Yield` operations. The parent captures the target before RHS
work; the selected child contains the RHS and its final Store, while the skipped
child contains neither and yields the captured old Number. The selected child
yields the stored RHS snapshot. Existing value visibility, slot mutability and
region verification apply unchanged; no new IR operation, type, emitter layout or
ABI is introduced. See [the source contract](compiler.md#numeric-logical-assignment).

## Type-only module items

Declaration-level TS type-only module items are source admission and accounting
only. They emit no value, slot, operation, module import or Wasm instruction.
The original parser/Semantic pass still precedes filtering; each item charges the
existing shared source-statement budget. Existing numeric IR, verifier, emitter,
runtime and ABI remain unchanged. See [the source contract](compiler.md#type-only-module-items).

Generic function declaration type parameters are frontend metadata only. They add no
IR fields, values, instructions, Wasm parameters or runtime type proofs. Entry/helper
signatures and ordered calls keep the existing numeric contract. See [the source contract](compiler.md#generic-declaration-metadata).

## Primitive return profile

`compile_value` selects an explicit entry-result signature of `General`, retaining
Number parameters and Number helper results. The module verifier checks each return
and each call result against that profile. A call to entry zero produces General;
a numeric helper call produces Number. The selected profile is not inferred from
an annotation, and the existing numeric module debug structure stays unchanged.

`Nullish` defines canonical undefined/null General constants. `Pack` consumes a
visible Number or Predicate and defines a General snapshot; it rejects General
inputs, missing/forward inputs and non-General results. General-valued `If` arms
must each yield a visible General value. General source cells are locals; parameters remain Number.

Emission assigns one physical local per scalar snapshot and two per General
snapshot, computing offsets from logical widths. General loads push tag then
payload, and stores consume payload then tag. General joins and entry returns have
Wasm result type `(i32,f64)`. Calls capture Number arguments in source order. No
heap, imports, boxing allocation or reserved Number bit patterns are introduced.
The [wire contract](adr/0002-tagged-primitive-values.md) defines the four tags.

### Primitive strict equality

`PrimitiveStrictEqual` consumes two visible `General` values and defines one
`Predicate`; its `negated` flag selects `!==`. The verifier rejects scalar operands,
wrong result types, undefined values and uses before definition. With the current
canonical four-kind representation, emission computes `(left.tag == right.tag) &
(left.payload == right.payload)` and optionally negates it. Canonical nullish payloads
are +0, Boolean payloads are 0/1, and Number payload equality supplies NaN/signed-zero
semantics. This operation must be extended before admitting reference/heap kinds;
raw payload equality would not define object or string semantics.

Existing scalar `Compare` operations stay unchanged. The new operation performs no
coercion, allocation or capability access; source evaluation retains left-to-right
snapshots through the explicit pending-work evaluator.

### Immutable General source cells

`Initialize`, `Load` and `Store` use the existing typed visibility/initialization rules
for General cells. General parameter slots are rejected; stores require mutable cells. Every General cell
occupies consecutive `(i32,f64)` locals; a separate SlotId-to-offset map accounts for
this width before assigning computed-value offsets. Scalar-only functions retain
identical layout and output. Initialization consumes payload then tag; loads capture
tag then payload into a fresh General ValueId. General aliases preserve kind and
payload without asserting a more specific runtime type.

Original lexical owners, initialization once per region entry, declaration order,
source-local counts and const-store restrictions still apply. A General binding
feeds Predicate operations through explicit `GeneralTruthy`; Number operations
still require a future numeric conversion.

### General truthiness conversion

`GeneralTruthy` consumes a visible General input and defines a Predicate. The
verifier rejects scalar inputs, wrong result types and uses before definition.
A pending evaluator continuation captures the General result once, then emits
conversion; it does not recursively enter evaluation or charge an extra source node.
Positive leaf routing recognizes null, unresolved global undefined, General cells
and calls whose actual internal result signature is General. Existing scalar
routing and source wrappers retain their original depth charges.

On the closed canonical four-kind domain, `abs(payload) > 0` implements ToBoolean:
nullish payloads are +0, Boolean payloads are 0/1, and Number zero/NaN compare false.
No tag dispatch is needed. New runtime kinds must revisit this operation before
admission. No heap, import or numeric coercion is introduced. General-valued logical
selection uses the contract below.

Represented function values use a separate front-end predicate rule: evaluate the
function expression once through its existing typed cursor, retaining its effects,
then emit the existing `Boolean(true)` operation. Testing truthiness does not emit
an invocation of that function. The pair never enters General storage and no new
runtime kind is admitted by `GeneralTruthy`. Generated region context crosses this
cursor boundary. See [function truthiness](function-truthiness.md).

### General logical joins

The existing `If` operation represents General-valued `&&` and `||`. The evaluator
captures the left General ValueId, emits GeneralTruthy for its condition, and owns a
separate right child region. One arm yields the original left capture without more
operations; the other evaluates and yields the right General capture. The join and
both yielded values have General type under the existing verifier contract.

The same pending continuations retain their original Number/Predicate behavior.
A positive bounded Number-result scan keeps previously admitted Number-only logical
expressions on their scalar path before Pack. Newly admitted discarded logical
expressions use General mode only when the original scalar classifiers do not apply.
Neither conversion nor result selection alters either operand's captured payload.

### Write-aware local cells and plain stores

The readonly storage planner selects General layout for mutable locals whose
initializer/plain writes cannot stay Number under the shared positive result
classifier. Its SymbolId fixed point precedes emission; bindings still become
available to evaluation only after their normal initializer. The source-local cap
bounds supported inference, and oversized modules retain ordinary lowering checks.
Oxc's node store is retained explicitly for this pass; no linter/CFG features are
needed. The planner adds no IR operations or source bindings.

General cells use the same paired physical layout for mutable stores as for
initialization. A pending StoreGeneral continuation consumes an already captured
General RHS, emits the typed Store and returns that same snapshot. When the source
context requires a Number RHS result, it captures a Number, packs a separate General
value for the Store, then returns the original Number. This is storage packing,
not ToNumber. The verifier retains type, initialization, mutability and lexical-owner
checks. Parameters remain Number; unsupported General compound/update/logical
writes are diagnosed before creating scalar loads.

### NumberFromGeneral

`NumberFromGeneral` consumes one visible General snapshot and defines a Number.
The verifier rejects missing/forward inputs, non-General inputs and non-Number
results. Canonical payloads for null, Boolean and Number already equal their
ToNumber result; tag zero (undefined) selects a NaN constant instead. The emitter
uses one tag test and a typed f64 branch without altering a retained payload.
No runtime helper, import or heap allocation is needed.

Lowering schedules this conversion only for unary +/minus and the existing five
binary arithmetic operand requests, after preserving scalar Number/Predicate
paths. It does not turn a numeric ABI return or argument into a coercion position.
Each operand is captured once by the same nonrecursive pending evaluator; General
writes and unsupported operator families retain their admission checks.

### Source parameter shadows

Physical signature slots remain the first `parameter_count` Number slots and begin
initialized. If a source parameter may change primitive kind, lowering creates a
General root local, loads/packs the original incoming Number and initializes the
new cell before body evaluation. Source SymbolId resolution switches to that cell
after initialization. Other parameters receive no added cells or operations.

These generated cells use normal General local ownership/visibility/Store rules;
they are not General ABI parameters or additional source local declarations.
The verifier retains its exact Number signature-slot invariant. The original
positive storage classifier records symbol dependencies, and reverse invalidation
computes the greatest Number candidate set without repeated whole-plan scans.
Unseeded Number cycles remain candidates; normal lexical source validation still
controls admission. A non-Number seed invalidates every dependent candidate.

### Discard routing for General values

Discard requests keep the existing positive scalar and General routes before
falling back to General evaluation. The resulting captured ValueId is forgotten
by the evaluator after its operations/effects have been emitted. A General result
is one logical snapshot even though its physical layout has two locals; no implicit
numeric conversion or additional IR operation is required.

This extends sequence/conditional/general-call and nullish expression statements
while retaining once-only evaluation, lexical validation, selected-branch effects
and original source-budget charges. Every previously admitted expression keeps its
original specialized route. Discarding a value does not waive admission of its
source operands or make General arguments/unsupported operators valid.

### General logical assignment

General `&&=`/`||=` captures one typed `Load`, applies `GeneralTruthy` and builds a
General-result `If`. Only the selected child evaluates the RHS and stores its
captured result; the empty skipped child yields the original captured value. The
existing Number route keeps its scalar operations. Read-only storage planning now
includes possible logical RHS writes in its existing dependency analysis, allowing
Number-initialized source locals/parameters to use General cells when needed.
No operation, signature or source-budget change is required. See
[the source contract](compiler.md#general-logical-assignment).

### Async suspension and continuation splitting

The separate [primitive async profile](async.md) adds `Await` with a Number or
General operand/result pair and a fulfilled/rejected flag for admitted intrinsic
forms. The verifier requires matching Number or General types and defines the result in the original region's
lexical/value scope. Synchronous emission explicitly rejects any Await IR.
General await retains its kind; it does not apply ToNumber. Known-Number operand
planning follows the inspected operand rather than classifying every await as Number.

The async emitter splits verified regions into a bounded continuation graph.
Branches choose child entries, yields copy typed snapshots to their parent result,
loops retain separate condition/body/update targets and early exits bypass the
suffix. Await records a resume state and returns to the host; resumption stores
the awaited value before following the continuation. Internal graph edges create
no host jobs. Each invocation retains its cells and snapshots in a separate Wasm
instance, with a stored state index and terminal completion marker. This is a
conservative coroutine frame, without general Promise values or live-only spilling.

`TryCatch` defines a General reason snapshot only in its handler. The protected
body and handler inherit outer cells/values but not each other's private snapshots.
`Exit::Throw` requires a visible General reason. Source admission restricts throw
to the async entry, and synchronous emission rejects it. It jumps directly to the
nearest lexical handler or emits immediate rejection status 3. Catch replaces the body's throw completion channel with the handler's
alternatives. Awaited rejection still uses the original reaction boundary; caught
source throw adds none. See [primitive source throw](async-throw.md).

### Scoped inline calls

`InlineCall` owns a statement body and defines a Number or `Closure(signature)`
result in its caller. Its child inherits initialized captured cells and argument
snapshots, but has its own typed-return and loop-control scope. The verifier
requires every structural return path to match the result type, including the
complete closure signature, before defining the call result. Child locals and
snapshots do not escape the region. This remains true when the containing function
returns a General value. See [block function results](block-function-results.md).

Synchronous emission branches body returns to a typed result block, preserving the
caller's suffix. The async splitter copies the returned value's full width to the
call result: one field for Number, both code/environment fields for Closure.
It then follows the caller continuation. Neither path turns a closure return into
a return from the containing function. Calls retain original capture slots, fresh
parameter/body locals and separately bounded generated expansion.

### Nullish selection

`GeneralIsNullish` consumes a visible General snapshot and defines a Predicate.
The backend tests only the canonical Undefined (0) and Null (1) tags. It does not
inspect the numeric payload, so false, signed zero and NaN are non-nullish.
Binary `??` inverts that predicate for the existing operand-selection regions;
scalar Number and Closure values use a constant non-nullish predicate. Predicate
source contexts select the actual value before ToBoolean. Both source operands
remain validated; only the selected RHS executes. Existing async splitting emits
the same typed leaf operation and preserves selected suspension boundaries.

### Fulfilled Promise locals

`FulfilledPromise` distinguishes a stored fulfilled primitive Promise from its
General completion. `FulfillPromise` requires a visible General input and defines
that Promise type; `PromiseValue` requires a visible Promise input and defines
General. Source aliases retain the Promise type and snapshot. Existing typed
loads, stores, captures and suspension storage preserve it. Exported parameters
and result profiles reject it. Projection precedes the
existing await operation, so construction adds no job and each await keeps its
reaction boundary. The source evaluator has a distinct Promise mode for initialized
mutable cells, typed assignments and same-kind selections. It uses the existing
branch/store operations and projects only at await; truthiness does not inspect
the primitive payload. See [the bounded source contract](fulfilled-promise-locals.md).

`InlineCall` may yield `FulfilledPromise` from a statically known synchronous arrow,
checking every return against that type. `AwaitCall` still yields only General,
and public-result restrictions are unchanged. Caller await
projects the returned Promise before using the existing suspension protocol.
Private closure signatures may also return `FulfilledPromise`; lifted definitions,
returned-function chains, callback signatures and `ClosureCall` preserve that
logical kind. It remains distinct from General even though the backend reuses
the existing tag/f64 return pair. Private closure parameters may also use this
type, including acyclic callback and returned-function signatures; ordinary
helpers and exported entries retain Number inputs. The shared frame policy uses
sixteen-byte Promise entries, with logical type checks independent of their
General-compatible physical encoding. Source inference now supplies this kind for
fulfilled Promise arguments through known/runtime closures and supported callbacks
and factories. Typed definition inputs, actual binding/call modes and positive
Number-result facts agree on that kind. All actual calls still validate it; mixed
primitive/Promise parameter uses and result interfaces remain separate work.

### Settled primitive completion snapshots

`SettledPromise` is a distinct private value that retains either
fulfillment or rejection together with a represented primitive payload.
`SettlePromise` runs a closed General-return region synchronously and captures
its return or throw. Primitive catches, loops and inline calls keep their lexical
boundaries; an inline call's normal result kind does not narrow its General throw
payload inside this closed scope. Actual suspension anywhere inside is invalid.
`PromotePromise` explicitly converts a fulfilled snapshot to the settled kind.
`SettledValue` projects its primitive payload, and `PromiseRejected` queries its
settlement. None of these operations adds a reaction.

`AwaitSettled` schedules exactly one caller reaction. Fulfillment continues with
the primitive; rejection enters the caller's nearest catch on that reaction or
rejects the async entry. Both branches count as one logical await site. Private
closure inputs/results, callbacks, returned functions, captured cells and async
storage retain the distinct type. Public entries and ordinary helpers still
accept only Number inputs, and entry results remain Number or General.

Non-suspending source async closures with primitive throws now construct this
kind. Constructor groups retain one settled private result interface, including
fulfilling async and synchronous fulfilled-Promise peers. Supported synchronous
callback/result forwarding preserves it; fulfilled branches explicitly promote.
Source settled results are confined to immediate await and synchronous forwarding,
including conditional/comma results. Source Promise slots and parameters retain
their fulfilled-only contract. Payload/status queries remain IR-only operations.
Pending promises, async-return adoption, thenables,
object identity, general payloads and unhandled-rejection tracking remain open.
Executable IR fixtures and maintained source oracles separately compare native
JavaScript with Wasm values and reaction traces; neither implies general Promise
object compatibility.

### Async private calls and invocation frames

A private closure signature can now carry an async call effect. Its `General`
result describes the body's primitive completion, not a Promise object available
to ordinary scalar operations. An `AsyncClosureEntry` imports the environment;
`AwaitClosureCall` validates the complete parameter signature and obtains that
primitive completion on the caller's await reaction. Ordinary `ClosureCall`
rejects the async effect even when its physical result lanes would match.
Callback parameters and functions returned by synchronous factories retain the
complete signature, including the effect.

The backend compiles each async body once into the existing state dispatcher.
Calling it saves the caller's locals, SSA snapshots and captured-cell pointers
in an invocation-local guest frame. A child keeps its own environment and
parameter cells. Completion restores the parent and transfers the primitive
outcome before scheduling the caller's reaction. An uncaught child rejection
first completes that child's reaction; it does not terminate the entry early.
Nested calls with the same signature and recursive activations therefore keep
separate saved state without expanding the call graph recursively at compile time.

Physical headers, lane offsets, code dispatch and spill policy belong to the
backend. Frames use the existing one-MiB invocation arena and are allocated
monotonically; reclamation and liveness-based spill reduction are future work.
The start/resume ABI, default fuel, 32 logical IR await sites and 4,096-state
limit are unchanged. Existing modules without async private entries retain their
previous emission.

The source lowerer now constructs this effect for runtime-selected async arrows
with own suspension. Constructor groups widen immediate async peers to the same
effect; callback inference and returned-function signatures preserve it. Direct
await emits the private awaited call, including original comma prefixes, without
exposing a pending value to ordinary expressions. Known directly awaited async
calls retain their existing inline boundary. Executable IR fixtures
separately cover native-JS/Node-Wasm/Wasmtime values and reactions, caller/body
catches, typed primitive/Promise/callback inputs, returned function pairs,
captured parameters, recursive restoration and resource limits. Their recursive
frame evidence does not admit recursive source initialization, detached pending
Promises, adoption, identity or unhandled-rejection tracking.

For a conditional await operand, the evaluator keeps an awaited request through
the existing conditional/comma cursor. Its leaves return General completions
using the existing primitive, fulfilled/settled or private-call await operation.
There is no pending-value projection or extra await after joining those results.
An adjacent `AwaitSite` marker records alternative continuations of one source
site. The verifier requires an actual await immediately after it; the backend
shares only the logical site count within that compiled function. Child sites,
other functions and unmarked IR operations retain independent counts. The marker
emits no instructions and changes no reaction, frame or host protocol.

### Pending Promise guest support

The backend now has an executable guest settlement/reaction kernel in
`emit/asynchronous/promises.wat`. It owns invocation-local records for pending,
fulfilled and rejected primitive outcomes, stable handles, first-settlement-wins
behavior and a FIFO of continuations. Subscribing to a pending Promise retains a
waiter; settling queues those waiters in order. Subscribing after settlement
also queues a later reaction. Direct primitive reactions use the same ready FIFO.

Ready-job count and newly-issued-job count are separate. A future adapter must
schedule one host microtask per enqueue at that boundary, rather than hide the
queue behind a batched drain. Records share the existing one-MiB arena; handle
range/alignment/kind checks and canonical primitive validation precede use. A
pending record cannot be projected as a settled primitive.

This kernel is a **pending-IR prerequisite**, not new source admission. Existing
modules do not include it yet; one narrow unused-helper expectation records that
following integration. Logical pending values, coroutine integration and adapter
pumping are the next unit. Adoption, thenables, general payloads, host rejection
notifications and independently escaping Promises are not provided by this kernel.
