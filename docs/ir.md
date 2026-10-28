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
fulfillment or rejection together with a represented General payload.
`SettlePromise` runs a closed General-return region synchronously and captures
its return or throw. Lexical catches, loops and inline calls keep their lexical
boundaries; an inline call's normal result kind does not narrow its General throw
payload inside this closed scope. Actual suspension anywhere inside is invalid.
`PromotePromise` explicitly converts a fulfilled snapshot to the settled kind.
`SettledValue` projects its General payload, and `PromiseRejected` queries its
settlement. None of these operations adds a reaction. The backend keeps rejection
in bit 31 and the General kind in the remaining low bits, so intrinsic Error kind
4 survives fulfillment, rejection, private transport and either projection. The
two-lane, sixteen-byte carrier and canonical host General tags are unchanged.

`AwaitSettled` schedules exactly one caller reaction. Fulfillment continues with
the General value; rejection enters the caller's nearest catch on that reaction or
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
frame evidence does not establish recursive source initialization, adoption,
identity comparison or unhandled-rejection tracking. Separate pending-value source
requests below admit stored identities and detached execution.

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

Ready-job count and newly-issued-job count are separate. Queue-aware adapters
schedule one host microtask per enqueue at that boundary, preserving the order
relative to unrelated host callbacks. Records share the existing one-MiB arena;
handle range/alignment/kind checks and canonical primitive validation precede use. A
pending record cannot be projected as a settled primitive.

`Promise` is now a distinct logical identity type. `StartAsyncClosure` requires
an async closure signature, executes its eager prefix, and returns that identity
at the first suspension or completion. `AwaitPromise` consumes the identity and
defines a General completion only through a later reaction. Handles can move
through slots, captured cells, branch results and typed private arguments/results;
they cannot be packed or projected as primitive completions.

Modules containing those operations link the kernel and use separate invocation
frames for every activation, including overlapping calls to the same function.
Starting a child and resuming a reaction use different control links. Each await
spills its activation; a queued job restores that frame before continuing. Async
body results remain General primitives, separate from the returned Promise handle.

The queue-aware protocol retains `start`/`resume` argument and triple-result
shapes. An additional `take_jobs() -> i32` consumes the newly issued count.
The host schedules that many individual callbacks; each `resume` consumes exactly
one ready guest job. Status 1 means the root is pending, 0/3 mean fulfilled/rejected.
Status 2 belongs only to the legacy primitive protocol. A pending root may have no
ready work, and a settled root may still have ready work. Node retains the instance
for these callbacks; Wasmtime exposes `pending_job_count()` independently of
`state()`. Hosts must account for already scheduled callbacks when registering
new ones. Repeated start and a resume with no ready job trap.

Queue modules also export `take_promise_event() -> (i32,i32,i32,f64)`, consuming
one FIFO tracking event: `(kind, invocation-local ID, primitive tag, payload)`.
Kind 1 reports the first rejected settlement without a handler; kind 2 reports
its first subsequent subscription (tag and payload are zero). Kind 0 is an empty
queue with all remaining lanes positive zero. A handler registered while pending
prevents both events. Repeated settlement or subscription produces no duplicate
event. Tracking records share the existing bounded arena but consume no reaction
jobs. First rejection and later handling may cross guest boundaries within one
host checkpoint; hosts must publish tracking at each boundary rather than wait
for an instance's ready queue to empty. This preserves cross-instance event order.
The Node bridge uses native rejected Promise proxies; Wasmtime exposes a consuming
`PromiseNotification` API and leaves checkpoint policy to its embedding.

Modules without identity operations retain their previous emitted protocol.
Source pending-value requests now construct these operations. Positive queries
follow original bindings and signatures without executing source expressions.
The cursor distinguishes a Promise handle from fulfilled/settled snapshots and
General primitives; aliases and captured getters retain the handle across stores
and suspension. Async call evaluation snapshots the callee before all arguments,
then starts its prefix eagerly. Existing direct-await requests retain their old
path. `Promise.resolve` of an existing represented identity preserves it while
evaluating surplus arguments in order.

Positive pending input facts now select `Promise` parameters in source private
signatures. They follow original constructors, binding assignments, call argument
origins and homogeneous synchronous returns. Known and runtime call evaluators
request identity values for these parameters; definition checking uses the same
kind. Captures and synchronous forwarding keep the existing i32 handle and its
private frame/cell layout. This adds no IR operation or host call. Conflicting
actual kinds remain rejected rather than implicitly packing a handle as a primitive.

`AdoptPromise { result: General, input: Promise }` resolves the current physical
async activation through a queued registration and a subsequent reaction. A
fulfilled reaction continues with the primitive result; a rejected reaction
settles that activation and bypasses its lexical catches. Source emits it for
pending async return values. It is not interchangeable with `AwaitPromise`.
The verifier rejects adoption in snapshot settlement bodies and inside an
inlined `AwaitCall` child, which lacks a separate physical activation; ordinary
structural `InlineCall` regions in the owning activation remain valid. The backend
saves/restores the frame before the registration callback. Both callbacks use
the existing reaction queue and runtime job budget, with no new host ABI, kernel
record layout or explicit await-site charge.

This does not complete Promise compatibility. Source snapshots now normalize to
identities before escape and adoption. Thenables, general payloads, mixed
Promise/scalar input interfaces and independently escaping Promises remain open. Detached
primitive rejection reporting is covered by
[pending source values](runtime-closures.md#pending-source-values). Maintained IR
tests compare complete native Promise/Node-Wasm microtask traces; separate runtime
tests cover queue accounting, root settlement, traps and the 32-job boundary.

### Intrinsic Error value extension

Self-resolution rejection adds an object identity to the async General carrier.
Modules containing `AdoptPromise` link an Error-aware version of the existing
kernel plus a private Error-record helper. Records occupy 32 aligned arena bytes
with a distinct magic and kind; the f64 payload carries the exact positive ID.
`error_kind` checks canonical integer conversion, alignment, range, record kind
and intrinsic error code. Promise and reaction record headers have different magic
and are rejected by Error-record validation.
The existing General tag/payload lanes, cell/frame layout and reaction queue stay
unchanged. Modules without adoption retain their previous emitted kernel and
value semantics; the primitive exported return ABI still rejects async tag 4.

The adoption self-check settles the owning Promise with this Error before normal
registration. Guest catches receive it through existing rejection continuations.
Error-aware numeric coercion yields NaN, identity comparisons use tag/ID, and
truthiness/nullish operations retain object behavior. Synchronous private helpers
in the same module receive the same Error-aware emitter behavior. String-producing
coercions remain separate lowering work.

### General relational comparisons

`RelationalCompare` consumes two visible General snapshots and defines a Predicate.
Its `Relational` operator has exactly `<`, `<=`, `>` and `>=`; equality remains a
separate operation. Source lowering evaluates each operand once, left to right,
retaining the first snapshot across RHS writes, private calls and suspension.
Proven Number operands retain their existing scalar `Compare` specialization.

General kinds survive until comparison, rather than passing through
`NumberFromGeneral`. This distinction matters for the intrinsic Error extension:
two represented Errors convert to the same intrinsic TypeError string, so `<` and
`>` are false and `<=` and `>=` are true even for distinct object IDs. Emission can
use this equal-string result directly without allocating a temporary string.
This relies on the current single intrinsic Error kind and fixed message. New
Error kinds, mutable properties, user coercion and General String values must
extend the dispatch before admission.

For the remaining represented pairs, numeric conversion applies: Undefined and
an Error converted alongside a non-string primitive yield NaN; Null converts to
zero; Boolean and Number payloads supply their numeric values. All four comparisons
with NaN are false. Explicit unary numeric conversion still happens independently:
`e <= e` is true, while `+e <= +e` is false. No Error ID participates in numeric
ordering. Primitive return protocols, General lanes and the async queue are unchanged.

### Represented Promise identity equality

`PromiseIdentityEqual` consumes two visible `Promise` values and defines a
Predicate. Its negated form implements inequality. The verifier rejects General,
Number and immediate fulfilled/settled snapshots as identity operands. Emission
compares the existing i32 handles; completion records are never read. No reaction,
rejection-handler registration, allocation, await-site charge or job is added.

Source `===`, `!==`, `==` and `!=` share this operation for two represented Promise
identities. Operand evaluation remains once, left to right, preserving the first
handle across RHS rebindings, helper calls and suspension. Equal completions do
not make distinct Promises equal, and `Promise.resolve(p, extras)` retains its
existing handle/effect contract. Private Promise inputs and captured aliases use
the same comparison.

When exactly one operand is a represented Promise and the other is a supported
General value, both are evaluated and the equality result is false. General
currently contains non-string primitives and intrinsic Error objects: numeric
loose comparison cannot equal a Promise's ordinary primitive string, and the
Error is a distinct object. Inequality negates that result. No handle-to-Number
cast is used, even for a Number equal to the opaque ID. General String
admission or custom Promise coercion must extend this source route; these are
not covered by the current constant cross-kind result. Immediate snapshots still
need conversion to stable identity records before identity comparison.

### Completion snapshot materialization

`MaterializePromise { result, input }` consumes a visible `FulfilledPromise` or
`SettledPromise` snapshot and defines a fresh `Promise` identity. The verifier
rejects other input kinds, undefined inputs and non-Promise results. Each execution
allocates once, even for the same snapshot; copying the result retains that identity.
This is a constructor effect, not a cast or an allocation on alias access.

Emission uses the fresh-record `$promise_completed` kernel helper, preserving the completion's
canonical kind, payload and rejection status, including intrinsic Error identity.
Construction adds no reaction or job and does not mark a rejection handled.
The result supports existing identity equality, await, adoption and synchronous
private helper returns. Constructor-only modules also link the queued kernel;
ordinary numeric and value profiles reject this runtime-dependent operation.
No Promise layout, physical lane width or host interface changes are needed.

Maintained IR tests compare native JavaScript and Node-Wasm results and complete
microtask traces, including repeated helper calls, Error payloads, adoption and
detached rejection reporting. The async frontend now uses this bridge for
completed source constructors and immediate async results, allocating once and
retaining aliases. `Promise.resolve(p)` preserves the existing identity. Those
values support comparison and async-return adoption, including through homogeneous
private interfaces. Broader payloads, source storage/call unions and thenables remain.

The fresh-record helper validates canonical kind/payload/status and initializes
the existing header directly, avoiding checks for previous settlement or waiters
that a new record cannot have. Fixed 32-byte allocation preserves alignment,
memory-growth checks and the 1 MiB arena cap. Rejected creation emits the existing
notification. General/Fulfilled immutable SSA pairs may share physical locals;
source slots remain separate. Snapshot IR fixtures explicitly exercise the
logical operations independently of frontend constructor normalization.

### Resolving selected async returns

The source evaluator carries an async-return request through conditional and
comma selections. Homogeneous Promise expressions keep the existing single
`AdoptPromise` route; mixed selections resolve each result arm to General using
either primitive evaluation or adoption. Branch selection precedes adoption,
and comma prefixes keep discard evaluation in source order. This uses existing
regions and operations without changing logical types, layouts or host ABI.

Async constructor effect inference asks whether any selected return leaf is a
Promise. Ordinary Promise value positions still require homogeneous provenance;
this does not introduce a boxed Promise/primitive union. Logical returns keep a
General or Promise left snapshot through the truthiness/nullishness test. The
kept-left branch adopts a Promise only after selection; the right branch resolves
its own return expression. Discarded Promise operands retain unhandled rejection
behavior and add no adoption jobs. Nested heterogeneous left joins use the
selection-local carrier below.
Constructor effect inference conservatively retains an activation for logical
Promise operands, even when runtime short-circuiting discards them.

### Selected and local async values

`AsyncValue` is a private logical union of General values and represented
Promise identities. `ToAsyncValue` accepts only those two visible input kinds;
`AsyncValueIsPromise` returns Predicate. `AsyncValuePrimitive` and
`AsyncValuePromise` project to General and Promise respectively and trap on the
wrong variant. The verifier checks inputs/results and rejects the carrier as
an exported completion. Private closure inputs and results may carry the union
as described below.

The shared backend layout uses the existing tag/f64 pair. Its private Promise
discriminator is 5, distinct from General kinds 0 through 4; a Promise ID fits
exactly in the payload. General values retain their original bits. The carrier
does not allocate or settle anything. No host completion accepts the private
discriminator. Logical tests branch on the variant, treat Promise objects as
truthy/non-nullish and otherwise use existing General operations. Final return
resolution projects a selected Promise into the existing `AdoptPromise` route.

Maintained tests check nested source effects/catches/job limits in native JS,
Node-Wasm and Wasmtime, plus typed IR misuse, projection traps and carrier
preservation across suspension and sibling coroutine frames. Const and let source
locals and captures now use these same slot/cell operations. Private input unions
now use this carrier. Mixed addition/relational operands remain open. Equality
can consume selected raw variants as described below.

### Equality over selected raw variants

Mixed Promise/General equality operands reuse `AsyncValue`. The frontend evaluates
both operands in source order, then emits variant tests and existing structured
`If` regions. The Promise/Promise branch projects checked handles and uses
`PromiseIdentityEqual`; the General/General branch projects checked General values
and uses `PrimitiveStrictEqual` or `PrimitiveLooseEqual`. Cross-variant branches
produce the appropriate equal/unequal constant for the admitted non-string
primitive and intrinsic Error domain. Projections execute only in matching branches.

No new IR operation, physical representation, host tag, private call union or
kernel behavior is introduced. Lift/discrimination/equality add no adoption or
handling operation. Both carriers use the existing suspended-value mapping when
the RHS awaits. General String or custom-object admission must revisit loose
cross-variant equality rather than silently retaining the constant result.

### Source storage

Const and let AsyncValue source slots retain raw snapshots using existing
Initialize, Load, Store and captured-cell storage. Explicit source awaits discriminate before either
AwaitPromise or primitive Await, with both branches sharing the original source
site identity. Async returns instead use the existing AdoptPromise branch.
The carrier remains excluded from exported completions; private inputs/results
now use the existing pair. No operation or physical layout was
added for local storage.


Mutable carrier planning examines original initializer/write provenance before
lowering. Logical assignment retains the original loaded carrier for the skipped
branch, emits Store only in the selected RHS region, and returns the stored raw
value. Consumption follows afterward, so await/adoption never participates in the
assignment test itself. No IR type, operation or storage width was added.

### Private carrier parameters

AsyncValue is now permitted in finite private closure parameter lists, including
callbacks and returned function signatures. The verifier still requires an
environment-bearing private function, matching parameter slots and exact
argument value types; exported input slots remain Number. Private results may
now be AsyncValue; exported completions cannot.

Argument entries use the existing i32 tag at byte zero and f64 payload at byte
eight, with sixteen-byte stride. The ingress lift accepts an existing carrier or
represented General/Promise, packing Number/Predicate first when needed.
No new IR operation or host completion kind was added. Normal private calls and
coroutine activation copies preserve the same logical pair.

### Private carrier results

Finite synchronous callback and returned-function signatures may return AsyncValue.
Inline regions and environment-bearing private functions check that exact result
type. Async signatures still complete with General, and exported roots still
accept Number inputs and complete with the selected Number/General profile.

Private indirect calls reuse the existing tag/f64 result table type. The caller
keeps both lanes across calls and suspension, projecting only at a typed consumer.
Returning the pair emits no adoption or handling operation; fallthrough and bare
return lift General undefined. No IR operation, tag or physical width was added.

### Finally completion continuations

TryFinally contains a protected body, one finalizer region, private return/reason
captures and a return continuation. Protected/finalizer returns carry AsyncValue;
their throws carry General. The raw-completion verifier context permits those
throws alongside awaits, separately from non-suspending settlement bodies.
The return capture is visible only in the continuation. Body snapshots cannot
leak into finalizer/continuation scope, and the hidden reason is not source-visible.
The continuation must return or throw under its enclosing completion contract.

The splitter installs cleanup entries for normal, return, throw and active loop
exits. Cleanup runs under the outer exception/loop targets; its return selects
the return continuation and its other abrupt exits replace the pending completion.
Nested finally scopes keep returns raw until the outer async boundary resolves
them through existing projection/adoption operations. No host completion kind
or physical activation-header field was added.

### Numeric-only selected-value conversion

The async frontend's Numeric context evaluates a may-Promise operand into the
existing AsyncValue carrier once. AsyncValueIsPromise selects an existing If:
the Promise branch yields Number NaN; the other projects AsyncValuePrimitive
and applies NumberFromGeneral. This adds no operation, tag, frame layout or
settlement boundary. Operands without Promise provenance retain the former
Arithmetic route and emission. Addition and relational contexts do not enter
this route because their object primitive-conversion semantics differ.
