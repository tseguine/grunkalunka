# Managed values and source data objects

The private structured IR now has a GC-backed value carrier and known object/array
reference kinds. Synchronous source lowering constructs ordinary data objects and arrays
and uses their references, properties, equality and truthiness through this IR.
This is an increment toward [#300](https://github.com/tseguine/grunkalunka/issues/300),
not complete source object/array support. Exported Number/primitive signatures and
the existing async contract are preserved.

[Property deletion](property-deletion.md) removes configurable own data fields,
preserves sparse-array length and updates the source facts used by later reads.

Optional property reads retain the original `?.` chain syntax. Static names,
literal computed keys and original `>>>`-proven unsigned runtime keys work on
source-proven objects, Arrays and Strings, including unions with Null and
Undefined. The receiver executes once; only the non-nullish branch reads its
selected field. The other branch returns Undefined.
Array length, sparse misses, primitive tags and selected reference identity retain
their ordinary behavior. Literal-key reads from a proven nullish-only receiver
perform no property selection, including for intrinsic prototype names.

Receiver/key effects and supported suspension stay on the existing owned cursor.
Unsigned key expressions, including calls and awaits, execute only after a
non-nullish receiver has been captured. Nullish receivers skip those effects;
key effects can change the selected property before it is read while rebinding
the receiver leaves its original reference intact. Complete source operands still
undergo validation, including in a branch that never executes.

String receivers expose their own length and canonical UTF-16 index properties,
using the same lazy unsigned-key path. Index reads preserve individual UTF-16
units, including isolated surrogate units; missing indices return Undefined.
A computed String key may select among known canonical index names and
`"length"` on ordinary and optional reads and in object binding and assignment patterns.
Length counts UTF-16 units.
Optional String runtime index expressions also accept source-proven Number values,
including ordinary loop variables. Fractional, negative, non-finite and out-of-range
indices return Undefined rather than undergoing integer truncation; negative zero
selects index zero. The key still executes only for a non-nullish saved String.
Other String properties, coercing keys and String/object receiver unions remain
guarded.

Optional computed reads also accept source-proven runtime String keys on
null-prototype dictionaries, including nullable dictionary receivers. A nullish
receiver skips key effects and yields Undefined; otherwise the saved dictionary
is read after key evaluation, including supported suspension and intervening
writes. Missing names yield Undefined, and names such as `constructor` and
`__proto__` remain ordinary own data fields. Source facts retain selected tags
and reference identity through aliases, helpers and captures. A nullish-only
receiver can skip a validated String key as well. Ordinary records require an
own-name proof and arrays accept finite choices that exclude inherited names.

Numeric projections still need a Number proof or explicit primitive conversion.
Consecutive optional member reads, such as `model?.selection?.label`, make
each property lookup against its own saved receiver. A nullish link skips
the remaining optional keys, including their effects and suspension.
An ordinary tail such as `model?.selection.label` also short-circuits if
`model` is nullish, when every reached `selection` is a known record,
Array, or String. A missing, deleted, or nullable selection cannot establish
that proof. Parentheses end the chain.
Unknown receivers, inherited fields, other runtime keys and optional calls
remain guarded. Type assertions establish no
receiver or selected-value proof.

Existing mutable numeric/primitive locals and parameters can also receive
`for...of` selections when source facts prove that they fit the existing cell.
Number cells require exclusively Number alternatives; tagged primitive cells
retain Number, Boolean, Null and Undefined tags. The selection is projected once
without user-defined coercion, and captured readers observe the same updated
cell. Empty loops leave it unchanged. Const/unbound targets, incompatible value
kinds and annotation-only proofs remain guarded.

`for...of` and `for...in` can assign each selection directly to a member target.
Static/literal own properties work on source-proven objects and Arrays;
`>>>`-proven unsigned runtime keys also work, and null-prototype dictionaries
accept source-proven String keys. Ordinary records accept String keys proven to
exclude Object prototype names. The iterable is evaluated once, then each
selection is saved before evaluating that visit's receiver and key. These target
effects, including supported awaits, do not run on an empty loop or on a removed
`for...in` key. Rebinding during key evaluation still writes to the captured
receiver. Array length/prototype writes, unknown or nullable targets, coercing
keys and assignment-pattern heads retain guards. Member writes update the same
alias/helper field and Array element facts as ordinary assignments.

Boxed values whose source alternatives are all Number, Boolean, Null or Undefined
can now project into the existing tagged scalar path. This preserves each actual
language tag and every Number bit; arithmetic, explicit coercion, relational
comparison and loose equality use their existing scalar operations. For example,
`const o={v:a?true:null}; return +o.v` works without pretending the field is always
a Number. Missing/deleted fields and sparse holes can likewise supply Undefined
when the existing source analysis proves it. Checked helper/capture flows and
primitive return interfaces retain these tags. Numeric-only returns still require
a Number proof or explicit conversion.

`for...in` also accepts source-proven Boolean, Null and Undefined values, including
mixed alternatives with Number. These scalar values enumerate no keys in the
represented pristine realm. The original RHS executes once, preserving effects,
awaits and rejection; no head assignment, pattern default or body visit executes.
Unsupported receiver kinds and object/nullish unions remain guarded. This is the
language loop contract, independent of the nullish behavior of existing object
enumeration intrinsics. Loop-head facts remain conservative; subsequent primitive
projections still require the ordinary source-kind proof.

`for...in` accepts source-proven represented function values as well. The current
function representation has no mutable own-property table or enumerable intrinsic
properties, so enumeration is empty. Constructing/evaluating the function RHS
still executes once and validates its body; the loop does not call that body.
Lexical/var/assignment heads and pattern defaults remain unvisited. Function
property mutation, opaque callable values and nullable function unions retain
their existing guards.


Primitive projection requires a nonempty set containing only those four origins;
type annotations, String/reference alternatives, unknown writes and unresolved
helper results do not satisfy it. Its verified IR checks a visible managed input
and tagged scalar output; the backend checks that the runtime tag is in the same
four-kind domain. No coercion, user-code call or thenable assimilation occurs in
the projection. Await preserves these primitive source alternatives, including
Null/Undefined; saved operands survive RHS mutation, rejection and suspension.
This extends the earlier singleton Number/Boolean proofs below to primitive unions.

Runtime String-key reads from proven null-prototype dictionaries also retain the
union of every possible own value, including writes through aliases and checked
helpers, plus Undefined for a missing key. When that domain contains only the
four represented scalar primitives, the same projection enables arithmetic,
comparisons and primitive return interfaces. Receiver/key evaluation and saved
operands still use the existing runtime read path, including across await and GC.
Any possible String, reference or opaque value blocks a primitive-only proof;
ordinary-object prototypes, array indexing and non-String key conversion gain no
new admission from this analysis. Unresolved facts finalize conservatively.

Source-proven Number fields and elements now project from boxed storage into
ordinary arithmetic, comparisons and numeric returns. The same path supports
destructured locals, assignment-pattern targets, intrinsic-array iteration and
checked helper results. Each read snapshots the Number before later effects or
awaits; signed zero, infinities and NaN retain their Number behavior.

The finite source-origin analysis requires every alternative to be a Number.
Entry inputs, Number expressions and checked source-call arguments/results supply
facts; type annotations do not. Alias writes, deletes, missing fields, sparse
holes and incompatible alternatives can prevent a proof. Uncertain values remain
boxed and cannot use this projection. The verified `ManagedToNumber` IR operation
also checks the runtime language tag before reading its f64 payload; it performs
no coercion or user-code calls. General runtime coercion and public reference
results remain separate work.

Awaiting a source-proven Number preserves its Number origin, including nested
awaits and boxed storage or assignment RHS values. The existing suspension path
still executes normally. This proof does not copy reference origins through
await: objects and functions may participate in thenable assimilation, and
unknown or mixed operands do not prove numeric fulfillment. Type assertions
cannot turn such an operand into a Number proof.

Managed values now support loose `==` and `!=` comparisons when either operand
is positively known to be Null or Undefined. That side may also be a union of
those two origins. The result tests the other operand's runtime nullish tag:
both Null and Undefined compare equal; every represented non-nullish value is
unequal, including empty Strings, false, zero and object/array/function references.
Both operands still evaluate left to right, with the left value saved before
RHS effects, throws or suspension. This covers `record.field == null` and
`record.field != undefined` without invoking managed coercion. Name spelling
and type annotations supply no nullish proof.

Two source-proven Strings also support `==` and `!=`. Abstract equality between
Strings uses the same exact UTF-16 content comparison as strict equality,
including NUL, surrogate units and distinct allocations with equal contents.
The saved left operand remains independent of RHS mutations or suspension.
Mixed-kind comparisons and uncertain String origins still require separate
coercion work and retain their current guards.

Source-proven String pairs also support `<`, `<=`, `>` and `>=`. Ordering compares
unsigned UTF-16 code units until the first difference, then the lengths when one
String is a prefix of the other. It does not parse numeric-looking text, normalize
Unicode or combine surrogate pairs. Both source operands evaluate left to right;
the saved left String survives RHS mutation, throws, suspension and collection.
The specialized IR verifies visible managed operands and a predicate result, and
the backend checks both runtime String tags. Mixed/uncertain kinds and annotation-only
String claims remain guarded; existing primitive relational behavior is unchanged.

Known object and array reference pairs also support `==` and `!=` by identity,
including aliases, nested data fields and null-prototype dictionaries. An object
and an array are unequal even if their contents match. Neither operand invokes
primitive conversion. Left snapshots survive RHS rebinding and suspension.
Mixed primitive/reference alternatives and annotation-only claims remain guarded;
general coercing comparisons remain separate work.

Supported synchronous function references also participate in `==` and `!=`
identity comparisons. Loose comparisons now trigger the same allocation-identity
analysis as strict comparisons, preserving aliases, module declarations and fresh
closures returned from factories. Positive reference unions may contain functions,
objects or arrays; cross-kind references are unequal. Comparison never invokes
the function. Saved function values survive RHS rebinding and suspension. Async
function storage, primitive/reference coercion and uncertain origins remain guarded.

Source-proven Number pairs in boxed storage also support `==` and `!=` without
coercion. NaN compares unequal to itself, signed zero compares equal to zero,
and infinities retain ordinary Number equality. Fields, elements, destructured
cells and checked helper inputs use the same operation and save the left Number
before RHS effects or suspension. Other represented primitive alternatives use
the tagged scalar bridge above. String/reference alternatives and type assertions
cannot supply the required Number-pair proof.

Source-proven Boolean pairs in boxed fields, elements and cells now support `==`
and `!=` without Number conversion. Boolean literals, comparisons, `in`, `delete`
and logical negation supply positive Boolean origins; aliases and checked helper
flows retain those facts. Left values remain snapshots across RHS writes and
suspension. Mixed Boolean/Number alternatives now use tagged scalar equality;
String/reference alternatives remain guarded, and annotations cannot supply a
Boolean proof. Boolean truthiness stays conservative
in the origin analysis; it does not predict which branch executes.

Proven Boolean values in managed storage can also enter the existing scalar
primitive path. Lowering preserves the saved Boolean tag and payload; explicit
numeric coercion then uses the existing primitive
conversion. This enables scalar consumers such as `+record.enabled` while preserving
Boolean identity and negative-zero behavior. Await preserves positive Boolean
origins, and managed await results box the existing General-protocol completion.
Boxed Boolean operands project before their await, so RHS effects cannot change a
saved value. Pure primitive awaits retain their existing lowering path. Unknown,
mixed and reference/thenable origins remain guarded; there is no new async ABI.

Boolean-only local `&&=`, `||=` and `??=` writes now retain Boolean origins when
the initial cell alternatives and every tracked update result are all Boolean.
This keeps later scalar coercions, loose comparisons and checked helper/capture
flows available after logical writes, including destructured and iteration cells.
Runtime selection, saved old values and successful-RHS stores still use the existing
logical-write path; no Boolean truth value is predicted. Numeric updates, mixed
assignments and uncertain alternatives still widen the source facts and prevent
a Boolean-only proof.

Awaiting a source-proven String also preserves its String origin and contents.
The operand evaluates once, then its traced value stays live across one real
suspension through the existing private numeric await protocol. That internal
fulfilled token's result is unused; the expression returns the saved String.
Strings are primitive and cannot be thenables, so no source code can replace
their fulfillment value. Nested awaits, String concatenation, local and property
`+=`, predicates, destructuring and discarded awaits compose with this path.
Operand throws still bypass the String suspension, and every executed String
await consumes the normal await/job budgets. String-valued Promises, async String
return interfaces and reference/unknown thenable results remain separate work.

Mutable source-proven String locals accept `+=` with a represented primitive RHS. The old String is saved before RHS effects or suspension, concatenated with
the completed RHS, then stored and returned. Captures observe the new String;
RHS rejection performs no store. UTF-16 units use the existing concatenation
operation unchanged. Destructured and iteration cells follow the same path.
Static and computed record fields and array elements also support String `+=`. A known String RHS can append to a represented primitive old value, including a missing (Undefined) field. Their
receiver and old String are saved before the RHS; receiver rebinding does not
redirect the final store. The same operation covers nested receivers. Successful
completion stores once, and a throw or rejection leaves the saved reference
untouched by the compound store.
Type assertions, const local targets,
array length and accessors do not acquire String support from this increment.
Dictionary fields retain an absent alternative in the source analysis; an earlier
plain write alone does not prove that such a field contains only Strings.

The exact original numeric-field, array-element, destructuring declaration,
destructuring assignment and `for-of` rejection sources now execute unchanged in
the canonical corpus. Compiler admission/verifier tests and synchronous/async
native-JavaScript, Node/Wasm and Wasmtime comparisons cover the source path.

Source-proven primitive data fields and static array elements support
prefix/postfix `++`/`--` and the eleven existing arithmetic/bitwise compound assignments. The
receiver and converted old Number are saved before RHS effects or suspension.
A successful write stores a newly boxed Number back into that original receiver;
postfix returns the old snapshot, while prefix and compound assignment return the new
Number. Rejection during RHS evaluation leaves the property unchanged. Ordinary
read and write operations share the existing managed storage and numeric IR.
Runtime unsigned keys proven by original `>>>` source also support these updates
on represented objects, Arrays and null-prototype dictionaries. Null-prototype
dictionaries additionally accept source-proven runtime String keys. All numeric
property updates require stored alternatives from Number, Boolean, Null and
Undefined; missing fields and holes contribute Undefined. Existing primitive numeric
conversion supplies the old Number before arithmetic. Receiver, key and old value
are captured once in source order, including across supported awaits; a rejected
key or RHS performs no final store. Array growth and the uint32 maximum named key
retain the ordinary sparse-storage rules. Type annotations supply no proof.

Static-key updates admit missing/deleted own fields and holes with the same
primitive conversion. Array length, unproven runtime keys, String/reference stored
values, effectful object coercions and accessors retain their guards.

Source-proven own Number properties also support `&&=`, `||=` and `??=`. The
receiver and old Number are saved before selection, and the RHS plus boxed store
remain entirely inside the selected branch. Skipped assignments return the old
snapshot without evaluating the RHS. A proven Number is never nullish, so `??=`
always retains the old property while still validating the complete source RHS.

Static logical property assignments also preserve represented mixed values in
managed, predicate and discarded positions. They test the old boxed value by its
runtime tag, retain String contents and reference identity, and store only after
the selected RHS succeeds. Missing ordinary data fields and sparse-array elements
read as Undefined, so `record.label ??= 'default'` can create a field and an indexed
fallback can grow an array without filling preceding holes. Receiver rebinding
during RHS effects or suspension does not redirect the saved reference. Runtime
keys, prototype names, accessors and array-length writes retain their guards.

Null-prototype dictionaries also admit `&&=`, `||=` and `??=` through a
source-proven runtime String key. The receiver finishes before key evaluation,
the saved key is read before testing the old value, and the RHS runs only when
selected. Rebinding the receiver or key during callbacks or suspension cannot
redirect the store. Failed key evaluation or a rejected selected RHS performs no
write. Dynamic logical writes conservatively invalidate property-kind proofs
through aliases, just like other dynamic writes. General key coercion, ordinary
object prototype lookup and runtime array keys remain separate work.

Mutable local cells populated by destructuring and iteration use the same proven
Number update path. Prefix/postfix updates and arithmetic/bitwise compound
assignments load and validate the old boxed Number before evaluating the right
side, then store a newly boxed Number only after successful completion. Captures
observe the updated cell. Missing, String or mixed-origin local values remain
guarded rather than turning the private projection into general coercion.

The same proven boxed Number cells support `&&=`, `||=` and `??=`. Logical
assignment snapshots and tests the old Number once, evaluates and stores the RHS
only on the selected branch, and returns the exact old or new Number. Because a
proven Number is never nullish, `??=` retains the old cell and skips every RHS
effect. RHS values that could change the cell's kind remain guarded.

Boxed local cells also admit logical assignments with represented mixed values
in managed, predicate and discarded positions. The old value is saved once and
tested by its runtime truthiness or nullish tag; only the selected RHS executes
and stores. Strings, primitive kinds and reference identities remain intact,
including captures and suspension. This supports a nullish fallback such as
`let [x]=[null]; x ??= fallback` without asserting that the cell is numeric.
Numeric consumers still require the existing Number proof. Const/TDZ checks
and validation of the complete RHS apply even when its branch will be skipped.

**Current integration frontier (2026-11-01):** known source synchronous
function values, module declarations, typed closures/callbacks and captures
now work with traced references; later sections document these separately.
[Steering #355](https://github.com/tseguine/grunkalunka/issues/355)
addresses *runtime-dependent* property/call semantics and catchable source
exceptions; [#356](https://github.com/tseguine/grunkalunka/issues/356)
addresses traced references through async suspension. Static source proofs
remain useful fast paths, not a complete replacement for dynamic JavaScript
semantics.

**Loop-header update (2026-11-05):** [#374](https://github.com/tseguine/grunkalunka/pull/374)
corrected the original numeric closure mismatch. The two managed-result and
stored-function fixtures in [#334](https://github.com/tseguine/grunkalunka/issues/334)
now use that existing per-iteration cell renewal.

## Managed values in a single coroutine frame

Objects, arrays, Strings and boxed property snapshots can survive primitive
`await` operations when the module uses one physical coroutine frame. The existing
instance globals persist across resume calls; reference lanes now use nullable
WasmGC globals with typed null initializers. These are engine-traced roots, not
numeric handles or pointers stored in linear memory. Named synchronous helpers
retain their actual reference parameter/result types in an async module.

Synchronous closures can retain managed captures across these awaits. Reuse the
existing traced function objects, shared captured cells and GC argument frames;
the coroutine stores their actual references in typed globals. Captures retain
bindings rather than frozen values, so later rebinding and alias mutation remain
visible through the closure. Returned factory environments, function identities,
callbacks, cycles and Strings use the same representation as synchronous code.
Numeric-only modules retain their existing closure layout and arena.

Aliases, cycles, replacement snapshots, array mutations and the existing
handler/finalizer/job ordering are preserved. An immediate async call that the
existing lowerer inlines into the same physical frame can use this path too.
Success or source rejection clears the generated reference globals after the
return/throw expression has been evaluated. A trap makes the host invocation
unusable; dropping its Store releases the remaining roots without guest cleanup.

Maintained native-JS/Node-Wasm/production Wasmtime tests compare values, external
microtask markers and concurrent independent invocations. Private runtime tests
force copying collections while actual generated code is suspended, and observe
terminal root clearing through test-only exports of the generated globals.
This does not establish collection latency or a public reference ABI.
Additional private tests remove unrelated mutable reference roots while pending,
then collect and resume through a returned closure's environment. Generated
activation roots clear on completion; immutable module-function identities remain
for the instance's lifetime and disappear with its Store.

Related child coroutine activations use the traced storage described below;
host entry invocations retain their existing fresh Store/instance isolation.
Managed await operands, settlements, rejection
reasons and public reference results are not admitted by this increment.
[Steering #356](https://github.com/tseguine/grunkalunka/issues/356) remains open.
Earlier async exclusions below describe their named synchronous increments and
are superseded within these documented coroutine-storage boundaries.

## Managed values across related coroutine frames

Async children in one generated instance can share objects, arrays, Strings,
functions and captured binding cells while retaining their own saved snapshots.
Synchronous helpers may start these children and return normally. Existing prefix
execution, reaction ordering, finalizers and owned jobs after root settlement are
preserved; a child is not an independent host entry or an implicit parallel task.

Numeric control records and Number/primitive lanes keep their existing byte
storage. Reference lanes and closure environments instead live in WasmGC
activation records. A checked map uses the aligned control-record address and
function owner to find each record; restored references are cast to their verified
nominal lane types. No GC pointer or encoded reference is stored in linear memory.
Overlapping activations of one function can therefore reuse working globals
without confusing their saved references or shared captured cells.

The map grows geometrically within the existing 1 MiB arena's 65,536 aligned
positions. Addresses must be aligned, allocated and in range; duplicate live
registration and wrong-owner/stale lookups trap as private backend failures.
The arena does not reuse activation addresses. These checks are not catchable JS
exceptions. Async signatures admit the existing managed input kinds using the
same source facts as synchronous calls; their results still use General, and
existing count, callback and source-kind validation remains.

An activation record exists before its prefix executes and retains references
while suspended. Terminal success or rejection clears that function's working
reference globals and removes only the completed record. Other owned child jobs
keep their records even after the root settles. Uncaught primitive await rejection
also releases its completed guest roots. Traps leave the host invocation unusable;
dropping its Store releases remaining pending records. Immutable module-function
identities keep their instance lifetime. No collection-latency guarantee is made.

Maintained source/native-JS/Node-Wasm/Wasmtime tests compare identity, mutation,
snapshots, reentrant starts and job traces. Private production-adapter tests clear
working references and force copying collections while suspended, then check
completion, child ownership and invalid/stale identities. Managed Promise
settlements/reasons, managed await operands, public references and sharing across
separately instantiated host entries remain unsupported.

The already represented intrinsic self-resolution `TypeError` can also be stored
in these containers or captured across suspension. Its General wire tag differs
from its private GC language tag: boxing validates and preserves the existing
invocation-local Error record identity, rather than treating that identity as an
ordinary object reference. Same-error equality, distinct-error inequality,
truthiness, nullish selection and `typeof` retain their JavaScript behavior.
This does not add Error constructors/properties/prototypes or unbox a managed
property snapshot into a Promise payload or public result.

## Object spread composition

Object literals admit spread from the currently represented value domain. Each
operand executes once in source order, before its properties are copied and the
next initializer runs. Copying creates new own data entries while retaining the
boxed property values: nested objects, arrays and functions preserve identity,
and later replacement of a source property does not replace the copied value.

Objects contribute their enumerable data entries. Arrays contribute sparse own
properties, including named entries, while their non-enumerable logical `length`
is excluded. Strings contribute one property per UTF-16 code unit under its
decimal index key. Null/undefined, Numbers, Booleans, represented functions and
intrinsic Errors contribute no enumerable properties in this admitted profile.
The helper operates on existing WasmGC references and can compose with helpers,
closures and suspended activations; it introduces no host operation or new heap.

[Object rest bindings](object-binding-patterns.md#object-rest) copy remaining
own fields into a fresh object. Accessors, proxies and general prototype/Get
semantics remain unsupported. A spread that might overwrite a known callable
field invalidates that earlier source fact; an explicit later property can
establish it again. This preserves rejection of an unproven field call rather
than allowing a wrong-kind private projection trap. The physical property table
is not a general JavaScript key-enumeration API. No public reference or managed
Promise payload boundary changes. Modules without spread keep their prior output.

## Logical operations and representation

`ManagedValue` currently carries Undefined, Null, Boolean, Number, String,
object/array references, function identities and the represented intrinsic Error.
`ManagedObject` denotes a known ordinary object, and
`ManagedArray` a known array. Each has one traced WasmGC reference lane rather than
a numeric handle into linear memory.
Known Number computations still use f64, and the existing `General` primitive pair
is separate. `ManagedPrimitive` bridges its four canonical primitive kinds;
Error-enabled modules translate their Error identity into its distinct GC kind.
Async Error tags cannot be copied verbatim, and Promise tags remain unsupported.

The immutable value carrier has a kind, numeric payload and nullable GC reference.
Ordinary objects have mutable GC-backed property storage. Boxing an object keeps
the object's identity: two carriers for one object compare equal, while fresh
objects remain distinct. Primitive comparisons retain kind differences, NaN and
signed-zero rules. String equality compares lossless UTF-16 code units, never
carrier/array identity or Unicode-normalized text. Future BigInt equality also needs value comparison;
Synchronous private functions have the identity bridge described below. Broader
function-object behavior, Symbols and other object kinds remain follow-ups.

| Operation | Contract |
| --- | --- |
| `ManagedString` | Construct an immutable String value from lossless UTF-16 units. |
| `ManagedObjectNew` | Allocate a fresh ordinary object. |
| `ManagedPrimitive` | Copy a canonical four-kind primitive into an immutable managed carrier. |
| `ManagedFromObject` | Wrap an existing object without changing its identity. |
| `ManagedFromClosure` | Box an existing synchronous function identity, retaining its traced environment. |
| `ManagedGetOwn` | Snapshot one own data-property value, or Undefined if absent. |
| `ManagedSetOwn` | Replace/create one own data property while preserving object identity and prior value snapshots. |
| `ManagedEqual` | Strict equality in the currently admitted managed domain. |
| `ManagedTruthy` | ECMAScript truthiness for the currently admitted managed domain. |
| `ManagedIsNullish` | True only for Undefined or Null, independently of truthiness. |
| `ManagedTypeof` | Inspect a represented value and return its language type as a String. |

## Array backend foundation

The private IR also has `ManagedArray`, a traced reference with a mutable unsigned
32-bit logical length and a private ordinary-object property holder. It reuses the
existing sparse property kernel rather than allocating an element for every index.
This permits an empty array of length 2^32-1 and a write at index 2^32-2 without
storage proportional to length. The extra property-holder allocation is a simple
initial implementation, not a permanent allocation strategy.

`ManagedArrayNew` creates holes, `ManagedArrayLength` returns the length as Number,
and `ManagedFromArray` boxes the existing identity as the same Object kind used for
ordinary objects. Empty arrays are truthy; reboxing preserves identity and fresh
arrays remain distinct from each other and ordinary objects. Typed internal helper
arguments/results can carry array references.

`ManagedArrayGetOwn` and `ManagedArraySetOwn` use UTF-16 keys and immutable value
snapshots. Reads of `length` produce a managed Number. Canonical decimal indices
0 through 2^32-2 grow length when necessary; keys such as `01`, `-0`, `1e2` and
`4294967295` are named properties and do not. Missing entries are holes, distinct
in storage from explicitly stored Undefined. Both read as Undefined through these
own-property operations.

`ManagedArrayRest` copies the remaining intrinsic-iterator values from an unsigned
start position into a fresh dense array. Holes become own Undefined entries,
named properties are omitted, and fresh property cells retain the original value
references. The iterative helper takes linear work in the result length plus
the source's stored property count. Binding lowering separately preserves an
already exhausted iterator by allocating an empty result.

`ManagedArrayResize` accepts an already validated immediate `u32` length. Shrinking
removes indexed own properties at or above that length while retaining named
properties; growing creates holes and does not restore deleted entries. Generic
property writes to `length` reject in the IR verifier so they cannot bypass length
validation. Dynamic coercion, RangeError completion, property descriptors and
prototype lookup are not implemented by these operations.

The source cursor now supplies the receiver/key/length proofs described below.
Public reference exports and suspended async frames retain their existing guards.
Synchronous captures use the traced closure storage described below. Compiler
tests execute emitted IR in Node against native JS; runtime kernel tests exercise
Wasmtime,
holes, sparse storage, cycles, text edges and repeated forced copying collections.

### Appending to source-proven Arrays

`Array.prototype.push` appends source values to an existing, source-proven
Array. The receiver is retained before any arguments; each argument is evaluated
exactly once in order, and the resulting values are appended **after all**
argument effects finish. The returned Number is the final Array length.
A call with no arguments reads the current length without modifying elements.
Sparse holes and aliases keep their existing semantics; Number, Boolean,
Null, Undefined, Strings, objects, Arrays and closure values use the current
managed-value carrier. A dedicated GC Array append helper rejects maximum-length
overflow before mutating storage.

This first source profile admits only the built-in Array method, with static
or literal-computed keys and at most 1,000 fixed arguments. Nested fresh
Array-literal spreads (`xs.push(...[one, two])`) are expanded without a new
iterator or synthetic AST; arguments and their effects retain source order.
Arbitrary array-like receivers, overridden methods, dynamic iterables or
array-variable spreads, and catchable RangeError at the maximum possible length
remain outside the contract.

### Prepending to source-proven Arrays

`Array.prototype.unshift` prepends zero or more values on the same Array and
returns its new length. The receiver is captured first, and all argument
expressions (including nested fixed Array-literal spreads) run left to right
before indexed properties change. In the admitted own-data Array domain,
the compiler reuses sparse-preserving reverse and checked append operations
with reversed argument order, then restores the Array's orientation. Holes
remain holes, non-index properties retain their keys, and aliases retain
the same Array identity. An empty argument list simply reads the length.

As with `push`, this slice requires a source-proven built-in Array
receiver and at most 1,000 fixed arguments. Borrowed methods, overwritten
intrinsics, general iterables and catchable length-overflow RangeErrors are
outside the contract; unsupported programs must not be treated as admitted.

## Ordinary-object property storage

Property keys are lossless UTF-16 vectors, including NUL and lone surrogates.
The initial implementation compares GC-backed packed key arrays and stores entries
in a GC array. Replacing an existing entry preserves its position; appending a new
entry copies the reference array and updates the object's storage. This deliberately
simple implementation has linear lookup/append costs. It is not a chosen long-term
shape/cache/hash-table strategy or an ECMAScript enumeration implementation.

The ordinary-object operations are explicitly **own data-property** operations.
They do not implement JS prototype lookup, getters/setters, proxies, arbitrary prototype chains or
array exotic behavior. Future source lowering must implement the necessary semantic
operation or reject a case; it must not translate arbitrary `o[k]` into own-only
lookup and silently discard inherited behavior.

## Synchronous source admission

Fresh object literals admit ordinary initializer/shorthand properties with static
identifier, String and numeric keys. Computed Boolean and null literals select
the own names `"true"`, `"false"` and `"null"`. A substitution-free template
key uses its cooked UTF-16 units, including escapes and lone surrogates. A computed definition can also use a
represented runtime primitive, including a mixed String/Number name. The key evaluates once before its value; a later
definition with the same key updates the existing position. When the possible
names are known, each candidate field includes the value it could receive;
unrelated fields retain their types. A single possible name defines a definite
field, and a later definite definition replaces earlier alternatives. A computed
`["__proto__"]` defines an own data field. A shorthand `__proto__` does the same. The colon form `__proto__: null`
creates a null-prototype dictionary, including when a sequence ends in literal
null. A proven non-null primitive setter value leaves the ordinary prototype
in place. Setter operands execute in source order. Reference-valued prototypes
remain guarded. Values can contain
the four primitive kinds, Strings or other constructed objects and arrays. Properties evaluate left to
right; duplicate keys retain the last value. Static reads/writes preserve aliases,
receiver snapshots, RHS results and prior saved reads. Known-object bindings can be
reassigned to other known objects, and managed property snapshots can hold changing
value kinds. Conditional references, strict equality and truthiness retain their
source behavior. Type annotations do not select runtime kinds.

For example, this executes through the existing numeric interface:

```js
export function run(a, b) {
  let o = {x: a};
  const alias = o;
  const old = o.x;
  o.x = b;
  o = {x: a};
  return alias !== o && old === a && alias.x === b ? a : b;
}
```

All admitted object references originate in fresh literals and cannot enter through
the numeric host arguments or escape through a reference export. Prototype mutation
is unavailable. To keep own-property operations faithful, **every intrinsic
Object.prototype key rejects in ordinary member access**, even when a computed
literal has defined a shadowing own field. Own-property presence, enumeration
and rest can observe such a field. The guard is
checked against the actual native intrinsic names in tests. Other absent static
keys return Undefined. This temporary admission boundary is not a chosen permanent
object model or a replacement for prototype-aware lookup.

Ordinary-object reads still require a static key and a known receiver. Optional
access, accessors and literal method definitions remain outside this profile. Implicit managed coercions, loose equality
and reference-aware async frames remain separate work. Managed properties cannot
be returned directly through the primitive ABI; observations must produce a supported primitive result.
Known typed function values in managed containers and calls through proven
compatible own fields are supported as detailed below; opaque dynamic calls
remain separate work. String methods, indexing and mixed-kind coercions
also remain separate work. Unsupported branches
are checked even when a particular input would not execute them.

String literals, including empty text, NUL, non-BMP characters and lone surrogates,
flow through traced local snapshots, properties and ordinary helper arguments/results.
Strict equality uses content; only the empty String is falsy. Properties and local
stores replace carriers without changing a previously saved String value.
The legacy linear-memory String/JSON wire representation remains a separate
boundary: its tags/pointers cannot be reinterpreted as this GC value carrier.
Exported String results and general JSON composition need explicit conversion work.

Packed UTF-16 arrays are mutable only during private literal construction. Short
literals use `array.new_fixed`; above V8's 10,000-element instruction limit they
use a private traced temporary and array initialization. Neither property keys nor
String contents can be mutated by admitted source. This fixes large static keys
through the same construction path. Source-size and production fuel limits are
unchanged. Large semantic probes use an explicit bounded core-engine fuel budget
and separately assert controlled exhaustion at the public runtime's normal budget.

## Source String length

Known String receivers admit `.length` and static `['length']` reads. The result
is a Number counting UTF-16 code units: an astral character occupies two units,
and a lone surrogate or NUL occupies one. The receiver executes once through the
ordinary expression cursor. Saved values keep their original length after a
local or property is replaced; captured bindings observe the current value.
The same operation works with admitted helper results and values retained across
primitive awaits.

The existing finite source-flow facts now distinguish String origins from other
boxed values. Admission requires every receiver alternative to be a String;
TypeScript annotations and assertions do not establish that proof. The backend
reads the traced UTF-16 payload through a checked private projection, with no
new allocation or host call. Unknown or mixed-kind receivers, dynamic property
keys, optional access, and String property writes remain outside this slice.
Native JS, Node-Wasm and Wasmtime tests cover code-unit counts, source effects,
helpers, captures, snapshots, erased types and async settlement traces.

## String concatenation

Binary `+` concatenates when either operand is a known String and the other is a represented primitive.
Both operands execute once, left to right; the saved left value survives a write
or primitive await in the right expression. Concatenation creates a fresh UTF-16
payload and preserves empty text, NULs, surrogate pairs and lone surrogates
without normalization. Results compose with `.length`, equality, truthiness,
local/property storage, admitted helper signatures and captures.

Object conversion and addition that can choose between numeric and String results
remain guarded. String `+=` over mutable locals and
computed or static own data references uses the same concatenation operation as described above.
The private helper is included only when the module needs it. It checks the sum
in an unsigned 64-bit lane before allocating the Wasm32 array; overflow and
engine allocation exhaustion are resource failures, not catchable source
exceptions. A private Wasmtime test exercises arithmetic boundaries without
allocating enormous arrays. Source differential tests compare operand effects,
saved values, erased annotations and async results/traces with native JavaScript.

## Own-property presence

`Object.hasOwn(receiver, key)` tests whether a supported receiver has the requested own key, including when the stored value is Undefined. This
also works for inherited-looking names: an ordinary empty object has no own
`constructor`, while a dictionary can explicitly own that key. An array owns
its non-enumerable `length`; holes are absent and a slot set to Undefined is
present. The guest checks the existing property table directly.

The call saves its receiver and key in source order and tests presence after
all surplus arguments have executed. Later arguments may mutate the original
receiver. Direct calls, literal member names and the existing `.call` form
share this behavior across helpers and supported suspension. The result is a
Boolean. Keys can be proven Strings or statically determined primitive names. Numeric
literals reuse the JavaScript spelling used by member access, including
`-0` as `"0"`. Boolean and null literals use their keyword names, and an
omitted key, the intrinsic Undefined value or a `void` expression uses
`"undefined"`. The operand of `void` still runs at the key's position,
including across an await. Other runtime coercions need their own semantics.
A proven String owns `length` and the canonical indices of its UTF-16 code
units. Surrogate halves occupy separate positions. Noncanonical numeric text,
such as `"01"` or `"-0"`, has no corresponding character property. Known
Number and Boolean primitives have no own properties, and their operand and
argument effects still execute. Nullable or otherwise unproven receivers
require additional handling. The semantic reference is
[Object.hasOwn](https://tc39.es/ecma262/multipage/fundamental-objects.html#sec-object.hasown).

## Property membership

For a proven null-prototype dictionary, `key in dictionary` checks presence
even when the stored value is Undefined. The left operand is evaluated and
saved before the right operand. A change to the key variable during receiver
evaluation leaves the saved name intact. A failure in the left expression
prevents receiver evaluation. The supported keys match the known Strings and
static primitive names used by `Object.hasOwn`. On ordinary objects and arrays, static key names also admit membership tests.
A name inherited from the known intrinsic prototype is present even when
there is no own property. For other static names the own table decides, so
an array's length, holes and named properties keep their usual behavior.
In both cases the RHS still evaluates. For a possible mix of ordinary and
null-prototype objects, only keys with no inherited behavior share that
specialization. A proven runtime String key on an array or an object with a known ordinary
prototype first tests the own table and then the corresponding intrinsic name
sets. Each required intrinsic name set is a private packed WasmGC table created
once per instance. Guest comparisons select its bucket by UTF-16 length and
compare code units without allocating prototype keys during a lookup. A dynamic key on a possible mix of
ordinary and null-prototype records still requires more runtime information.

## Synchronous source arrays

Array literals construct traced references on the same expression cursor as
objects and primitive values. Elements evaluate left to right; holes remain absent
entries and trailing commas retain their JavaScript length. Arrays can contain
the four existing primitive kinds, Strings, objects and other arrays, including cycles. Aliases,
rebinding, saved element reads and the receiver snapshot before an assignment RHS
retain their identity/value semantics. Empty arrays are truthy.

Known-array reads and plain writes accept static String and numeric literal keys.
Computed reads, including optional reads and object binding patterns, also accept
a runtime String when every possible name is known and misses the inherited
intrinsics. For example, choosing between `"length"` and `"0"` reads the
current length or the first element. Named properties and holes use ordinary
array lookup. A computed write still requires its separate write proof.
Numeric property names use the JavaScript spelling of the parsed Number, also in
record literals, bindings, dictionary accesses and object-rest exclusions. For
example, `0x10` names `"16"`, `-0` names `"0"`, and `1e21` names `"1e+21"`.
Only a canonical array index below 2^32-1 can grow the array. A negative,
fractional or larger numeric key selects a named property. `length` reads, including computed static text, produce
known Numbers and preserve existing numeric layouts. Plain length writes accept
valid unsigned integer literals through 2^32-1, retain the assigned Number result,
remove truncated elements and leave holes on subsequent growth. Erased TS wrappers
and parentheses are checked under the existing expression-depth budget.

Property reads and writes reject every Object.prototype intrinsic key and every
Array.prototype String key except the array's own `length`, including shadowed
keys. Direct `at` and `includes` calls have checked specializations.
Tests compare this boundary with the native intrinsic names. The additional Array
guard does not restrict ordinary objects' own keys such as `map` or `push`.
Prototype mutation and foreign references remain unavailable in this profile.

`array.includes(needle, fromIndex)` tests membership in a proven managed
Array. It uses SameValueZero, so NaN matches NaN and signed zeroes compare
equal. A hole compares as Undefined, and referenced objects compare by
identity. The optional starting position admits Numbers and explicit
Undefined. The receiver and needle are saved before later argument effects,
and the search observes the resulting length and contents. The guest searches
the stored entries, so a sparse array's logical length does not determine the
number of iterations.

`indexOf` and `lastIndexOf` return the first or last matching numeric
position, with -1 for a miss. They use strict equality and skip holes.
Stored properties can be out of insertion order, so the search selects the
lowest or highest matching index in the requested range. An omitted
`lastIndexOf` position starts at the final element. Explicit Undefined or
NaN starts at zero. Argument snapshots and sparse traversal use the same
machinery as `includes`.

A single spread literal `[...items]` makes a shallow, dense copy of a proven
Array. A proven String operand produces a dense array of the Strings yielded
by its intrinsic iterator, including two-unit supplementary characters and
one-unit unpaired surrogates. Each hole in an Array operand becomes an own
Undefined element. Named properties are
omitted, and copied references retain identity. The operand runs once,
including when it suspends. Indexed and iterated values follow the source's
element origins and include writes through aliases of the new array.

Mixed literals with spread, descriptors, general key coercions, invalid
or dynamic length writes and optional access still reject. Element snapshots use
the general carrier: returning `items[1]` directly through a Number export still
requires a primitive-result proof/projection, and exporting an array requires a
reference-aware boundary. Those existing gap fixtures remain rejected; construction
and equality tests do not claim those gaps are closed. Array values can survive
supported async suspension through the existing traced value lanes.

### Runtime element selection with `at`

Known arrays support [the `at` operation](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.at)
through `array.at(index)` and `array['at'](index)` with a Number
index; omitting it selects zero. Indexing follows Number `ToIntegerOrInfinity`:
fractions truncate toward zero, NaN and signed zero select zero, negative indices
count from the current length, and out-of-range positions and holes return
Undefined. Large sparse lengths do not allocate dense storage.

The receiver and index are evaluated once in source order. Surplus argument
effects run before the operation reads the array's current length and element.
The selected value is an immutable boxed snapshot, preserving object/array/function
identity and String/primitive values. It composes with supported helpers, loops,
`Object.keys`, and async suspension, using the existing traced value lanes.

Selection from a source-known array retains its possible element origins plus
Undefined. A compatible binding default can therefore establish a String, object,
or array receiver for later operations, including String methods on a selected
record field. These facts flow through ordinary helpers, captures, and supported
array copies. Defaults remain lazy and replace only Undefined. Every selection
keeps the missing alternative, even with a literal index; this is not an
in-bounds proof. Mixed element kinds, incompatible alias writes, holes and
uncertain structural mutations remain conservative and can prevent a typed
consumer after a default. Dynamic calls and boxed numeric projection do not gain
new admission from these facts.

This specializes the intrinsic only for known arrays: admitted source cannot
replace their `at` property, mutate their prototypes or import a foreign array.
Ordinary objects with their own callable `at` retain the ordinary call path.
Non-Number index coercion, borrowed `.at.call`, optional calls, dynamic iterable spreads,
generic computed properties and arbitrary boxed-element numeric arithmetic remain
separate work. TypeScript annotations do not widen this admission.

The private `ManagedArrayAt` operation takes a checked Array and Number and
returns a managed value. The Wasm helper reuses sparse array lookup and the
existing unsigned decimal key formatter; no host operation or public ABI changes.
The existing `managed-arrays` and `async-suspension` suites compare native
JavaScript, Node-Wasm and Wasmtime results and argument order.

## Short-circuit values and defaults

Source `&&`, `||` and `??` select actual managed operands through the existing
ordered expression cursor and typed branch regions. The left operand is evaluated
once and retained as a snapshot. The right operand is validated as source in all
cases, but runs only when selected. Objects are truthy; String truthiness uses its
length; `??` tests only Undefined/Null, preserving empty text, false, zero and NaN.
`ManagedIsNullish` produces a predicate from the managed language-kind tag.

Nullish results also retain their source receiver origins. An optional String,
record or array selected by `at`, a field read, helper or captured callback can
use a compatible `??` fallback and then feed a String method or container read.
Unlike a binding default, `??` replaces both explicit Null and Undefined.
Definitely non-nullish left operands keep their own origins without acquiring
those of the unselected fallback; the fallback still undergoes source validation.
The analysis retains every unknown or other non-nullish alternative and all
alias writes. It does not infer that a selected element exists, narrow an unknown
Number/Boolean into a String, or detach a selected child from later field writes.

Logical `||` and `&&` use the same result-origin query. Fresh ordinary objects,
null-prototype dictionaries, arrays and represented functions are definitely
truthy; explicit Null and Undefined are definitely falsy. A nullable String can
therefore use a String `||` fallback, and a known reference can select another
receiver with `&&` without retaining the discarded operand's origin. Strings may
be empty and other primitive or unknown origins retain both possible selections.
The result keeps selected object identities and alias writes. This refines the
expression result, not the types of bindings inside its right-hand branch: a
nullable receiver read there still needs an existing positive receiver proof.

Logical results work in data properties, local bindings and ordinary helpers,
including mixed primitive/reference helpers and discarded expressions. Static
Number-only arguments retain f64. A Boolean/nullish argument to a helper that also
transports managed values uses the existing carrier, since ordinary source helper
parameters exclude the `General` pair. Boolean/nullish helper results in numeric
entry profiles use the same carrier when needed to retain their kind; other entry
profiles retain the original primitive-result planning. Metadata supplies no kind
proof. This does not add primitive projection from arbitrary managed results.

The existing traced suspension path carries these selected values across awaits,
including a lazy fallback that itself suspends. Managed logical assignments,
opaque/dynamic function calls and public managed exports remain outside this
increment. Known typed function values
in the carrier are covered by later source increments. Unsupported
operations in an unselected branch remain rejected.

## Source type inspection

`typeof` inspects represented Undefined, Null, Boolean, Number, String, object and
array values. Results use the existing String carrier: null reports `object`,
arrays report `object`, and NaN/infinities remain `number`. Property snapshots,
missing properties, local bindings, selections and ordinary helper arguments and
results follow the same expression cursor. Operands evaluate once, including
updates and comma effects; discarded results retain those effects.

Known typed closures are evaluated without invoking the resulting function and
report `function`. A directly referenced hoisted module function has no read or
construction effects, so its result can be constant text; its complete body still
undergoes normal validation. Symbol identity and initialization checks preserve
shadowing and reject TDZ reads. Erased annotations supply no runtime kind facts.

In the current closed source environment, `typeof` on an unresolvable identifier
reference returns `undefined` text without reading that absent binding. Parentheses
and erased TypeScript wrappers preserve this reference behavior. A resolved local
still requires initialization; `typeof (0, missingName)` still evaluates its value
and rejects that missing read. Intrinsic/global names and common host namespaces
keep their admission guards, even when their value operations are unimplemented;
semantic lexical non-resolution alone does not prove that a built-in is absent.
The exact original missing-name fixture now executes unchanged. General ambient
global lookup and actual TDZ exception execution remain open. BigInt/Symbol operands, dynamic
`Function` construction, Error/Promise carriers from other protocols, mixed
function/primitive values, public String exports and managed async frames are not
added here. Rejection tests establish those boundaries, not executed language
semantics for those cases.

## Calls, cells and lifetime boundaries

Ordinary synchronous named source helpers can accept/return managed references,
including arrays, String values, mixed primitive/reference arguments and property-value results. A finite
source-flow query joins actual arguments, local initializers/writes and returns by
resolved symbol identity until no kind facts change. Object-only flow uses a known
object reference and array-only flow a known array reference; String-bearing or mixed flow uses the managed carrier. TS type
annotations supply no runtime facts. Existing primitive planning remains responsible
for calls with no object-bearing flow, so ordinary numeric helpers retain f64.

For example, `function set(o,v){o.x=v;return o;}` can mutate a caller's object and
return the same reference. Forwarding, mutually recursive helpers, fresh-object
results, mixed-value identity functions and `.call` use the same typed transport.
Ignored `thisArg` and surplus arguments still evaluate in source order. Managed
private closure signatures and async helper frames are not admitted by this increment.
The flow query does not establish source validity: unknown-kind property receivers,
unsupported operations and every source branch still undergo ordinary validation.
A helper seen with both object and numeric callers cannot silently treat all inputs
as objects. Function aliases and public reference exports remain unsupported.

Logical argument checks use the callee's typed slots, and the Wasm header uses the
shared backend layout. Existing ordinary Number calls retain f64 signatures. Exported
inputs/results remain in the existing admitted profiles. Source private-closure
planning still rejects managed arguments/results; typed IR parameters are described
below.

Uncaptured source cells and computed-value snapshots use traced locals. A cell
store changes the binding, not an earlier loaded snapshot. The private IR now
supports traced captured cells and closure environments as described below. Source
captures now use that storage; async spill migration remains a separate step and
managed modules needing traced closure or multi-coroutine storage still reject
explicitly. Single-frame suspension uses the rooted globals described above.

The object/String GC kernel is emitted only for modules using managed kinds and
appended after source functions. Closure storage types precede closure function
types and add no functions. This keeps source call indices stable. Existing
numeric/primitive modules do not acquire GC types or runtime helpers merely from
this implementation being available.

## Traced closure backend

The synchronous private IR admits captured `ManagedValue`, `ManagedObject` and
`ManagedArray` cells. A module with a managed captured cell or a managed private
closure parameter uses one GC cell domain for all its captured bindings and
closures. This matters when both numeric and managed closures refer to the same
mutable binding: both environments must
retain the same cell. A load still creates a value snapshot; storing a new reference
updates the binding without changing an earlier loaded value.

Each cell has unboxed i32/f64 fields and a traced reference field. An environment
is a GC array of cell references. A private GC function object contains a table index
and a traced environment reference; typed closure values retain this one identity.
Call arguments use fresh cell entries in a GC
frame array. Closure parameters/results retain their existing logical contracts,
including primitive pairs and returned/callback closures. Private synchronous IR
parameters also admit known object/array references and the managed value carrier,
with exact argument types and counts. Managed results use the typed path below;
asynchronous managed parameters remain rejected. Source inference is covered separately.

### Typed managed closure arguments

The backend's traced argument entries already store the reference lane. The IR
verifier now admits `ManagedObject`, `ManagedArray` and `ManagedValue` as synchronous
private closure parameters, including callback and returned-function signatures.
GC closure storage is selected even when such a function captures no values.
Argument count/type checks, parameter initialization, exported numeric inputs,
public-result guards and asynchronous frame rejection remain unchanged.

Maintained emitted-IR/native-JS/Node tests cover alias mutation and prior snapshots,
arrays, lossless String and Number carriers, nested capture of a managed parameter,
callback/returned-function signatures and the 1,000-argument boundary. These
directly test the backend; source argument inference is covered separately below.
No new physical frame format, public reference ABI, dependency or workflow is introduced.
A separate manual run executes an actual emitted object-argument artifact through
the production Wasmtime numeric adapter over 64 input pairs; temporary capture
instrumentation is removed afterward. This does not establish source admission.

### Typed managed closure results

Synchronous private IR closures can also return `ManagedObject`, `ManagedArray`
or `ManagedValue`, including through callback and returned-function signatures.
Logical signatures and inline function-body regions still match the exact result
type. The inline-body gate admits these existing managed types; awaited inline
calls retain their separate General-only result check. The private Wasm call uses
one nullable `eqref` result, then casts to the IR-proven reference type before
assigning the caller's typed local. Objects and arrays retain identity; String and
primitive carriers keep their language kind. No numeric handle or byte spill is
introduced. A managed result selects traced closure storage even without managed
parameters or captures. Number and General result paths are unchanged; GC function
results use the canonical identity described below.

A separate physical function type for each managed result was considered. The
shared reference lane avoids forward references into the later managed type group
and preserves existing type/function ordering for modules without this feature.
IR validation retains the precise logical contract; this does not permit forging
or interchanging object, array and carrier result signatures.

`ir::gc_closure_result_tests` executes emitted modules against native JS in Node.
It covers alias mutation, array length, lossless text, numeric/boxed carriers,
callbacks, returned functions, fresh factories and argument/type/async boundaries.
A manual verification runs an actual emitted factory-result artifact through the
production Wasmtime numeric adapter over 64 pairs; temporary capture instrumentation
is removed afterward. That probe does not force collection or establish source
result inference, a public reference ABI or suspended managed frames.

GC types precede explicit closure function types so type references do not cross
forward into a later recursion group. The source function indices remain stable.
Modules without managed captured cells, managed closure parameters/results or function boxing retain their
existing numeric arena, closure layout and async behavior. Modules using the traced
path need no numeric arena reset wrapper. Ordinary Number arithmetic and locals retain f64 in both paths.

Synchronous source closures now use this storage for captured objects, arrays,
Strings and managed property snapshots, including shared rebinding and mutation.
Selected closures, callbacks and supported factories transport managed arguments
and results alongside the existing primitive and typed function lanes.
Capture pruning still validates
unused bodies and excludes erased metadata/unreferenced cells. Managed values
cannot enter the old async byte-spill frames. Traced suspension, general function
values in containers and public reference ownership remain follow-up work. This
implementation does not add a shared heap
across runtime instances or promise reclamation latency/throughput.

Maintained typed-IR tests execute actual emitted modules in Node against independent
native-JS observations. They cover object/array/String rebinding, writes from a
closure, primitive and callback lanes, nested returned closures, per-iteration cells,
the 1,000-argument boundary and 70,000 calls beyond the legacy bump-arena capacity.
The runtime test executes the shared storage type fragment in Wasmtime and forces
copying collections before and after alias mutation, checking a cyclic environment
and saved object/String snapshot. That hand-built lifetime fixture is not evidence
of source capture admission. A separate manual verification executes an actual
emitted managed-capture IR artifact through the production Wasmtime numeric adapter;
it does not replace the maintained lifetime fixture or establish source admission.

## Canonical private function identities

In the GC profile, each `MakeClosure` allocates one immutable function object with
its code index and traced environment. Typed closure snapshots, cells, arguments
and results carry that reference. `ManagedFromClosure` adds an immutable carrier
around the same reference; aliases and repeated boxing remain equal, while two
creations of the same code are distinct even without captures. The carrier uses
private tag 6, with `typeof` equal to `"function"`, truthiness true and nullishness
false. This tag is separate from the primitive wire and async protocols.

Allocating a new function object on each boxing was rejected: it would split
identity and complicate future properties on the function itself. Allocation at
creation retains one object for those later operations. Existing non-GC numeric
closure pairs keep their prior representation. GC calls still pass the same typed
environment and argument frames; Number arithmetic and results remain unboxed.

Maintained emitted-IR/native-JS/Node comparisons cover reboxing, fresh creations,
object/array storage, saved property values, type inspection and subsequent typed
calls. Validation requires a visible known synchronous closure and a managed
result. The source GC observer now retains the actual generated function object
after defining calls return. A separate shared-fragment Wasmtime test keeps a
boxed function/environment/object cycle across forced copying collections and
checks a cleared root. Neither test claims a public reference ABI or reclamation
timing. The source increment below connects known function values to containers;
arbitrary calls through stored values remain separate work.

## Checked container projections in the IR

`ManagedToObject` and `ManagedToArray` recover the same ordinary-object or array
reference from a boxed value. The verifier requires a visible `ManagedValue`
input and the exact typed result. Both carriers use language tag 4; the emitter
therefore checks that tag **and** casts to the matching nonnull `$gc_object` or
`$gc_array` payload. A tag alone cannot distinguish their different storage.
Reboxing preserves identity, and typed property writes, reads and array length
operations continue to use the original container. There is no new allocation or
layout, and numeric-only artifacts do not acquire a GC dependency.

Wrong kinds, wrong carriers and null payloads trap at this internal boundary.
This does not implement JS TypeError completions or authorize unknown source
receivers. Source admission uses the exhaustive receiver proofs described below;
a generic IR lane alone does not prove a particular receiver kind. GC references
in suspended async frames still require their own work.

Maintained emitted-IR/native-JS/Node tests cover nested object/array storage,
saved children after replacement, identity and mutation, sparse array lengths,
and projections at actual generated helper/closure receiver boundaries. Negative
tests distinguish valid language values of the wrong kind from deliberately
malformed tag/payload controls. Exact WAT/Wasm and oracle expressions can be
retained with `GRUNKALUNKA_IR_TEST_ARTIFACTS` for Wasmtime replay. The tests retain
2 MiB native-stack traversal and explicit numeric async-frame rejection.

## Checked function projection in the IR

`ManagedToClosure` projects a boxed managed value to a checked synchronous
`Closure(signature)`, preserving the original function object and its traced
environment. Wrong language kinds and mismatched complete signatures trap at this
internal projection. The verifier requires a visible managed input and a valid
synchronous result signature. This is not a JavaScript exception boundary.

GC call frames erase logical parameter types into shared cells. Checking only the
core-Wasm indirect-call type would permit incompatible Number/General parameters.
The emitter therefore generates an on-demand guard over table targets whose full
parameter/result signature matches. Existing signature-registry invariants make
those IDs canonical. The check does not allocate another function, change the GC
layout, or affect artifacts without the new operation.

Focused IR tests cover calls, identity, captures, rich signatures, snapshots,
wrong-kind/signature traps, malformed inputs and 2 MiB native-stack traversal.
`GRUNKALUNKA_IR_TEST_ARTIFACTS` can retain their exact WAT/Wasm and native oracle
expressions for Wasmtime replay. Source calls through known own fields use this
operation as their checked backend bridge.

## Calls through known own fields

Source calls such as `o.f(a)` and `items[0](a)` can recover a typed function from
an own data property. An iterative original-source analysis gathers allocation
sites, aliases, field writes, known call arguments and returned constructors.
It admits a call only when every gathered alternative is a synchronous function
with the same complete parameter/result signature. Positive constructor facts
also feed existing callback and managed-value planning; those facts alone never
authorize a call or erase an unknown/nonfunction alternative.

The field must be present at construction. Missing fields, holes, incompatible
writes, and array shrinking that could delete the element invalidate this proof.
The analysis merges calls and writes conservatively, so some valid programs with
later field initialization or heterogeneous values still reject. Dynamic keys,
prototype behavior, async targets and unknown-kind receivers remain unsupported.
Proven nested receivers use the checked recovery described below.

Lowering evaluates the receiver, reads the actual field once, and applies
`ManagedToClosure` before the existing typed call. The analysis supplies a
signature, never a replacement runtime callee. Saved functions, argument-time
overwrites, explicit `.call` receivers and surplus argument effects therefore
keep their original evaluation order and identity. Known primitive lanes remain
specialized. No universal dynamic call ABI or JS TypeError handling is implied.

Ordinary maintained tests and canonical native-JS/Node-Wasm/Wasmtime fixtures
cover object/array calls, callbacks, factories, managed inputs/results, erased TS
annotations and negative alternatives. A generated function reached through a
property retains cyclic captured state after its defining call returns and
across forced Wasmtime copying collections. Native-stack regressions retain
the 2 MiB limit.

## Nested source receivers

Objects and arrays read from own fields, saved in aliases, or passed through
known helpers and closures can act as receivers when every original-source
alternative has the same container kind. This supports nested reads, writes,
array lengths/resizing and function-field calls while preserving reference
identity, captures and property snapshots. Constructor fields, aliases, known
formal/actual/result edges and writes share the existing finite source relation.
Logical function assignments retain replacement factories even when their
generic result signatures agree; those factories can return different kinds.
An unknown, missing, noncontainer or mixed object/array alternative prevents the
proof. Later initialization of an absent constructor field remains conservative.

Representation and receiver proof are separate. Property reads, generic slots
and helper lanes keep `ManagedValue`; they do not become typed object lanes just
because a consumer needs a receiver. The iterative cursor reads the actual value
once and applies `ManagedToObject` / `ManagedToArray` at that use. Already typed
references retain their existing path. This preserves receiver-before-RHS and
callee-before-argument snapshots, explicit `.call` offsets and surplus effects,
without changing helper signatures or choosing a runtime target from metadata.

Maintained source/native-JS/Node-Wasm/Wasmtime fixtures cover all object/array
combinations, aliases, factories, captures, cycles, replacement during assignment
or call arguments, TS erasure and source bounds on a 2 MiB native stack. Generated
functions reached through nested receivers retain captured cyclic state across
forced collections after their defining call returns. Public reference exports,
prototype/dynamic-key behavior and GC async frames remain separate work.

## Source function values in managed storage

Known typed source arrows can be stored in object/array properties, compared by
identity and inspected with `typeof`. They can coexist with other managed values
and different function signatures. The original typed bindings remain callable;
a property containing an opaque managed function is not yet a callable source
target. Known uniform fields use the admission above; module declarations use
the identity adapters described below. General changing-kind binding/signature
inference remains separate work.

Identity demand follows the existing finite constructor graph through aliases and
known callback/factory arguments. An observed immutable function binding is
materialized once at its definition, including already-evaluated factory metadata.
Reads load that same reference; factory prefix effects are not repeated. Unobserved
primitive functions retain the existing static path. This avoids both per-read
recreation and a lazy cache whose first read might occur in only one branch.

`managed-function-values` compares JS/TS in native JS, Node-Wasm and Wasmtime,
including fresh captureless/captured functions, aliases, reassignment, snapshots,
factory effects, heterogeneous arrays, cycles and erased annotations. A test-only
root retains an actual generated function carrier and its object/environment cycle
after source calls return, then forces copying collection and calls the saved
function. Clearing the root must trap. This establishes reachability, not measured
reclamation or a public reference ABI. Suspended function carriers still reject.

## Source managed closure arguments

Known synchronous arrows now infer object, array and String/carrier parameters from
actual argument flow. The existing constructor graph supplies local, alias,
selected-callback and supported factory call edges; a finite value-kind union also
accounts for assignments and corresponding formals of selected constructors.
Unresolved facts start empty rather than inventing a Number origin. Object-only and
array-only inputs retain their specialized lanes, mixed managed/primitive values
use the managed carrier, and Number-only inputs retain their existing path.

The evaluator snapshots actual arguments left to right before binding fresh formal
cells. A returned closure may capture a managed formal, preserving identity and
mutation after the factory returns. TS annotations do not determine these kinds.
`managed-closure-inputs` compares source in native JS, Node-Wasm and Wasmtime,
including callback/factory composition, mixed kinds, argument effects, property
writes, array cycles and erased contradictory annotations. Public managed references,
unknown property receivers, prototype/coercion operations and suspended managed
frames remain explicitly unsupported. No public reference ABI or new frame format
is introduced.

## Source managed closure results

Known synchronous arrows return objects, arrays, Strings and mixed managed values
through the same traced private result path. Input, return and call-kind facts
converge together over the existing constructor graph; selected peers share a
compatible result type. Block fallthrough and bare returns contribute undefined.
Object/array-only results keep their typed references, mixed results use the
managed carrier, and Number-only graphs retain their existing path. The lowerer
borrows one planning context rather than growing its copied native frame with
several maps. TypeScript annotations supply no runtime kind proof.

Actual calls still snapshot the callee before evaluating arguments left to right.
Returning a reference preserves identity and mutation; a saved property read is
still a value snapshot. Maintained `managed-closure-results` comparisons execute
JS/TS against native JS, Node-Wasm and Wasmtime, covering concise/block returns,
callbacks, factories, mixed values, method calls, reassignment effects and erased
annotations. Exact former source-result rejection cases now execute positively.
Public numeric exports, unsupported coercions/prototypes, opaque stored-function
calls and suspended managed frames retain explicit rejection controls.

Admitted runtime function values with managed results or observed data identity can
directly capture a `for (let ...)` header through the existing per-iteration
cell renewal. [#334](https://github.com/tseguine/grunkalunka/issues/334) retains
two unchanged sources: object-returning closures saved from iterations 0 and 2,
and a header-capturing closure stored in an object after the loop. Body-local
snapshots and the numeric iteration-cell regressions remain covered separately.

A test-only observer retains an actual generated private call's returned object
in a traced global without replacing its source functions or call/frame code.
After both the private call and numeric entry return, Wasmtime copying collections
preserve a cyclic reference and an updated property payload. Clearing the test root
makes its observer trap. Unmodified source is independently compared in all three
engines. This adds no public reference export and makes no reclamation-time claim.

## Source capture lifetime evidence

`managed-captures` compares actual JS/TS source against native JS in Node-Wasm and
Wasmtime. It covers captured objects, arrays, Strings and changing-kind snapshots;
shared binding writes, property updates, receiver evaluation order, factory results,
callbacks, lexical iteration cells, shadowing and erased annotations. The exact
former object-truthiness, array-length and returned-array-reader guards execute
unchanged as positive tests. Invalid writes,
unsupported prototype operations and async frames remain explicit rejection tests.

For forced collection after a defining call returns, the lifetime test adds a
**test-only** root/observer to generated WAT. It copies an actual constructed
closure's code/environment pair into traced globals without replacing its generated
functions, cells or environments. After the source entry returns, repeated Wasmtime
copying collections precede calls through that saved closure. Native JS and the
unmodified artifact are compared separately. Clearing the test root releases it;
the test makes no measured reclamation or public-reference-ABI claim.

## Evidence and remaining work

Ordinary compiler tests construct typed IR and execute the emitted module in Node
against independent native-JS observations. They cover all four primitive kinds,
object identity under reboxing, multiple UTF-16 keys, missing values, overwritten
properties, saved snapshots, mixed reference/Number helper parameters and local
binding stores. Malformed IR, exported/private-closure signatures and async
byte-spill boundaries have rejection tests; typed managed captures have positive
backend tests.

Maintained runtime tests also compile JS and TS source and compare native JS,
Node-Wasm and Wasmtime over Number boundary inputs. They cover literal order,
aliases, overwrites, saved property/reference snapshots, receiver/RHS mutation,
conditional reference values, truthiness, UTF-16 keys, source helper reference
transport, recursive forwarding, call effects, misleading annotations and rejection
boundaries.
The canonical compatibility corpus includes source object programs and preserves
the exact formerly rejected object-discard sources as executed evidence.
Small-stack probes exercise both source depth diagnostics and existing constructor
checks without increasing native stacks or source limits.

Runtime tests include the actual shared GC helper fragment and execute it in
Wasmtime. They check Number bits, property snapshots and a self-referencing object
graph through repeated copying collections. String content reachable through
object properties is also checked across forced collections. A separate recorded
verification ran an actual emitted mixed-helper IR artifact in both Node and Wasmtime. That manual
artifact check does not substitute for maintained source differential tests.

Node requires standard WasmGC support; the project already selects Node 24. Use
the pinned Wasmtime copying collector from
[the runtime prerequisite](https://github.com/tseguine/grunkalunka/pull/301).
No new package version, host authority, public reference ABI or dedicated CI
workflow is introduced. Ordinary Cargo tests and existing package dependencies
select the compiler and runtime consumers of the shared helper file.

Broader String operations, declaration identities, opaque stored-function calls,
suspended GC frames, public heap ownership and full coercion/prototype behavior
remain work under the original general-purpose JavaScript goal.
This source increment does not complete the value domain or justify counting
rejected programs as supported.

## Own enumerable keys, values and entries

`Array.isArray(value)` tests the brand of the represented value. Empty,
sparse and resized arrays return true. Records with numeric properties or
`length`, Strings, primitive values and represented functions return false.
The checked value is captured before surplus arguments run. The unshadowed
intrinsic admits literal-computed access and the supported `.call` form.
The result is a Boolean and supplies no branch-local type refinement.

`Object.keys` builds a fresh managed array from the currently represented
data-only domain. Objects and sparse arrays enumerate existing own data entries:
canonical array-index strings first in ascending numeric order, then other
strings in insertion order. Updating a property retains its position; array
`length` is not enumerable. Strings supply UTF-16 index names. Numbers, Booleans,
represented functions and intrinsic Errors supply no enumerable data.
The result snapshots keys and shares immutable string storage without sorting or
mutating the source. Subsequent source/result writes remain independent.
Iteration over the returned names supports String operations such as
`name.trim()`. Indexed reads can miss. A String binding default covers a
missing index, while incompatible writes through aliases invalidate the proof.
This follows [Object.keys](https://tc39.es/ecma262/multipage/fundamental-objects.html#sec-object.keys)
and [ordinary key ordering](https://tc39.es/ecma262/multipage/ordinary-and-exotic-objects-behaviours.html#sec-ordinaryownpropertykeys)
within the admitted domain, not arbitrary exotic objects or descriptors.

`Object.values` uses the same ordering and takes its snapshot at the same point
in argument evaluation. It copies boxed values into fresh slots, preserving
child object identity. Sparse holes and array length are omitted; present
Undefined entries are included. Each String code unit contributes a one-unit
String, including lone surrogates. When the source fields and observed writes
all yield Strings, each iterated value supports String operations. When they
yield known records, loop heads can destructure their fields. Data literals,
null-prototype dictionaries and UTF-16 String inputs participate. Spread
sources and unidentified allocations retain generic values. A fixed outer
index can miss, even when all yielded elements share a kind.

`Object.entries` uses that same enumeration and creates a separate fresh,
dense `[key, value]` pair for each result. Pair slots can change independently
of the input and of other pairs; referenced objects retain identity. A
`for...of` head can destructure the pair and use String operations on its key.
Its value slot carries the same source alternatives as `Object.values`.
Writes to source objects, returned pairs and shared child records are included
in the proof. Replacing rows, overwriting slots, shrinking an array or creating
holes can invalidate it. No fixed outer index
is assumed to exist merely because the source is an entries call.

The frontend recognizes the unshadowed intrinsics, including a literal computed
member and the existing `.call` form. Arguments execute once, left to right;
ignored receiver/surplus arguments execute before enumeration. TypeScript types
remain erased. Known nonnull values use a checked specialization. Unknown or
nullable boxed values use an explicit normal/throw operation in the async profile:
null/undefined, including a missing argument, throw a fresh catchable TypeError.
Existing handlers, finalizers and saved activation roots preserve its identity
and key arrays through suspension and copying collection.

General synchronous exception propagation is still unsupported, so nullable
`Object.keys` rejects in synchronous source. Getters, proxies, Symbols, custom
prototypes, mutable function/Error properties and arbitrary dynamic property reads
or writes remain outside this increment. Resource exhaustion and malformed private
values are engine/protocol failures, never fabricated source TypeErrors.

## Module-lifetime private function identities

`ModuleClosure` reads a single immutable function object for a capture-free,
synchronous private target. The backend allocates that object in a traced Wasm
global at module instantiation. Repeated reads, including reads in different
function bodies, keep identity; distinct targets and separate module instances
remain distinct. Ordinary `MakeClosure` still allocates a fresh function.
The existing typed closure call ABI handles Number, General and managed values.

The verifier checks the target's entry, empty capture list and complete signature.
The operation selects GC storage even when no managed carrier is otherwise used;
numeric async spills explicitly reject it. Primitive modules without the operation
retain their encoding. Per-read allocation would split declaration identity, and
process-wide interning would merge separate module instances. A module global fits
the existing engine tracing model without numeric handles or a new frame format.

Eleven maintained emitted-IR/native-JS/Node groups cover these contracts, malformed
IR and a 2 MiB native stack. A test-only observer retains an actual generated
module-function read across separate call activations. A runtime fixture uses the
production GC type fragment to compare per-instance identities against native
JS/Node and to force Wasmtime copying collection between typed calls. These are backend tests;
source declaration integration is described below. The backend tests alone do not
establish source compatibility, a public reference export or suspended GC frames.

## Source module declaration values

Top-level synchronous function declarations, including `run`, can be read as typed
function values. A source-reference scan plans one capture-free adapter per
declaration used as a value; plain direct calls retain their existing ABI and
Number specialization. The adapter forwards the checked Number, General or managed
parameters/result to the original body. Its `ModuleClosure` identity belongs to
the module instance, so aliases, reads in different bodies, object/array storage,
and typed local callbacks refer to the same declaration. Distinct declarations and
fresh arrow creations remain distinct. Source order and erased TS annotations do
not change binding identity.

Representation planning follows declaration aliases when gathering managed input
facts and carries checked callback result lanes into the existing finite flow
planner. It does not evaluate source expressions or choose a runtime call target.
Lowering checks the entire signature and preserves argument/receiver effects and
callee snapshots. Managed property values keep their existing explicit consumer
rules; transporting a function does not prove an arbitrary property is numeric.

`module-declaration-values` executes source programs against native JS, Node-Wasm
and Wasmtime, with primitive-profile cases also in the canonical primitive replay.
It exercises hoisting, aliases, identity, callbacks, managed input/results, cycles,
annotation erasure, zero/multiple parameters and a 2 MiB native stack. Nested or
asynchronous declaration identities, writes to module function bindings, calls
through opaque stored carriers and suspended GC references remain follow-ups.
The 1,000-parameter frame control uses an explicit two-million-fuel engine budget;
it does not increase the production per-call budget or promise that every admitted
wide program fits that smaller budget.

## Managed inputs through module callbacks

Module-helper input planning follows original-source constructor origins through
known local calls, aliases and returned factories. It records formal/actual input
edges without evaluating source code, then computes managed lanes using the
existing finite kind lattice. Public entry parameters remain Numbers. Programs
without a synchronous private helper or a non-Number runtime value origin retain
the earlier path. Boolean/nullish values also seed boxed input facts; unused TS
literals and import/export specifiers do not establish runtime value origins.

The storage planner also follows named module targets for managed results through
callback aliases. Known targets must agree on a result lane; these positive facts
do not establish a complete callable signature. Lowering still checks every
alternative and rejects unknown/incompatible calls. A mismatched managed consumer
returns a diagnostic instead of reaching a scalar-routing assertion.

`module-callback-inputs` compares source with native JS, Node-Wasm and Wasmtime for
objects, arrays, strings, mixed inputs, mutation, reads, cycles, forwarding,
factories and argument snapshots. Receiver and surplus effects still execute once;
they do not become declared input facts. Type annotations remain erased. Two-MiB
stack, 128-local boundary and explicit opaque/unknown/signature/async guards cover
the limits. This changes source analysis, not the call ABI or coroutine frames.

## Array range copies

Proven intrinsic arrays support `slice(start, end)` and literal-computed
`['slice']` calls. Number endpoints use JavaScript relative-index clamping;
omitted or supported explicit Undefined start/end select zero/current length.
The receiver and evaluated endpoints are saved before later argument effects.
Length and contents are read after all arguments finish, including surplus
arguments and supported suspension. Only own indexed data entries are copied;
holes stay absent, named properties are excluded, slots are fresh, and child
references keep their identity. A two-pass sparse-table copy costs space and
time proportional to stored entries rather than logical length.

The result is a proven Array and composes with helpers, captured handlers,
iteration, length, own-key queries and boxed indexed reads. Iteration and binding defaults retain copied element kinds from all possible
source indices, including aliases and writes to either array. Dynamic endpoints
may select any subset, so analysis conservatively includes all source positions.
Named fields do not contribute. Direct indexed reads can miss; holes, resizing
and incompatible writes retain conservative guards. Shared child identities
remain linked for field mutation analysis. Borrowed/optional calls, dynamic iterable spreads,
endpoint object coercion, custom species and prototype changes remain
outside the admitted surface. Ordinary source methods named `slice` retain their
existing call behavior. Tests in `managed-arrays.rs`, the suspension suite and
`a1_ui_import.rs` compare source behavior across native JS, Node-Wasm and Wasmtime.

## Reversing an Array

Proven intrinsic arrays support `reverse()` and literal-computed `['reverse']`
calls. The operation saves its receiver, evaluates all ignored argument effects,
then reverses its current indexed entries while retaining length, sparse holes,
named fields and child identities. It returns the same Array reference. The
private guest helper visits stored entries instead of logical length; it invokes
no host operation. Helpers, captured handlers and suspension retain the returned
alias, so writes through it affect the source array.

Reversal invalidates positional field/callable/String proofs through aliases
of a reversed array, while iteration retains the union of possible element
origins. A rest suffix after reversal considers every original position. Holes,
resizing and incompatible writes retain their conservative guards. Boxed reads, identity comparisons, length, own-key queries
and searches remain available. Optional/borrowed calls, spread arguments,
overridden methods, prototype mutation and generic array-like receivers remain
outside the admitted surface. Existing `pop` and `reverse` share the same
source-ordered argument scheduler. Source-unit and differential regression cases
cover sparse slots, aliasing, argument effects, rejection and the A1 service call.

## Consuming the front of an Array

`shift()` and literal-computed `['shift']()` remove and return the first boxed
value of a proven intrinsic array, or Undefined for an empty array or first-slot
hole. Remaining indexed entries move down by one, preserving holes and named
properties; the length decreases only when it was nonzero. Object and function
values retain identity. The sparse-table implementation visits stored properties
rather than logical length. It shares the pop/reverse scheduler, preserving the
receiver across all argument effects and awaits before inspecting current data.

As with `pop`, the returned value stays boxed and positional source proofs are
invalidated through aliases. Borrowed/optional calls, spread arguments, overrides
and generic array-like receivers remain separate work. The array, suspension and
A1 suites cover queue consumption against native JS, Node-Wasm and Wasmtime.

## Reversed snapshots

`toReversed()` creates a fresh dense reversed array from a proven intrinsic
Array. It leaves source slots and length unchanged, preserves child identities,
excludes named properties, and materializes holes as own Undefined entries.
The existing guest dense-copy and reversal operations compose after all surplus
argument effects finish, preserving the saved receiver across suspension.
Iteration retains copied value origins, with independent result-slot writes and
shared-child field writes tracked separately. Direct indexed reads may miss;
unknown receivers, overrides, borrowed/optional calls and spread arguments remain
outside admission. Unlike sparse `reverse`, allocation is proportional to length.


`for...in` also accepts source-proven object, Array or String receivers mixed
with Null/Undefined. Nullish alternatives enumerate nothing. The original RHS
executes once, and the saved receiver supplies later presence checks even after
rebinding or suspension. Removed keys are skipped before assigning the head.
Heterogeneous container kinds and opaque receivers remain guarded; this does
not widen ordinary property reads or prototype mutation support.


Assignment patterns can target existing scalar parameters when source facts prove
that each selection/default fits the parameter's cell. Number parameters require
Number-only alternatives, and tagged primitive cells retain their original tags.
The RHS is still saved once, defaults remain lazy, and later selections/defaults
observe preceding writes. Incompatible parameter kinds and annotation-only proofs
remain guarded; this does not widen public result signatures.


Array/object assignment patterns also accept static/literal own member targets
on source-proven objects and Arrays. Each original destination reference executes
before its value is selected and before its lazy default; supported awaits retain
that address. Array patterns use a live, sticky-done iterator when target effects
can change the source. Alias field facts include every assigned selection/default.
Computed destinations additionally admit unsigned numeric keys on owned objects/Arrays
and source-proven String keys on null-prototype dictionaries. On ordinary records
the String key must also exclude Object prototype names. The receiver and key
execute once, in that order, before source selection; both survive awaited keys and
lazy defaults. Elisions finish before evaluating the next member reference, so an
exhausted iterator cannot be revived by a destination effect. Finite key sets widen only the possible target fields across aliases.
Nested target patterns, unproven/coercing keys, nullable/unknown
receivers and unchecked Array length writes remain guarded.


Array/object rest assignments also accept the same owned member destinations.
The original reference (including supported computed keys/awaits) executes before
collecting the fresh rest container. Array elisions retain sticky exhaustion;
object exclusions and shallow reference identity use the existing rest operations.
Source facts track the new allocation and its possible contents through aliases.
String Array rest uses code points; String object rest copies enumerable UTF-16 indices.


Object assignment patterns accept computed source keys with the ordinary owned-read
proofs: unsigned numeric syntax on proven objects/Arrays, or source-proven Strings
on null-prototype dictionaries. Each key is saved before the original destination
reference executes, followed by GetV and its lazy default; awaits retain that order.
Selected-value facts use the same conservative own-value/missing alternatives as
ordinary computed reads. On primitive Strings a proven finite choice of canonical index names and length uses the same selection and rest exclusions.


Computed source keys also participate in object-rest exclusions. Each saved key is
retained through destination effects/default awaits and excluded from the fresh
copy of the original RHS. Duplicate/static/runtime names share the existing key
representation. Source facts conservatively include absence for every field that
an unknown exclusion could remove, preserving kind and alias proof boundaries.


Computed source indices in object assignment patterns also accept source-proven
Strings with unsigned numeric syntax. They select UTF-16 code units (including
individual surrogate halves), with Undefined outside the string. Lazy defaults
and saved RHS/key/reference ordering use the same cursor as object/Array sources.
String object-rest copying preserves exclusions for both unsigned indices and proven String keys.
The existing projections still require a proven String before String-only operations;
numeric conversion of boxed Strings is outside this admission change.


Object-rest assignment from a source-proven String copies its enumerable UTF-16
index properties. Length and inherited names are absent. Literal/unsigned source
exclusions and existing member destinations share the saved RHS/key/reference
cursor, including awaits and rejected defaults. A temporary own-data object feeds
the existing rest operation after destination evaluation. Nullable/opaque String
sources and unproven String source keys keep their guards.


Declaration, nested and iteration object bindings also accept String object rest.
Their existing binding cursor copies UTF-16 own indices after preceding fields and
lazy defaults, including supported suspension; each rest result is fresh. Source
proofs and key-admission checks remain in force.


Computed object binding keys also accept unsigned numeric syntax on proven
objects, Arrays and Strings. Keys evaluate once before field selection and lazy
defaults; object-rest exclusions retain the saved numeric key. Strings select
UTF-16 code units. Existing dictionary String keys remain supported; coercing,
annotation-only and unproven receivers/keys keep their guards.

Object.hasOwn now accepts source-proven unsigned numeric keys (including the
>>>-derived 4294967295 named-property boundary) on its existing proven
object, Array, String, Number and Boolean receivers. The key is normalized
through the existing Uint32 managed-property carrier; receiver, key and ignored
extra arguments still execute in source order. Array holes remain distinct from
own Undefined entries, and primitive Number/Boolean receivers have no own keys.
Arbitrary runtime Number keys, key coercion and unsupported receiver proofs
remain guarded; TypeScript annotations do not establish unsigned range.

Fixed Array-literal spreads in `Object.hasOwn` calls now use the same
original-expression argument expansion as ordinary represented function calls.
Nested literal spreads and zero-length spreads preserve left-to-right argument
evaluation, receiver/key snapshots and ignored extra-argument effects. Each
selected key still needs a static primitive, source-proven String, or proven
unsigned Number; a spread cannot launder unknown kinds or receiver proofs.
Opaque iterable spreads, sparse literal holes and more than 1,000 flattened
arguments remain guarded. This does not add general iterable or standard
library call-spread semantics.

Ordinary records support computed reads and plain assignments when the key
can only select String names outside the Object prototype. For example,
`const key = enabled ? "active" : "idle"; states[key] = next` selects the
property at runtime. String literals, substitution-free templates, const
aliases and sequence tails can establish those names. The receiver is saved
before key evaluation, and the key is saved before the assigned value.
An absent field reads as Undefined. Null-prototype dictionaries continue to
accept arbitrary source-proven Strings, including intrinsic names.

Optional reads such as `settings?.[key]` use the same name proof for
ordinary records. A nullish receiver skips the entire key expression,
including its effects and awaits. A present receiver is saved before the
key runs, so rebinding its source variable cannot redirect the lookup.

Object binding and assignment patterns also admit these runtime names.
In `const {[key]: value, ...rest} = record`, the key runs once, selects
the field, and excludes that same name from the rest copy. Defaults retain
their usual missing-or-Undefined behavior.

A computed field with the same key proof also supports `&&=`, `||=`, and
`??=`. The receiver, key, and previous value are saved once. A skipped
assignment leaves the field untouched.
Numeric compound assignments and prefix/postfix updates use the same saved
key when the old value has a proven primitive numeric-coercion domain.
Postfix returns the converted old value and prefix returns the stored result.

Runtime field names can also be constructed by String concatenation and
template interpolation. Fixed prefixes, suffixes, and interior fragments
must together rule out every name on the Object prototype. For example,
`record["user:" + name]` can use an arbitrary supported String, while
`record["to" + name]` can still select an inherited method.

A lexical `let` may carry a computed record name across reassignment.
Its initializer and every write must preserve the safe String domain,
including writes inside closures. Copies between bindings share that proof.
Arithmetic compound writes, destructuring targets, and iteration-head assignments
cannot establish it. Property operations still load the current value and
save the key before any later RHS mutation.

Logical key expressions preserve finite names across `||`, `&&` and `??`.
For example, `(flag ? "save" : "") || "cancel"` can select either handler.
The same finite choice survives a local alias such as `const key = raw || "cancel"`
when every assignment to `raw` supplies a proven name. Nullish left operands
of a fallback also skip to the right name, including null, unshadowed `undefined`, or `void` values
flowing through local aliases. An ordinary initializer-free `let key` contributes undefined until a checked
write supplies a name. Under `??`, the empty String still selects its own field. The key
expression still evaluates with normal short-circuiting.

A local assignment result can name a computed field. For example,
`handlers[key ||= "default"](value)` uses the retained or assigned name.
All writes to the binding still participate in the proof.

When a runtime key has a small finite set of literal alternatives, ordinary
and optional reads combine just those fields' possible values. A dispatch
table can therefore call `actions[key](value)` when every selected field
contains a function with the same represented signature. Missing fields,
deletion, and writes through aliases still affect the result. Larger or
open-ended name sets use the conservative own-property lookup.

The direct `value instanceof Array` expression evaluates the left operand
once and tests the represented array in the pristine realm. A record with
array-like fields fails that test. The right operand must resolve to the
intrinsic constructor; user constructors, prototype mutation, proxies and
custom instance checks remain separate features.

Direct `instanceof Function` and `instanceof Object` also recognize the
realm's intrinsic constructors. Ordinary records, arrays, and functions
inherit from Object; `Object.create(null)` records and scalar primitives
do not. Null-prototype records use a WasmGC subtype with the same property
storage layout, so their prototype identity survives aliases and joins.
Rest and spread copies have an ordinary Object prototype.

The direct intrinsic forms `Array()` and `new Array()` each allocate a
fresh empty Array. A single proven uint32 Number sets its sparse length.
Holes have no own index entries, even at length 4294967295. Length
expressions execute once. Unchecked numeric lengths remain guarded until
the compiler supports the constructor's catchable RangeError.

Two or more ordinary constructor arguments produce dense own elements in
argument order. A single source-proven non-Number also becomes one own
element, including an explicit Undefined. Single Numbers still select the
length overload. The elements preserve their represented kinds and identities.
Fixed, non-elided array-literal spreads expand before overload selection,
so `Array(...[3])` has three holes while `Array(...[undefined])` has one
own slot. Both source and expanded argument counts use the call width limit.

Finite key alternatives also narrow mutation effects. A write or delete
through an alias widens every field it could select. Unrelated fields
retain their kinds, so a table's callback or label can still be used after
updating one of its counters. Open-ended keys retain wildcard effects.

Untagged template substitutions accept String, Boolean, Null, Undefined and
Number, including mixed primitive values. Dynamic binary64 Numbers use
ECMAScript's shortest round-tripping decimal, fixed/exponential cutovers and
spellings for both zeros, NaN and infinities. The guest formatter and its
immutable tables are emitted only for conversions that can receive a dynamic
Number. Numeric literals reuse their compile-time ECMAScript spelling. Each
substitution runs once in source order, including across suspension. Objects
with user-defined primitive conversion remain outside the admitted subset.

Computed reads and plain assignments on records and arrays accept runtime
Number, Boolean, Null and Undefined keys. Numeric keys use ECMAScript decimal
spelling. For example, `-0` selects index zero, while fractions, negative
integers, NaN, infinities and 4294967295 name ordinary fields without growing
an array. Proven uint32 keys retain the smaller unsigned path. Null-prototype
dictionaries can mix these primitive kinds with arbitrary String keys.

The same primitive keys work in numeric prefix/postfix updates, numeric
compound assignments, and `&&=`, `||=`, and `??=`. A property reference keeps
its receiver and key across RHS effects and awaits. Logical RHS expressions run
only when the saved value selects the assignment.

Calling the unshadowed `String` function applies the same primitive conversion.
No arguments produce the empty String. Subsequent arguments still run in order
and may suspend or throw. Fixed array-literal spreads follow ordinary call
argument expansion. Boxed String construction and object coercion remain guarded.

`in` and `Object.hasOwn` accept runtime primitive keys. On an array or record,
a non-String primitive key cannot spell an inherited intrinsic name. Possible
String keys retain the receiver prototype checks for `in`. The key and receiver
are evaluated in their original order, and `Object.hasOwn` tests presence
after its remaining arguments finish.

Optional computed reads on arrays and records accept runtime scalar keys. The
receiver is saved before key evaluation, and a nullish receiver skips key
effects. Numeric keys preserve their JavaScript spelling, including fractions.

Both `delete obj[key]` and `delete obj?.[key]` accept runtime primitive keys.
Array deletions preserve length and leave holes. Dynamic Strings that could
name the non-configurable Array length remain guarded.

Destructuring assignments use the same primitive keys to select a property,
exclude it from object rest, or write a saved member destination. Defaults
and subsequent assignments retain the original receiver and key.

Lexical binding patterns also share primitive-key conversion, including nested
records, rest exclusions, and per-iteration bindings. Member targets in
`for...of` and `for...in` evaluate the receiver and computed key on each visit.

The pristine `Number` function accepts the four scalar primitive kinds.
`Number()` returns positive zero, while `Number(undefined)` returns NaN.
The selected input is saved while additional arguments evaluate, including
across suspension. String parsing, reference coercion and boxed Number
construction remain outside this conversion.

Calling the pristine `Boolean` function applies the language truthiness rules
to the saved first argument. Omission produces false, and surplus arguments
still execute in source order, including their effects and rejections.

A unit-step `for` counter can supply an unsigned key or array length inside
a loop with an inclusive unsigned upper bound. Comparisons may place the
counter on either side, including `0 <= i` for reverse traversals.

Array length assignments accept runtime values proven to be uint32, including
another array's length, a checked loop counter or an unsigned shift. A conditional
or logical expression can also select among validated unsigned literals and
runtime unsigned values. Unsigned shift assignment and a bitwise AND assignment
with a nonnegative int32 mask also yield validated lengths. Shrinking
deletes indexed properties, growing creates holes, and the assignment returns
the original RHS Number (including negative zero).

Membership tests on a mixture of ordinary and null-prototype records check own
properties first. An own miss consults the pristine Object prototype only for
an ordinary record. This applies to both literal and computed inherited names.


Object literals and `Object.create` accept a null prototype selected through
a conditional, a sequence, or earlier simple `const` aliases when all possible
results are null. The original expression still evaluates at the construction
site, including effects before the selected null value.


Local logical assignment keeps the identities and possible contents of its
selected operands. For example, when `state` contains either a record or
null, `const row = state ??= {n: 0}` can read `row.n`. Array length and nested
record fields use the same source facts. A possibly null result of `&&=` cannot
supply an ordinary property receiver. A mutable local known to hold a record
or array also supports these operators with a same-kind RHS. Its current
reference is truthy and non-nullish, so only `&&=` evaluates the RHS.


A `let` binding can start uninitialized, null, or undefined and later hold
a String, record, or Array. Source-proven writes select storage that
preserves both the initial nullish value and the later reference. This
supports lazy initialization such as `let cached; const item = cached ??= {n: 0}`.


After a standalone, unconditional assignment to a `let` binding with a single
write, later reads in the same function use the assigned value's known kind.
For example, `let row; row = {n: 1}; return row.n;` reads a proven record.
A standalone assignment in a branch, loop body, or try block also establishes
the value for later reads inside that block. Reads after a possibly skipped
block and reads in nested functions retain their conservative checks.


When both arms of an `if/else` end in a plain assignment to the same local
and those are its only writes, reads after the conditional in its block
can use the union of the two RHS values. The same proof covers nested choices and exhaustive `else if` ladders. Every
reachable assigned value must support the subsequent operation.


Calls to `at`, `slice`, `includes`, `indexOf` and
`lastIndexOf` on proven Arrays accept nested, hole-free Array-literal
argument spreads. The receiver and earlier arguments are saved while later
arguments run; the operation observes mutations up to the actual call.


Supported String methods also accept fixed Array-literal spreads, including
UTF-16 indexing, range selection, search, concatenation and repetition.
Each method applies its ordinary defaults to the expanded argument list.


When all writes to a lexical local are plain assignments in one statement
list, later reads in that block use the most recent completed assignment.
A read inside a subsequent RHS still sees the previous value.


Binary `+` on two represented primitive operands can choose its result kind
at runtime. A String on either side selects UTF-16 concatenation; otherwise
Numbers, Booleans, null and undefined use numeric addition. Both source
operands execute once, left to right. Object-to-primitive hooks and BigInt
remain outside this operation.


The same runtime choice applies to `+=` on mutable locals and admitted own
properties. The old value, receiver and computed key are saved before the RHS;
the store occurs after successful evaluation, including across suspension.


Awaiting a source-proven union of represented primitives that includes String
preserves the selected tag and value. For example, `await (left + right)` may
resume with either a Number or a String. The operand executes once and each
await still schedules its reaction job.


String concatenation and search methods convert represented primitive
arguments with the same UTF-16 and Number spelling as explicit `String`.
An omitted search argument denotes the text `"undefined"`. Objects and
regular expressions still require coercions outside this profile.


A sum whose operands can only be Numbers, Booleans, null or undefined
produces a Number on every completing path. This fact is retained in
record fields, Array elements and destructuring, including when the
actual result is NaN.


For a lexical binding with no writes, a truthiness check or a loose nullish
comparison can establish a non-nullish receiver inside the matching branch.
An early return or throw from the opposite branch extends that fact to later
statements in the same block. Captured writes and nested function reads do
not inherit the proof.


Both branches of a `typeof` comparison or unshadowed `Array.isArray` check
retain the matching source kinds. Strict null and undefined comparisons
exclude only the tested value, so separate checks can compose. A `void`
expression also supplies undefined: its operand runs before the comparison
finishes, including in a switch label.
`typeof null` remains `"object"`.


The same guards apply to unchanged simple parameters of helpers and arrows.
An assignment, a redeclaration, or an aliased `var` cell prevents parameter
narrowing; source argument kinds still determine the callable interface.
Ternary arms and the right operands of `&&` and `||` also use the guard
established by the branch that selects them.
A truthy conjunction or a falsy disjunction can establish facts about both
operands, as in `if (!row || !items) return` before using both receivers.
A successful `while` or `for` test also narrows an unchanged binding inside
the loop body and the `for` update expression, including after a `continue`.
A guard clause ending in `break` or `continue` narrows later reads in the
same block. Within a switch case, the proof extends only to that case's end.
A simple local `const` can save a guard for a later branch. For example,
`const ready = row != null` lets `if (ready)` use the unchanged receiver.
Negation, aliases of aliases and mandatory operands of a compound guard
preserve the corresponding facts.
Strict equality with a String, Number or Boolean literal establishes its kind
on a match, including in a directly selected switch case. A missed literal
still permits other values of that kind.
