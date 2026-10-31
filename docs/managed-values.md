# Managed values and source data objects

The private structured IR now has a GC-backed value carrier and known object/array
reference kinds. Synchronous source lowering constructs ordinary data objects and arrays
and uses their references, properties, equality and truthiness through this IR.
This is an increment toward [#300](https://github.com/tseguine/grunkalunka/issues/300),
not complete source object/array support. Exported Number/primitive signatures and
the existing async contract are preserved.

## Logical operations and representation

`ManagedValue` currently carries Undefined, Null, Boolean, Number, String or an
object reference. `ManagedObject` denotes a known ordinary object, and
`ManagedArray` a known array. Each has one traced WasmGC reference lane rather than
a numeric handle into linear memory.
Known Number computations still use f64, and the existing `General` primitive pair
is separate. `ManagedPrimitive` bridges its four canonical primitive kinds;
extended async Error/Promise tags cannot be copied into this private tag space.

The immutable value carrier has a kind, numeric payload and nullable GC reference.
Ordinary objects have mutable GC-backed property storage. Boxing an object keeps
the object's identity: two carriers for one object compare equal, while fresh
objects remain distinct. Primitive comparisons retain kind differences, NaN and
signed-zero rules. String equality compares lossless UTF-16 code units, never
carrier/array identity or Unicode-normalized text. Future BigInt equality also needs value comparison;
Symbols, functions and other object kinds need their own
construction/identity operations before admission.

| Operation | Contract |
| --- | --- |
| `ManagedString` | Construct an immutable String value from lossless UTF-16 units. |
| `ManagedObjectNew` | Allocate a fresh ordinary object. |
| `ManagedPrimitive` | Copy a canonical four-kind primitive into an immutable managed carrier. |
| `ManagedFromObject` | Wrap an existing object without changing its identity. |
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

`ManagedArrayResize` accepts an already validated immediate `u32` length. Shrinking
removes indexed own properties at or above that length while retaining named
properties; growing creates holes and does not restore deleted entries. Generic
property writes to `length` reject in the IR verifier so they cannot bypass length
validation. Dynamic coercion, RangeError completion, property descriptors and
prototype lookup are not implemented by these operations.

The source cursor now supplies the receiver/key/length proofs described below.
Managed captures, public reference exports and suspended async frames retain their
existing guards. Compiler tests execute
emitted IR in Node against native JS; runtime kernel tests exercise Wasmtime,
holes, sparse storage, cycles, text edges and repeated forced copying collections.

## Ordinary-object property storage

Property keys are lossless UTF-16 vectors, including NUL and lone surrogates.
The initial implementation compares GC-backed packed key arrays and stores entries
in a GC array. Replacing an existing entry preserves its position; appending a new
entry copies the reference array and updates the object's storage. This deliberately
simple implementation has linear lookup/append costs. It is not a chosen long-term
shape/cache/hash-table strategy or an ECMAScript enumeration implementation.

The ordinary-object operations are explicitly **own data-property** operations.
They do not implement JS prototype lookup, getters/setters, proxies, `__proto__` initializer semantics or
array exotic behavior. Future source lowering must implement the necessary semantic
operation or reject a case; it must not translate arbitrary `o[k]` into own-only
lookup and silently discard inherited behavior.

## Synchronous source admission

Fresh object literals admit ordinary initializer/shorthand properties with static
identifier or String keys, including computed String literals. Values can contain
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
Object.prototype key rejects**, even when a literal would shadow it. The guard is
checked against the actual native intrinsic names in tests. Other absent static
keys return Undefined. This temporary admission boundary is not a chosen permanent
object model or a replacement for prototype-aware lookup.

Dynamic property keys, optional access, unknown-kind receivers, accessors, methods,
spread and numeric literal keys reject. Implicit managed coercions, loose equality,
managed logical-value joins, managed captures and reference-aware async frames
remain separate work. Managed properties cannot be returned directly through
the primitive ABI; observations must produce a supported primitive result.
Function values in managed containers remain unimplemented. String methods, indexing, length,
concatenation and coercions remain separate work. Unsupported branches
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

## Synchronous source arrays

Array literals construct traced references on the same expression cursor as
objects and primitive values. Elements evaluate left to right; holes remain absent
entries and trailing commas retain their JavaScript length. Arrays can contain
the four existing primitive kinds, Strings, objects and other arrays, including cycles. Aliases,
rebinding, saved element reads and the receiver snapshot before an assignment RHS
retain their identity/value semantics. Empty arrays are truthy.

Known-array reads and plain writes admit static String keys and unsigned integer
Number literals through 2^32-1. The backend distinguishes actual array indices
from named properties. `length` reads, including computed static text, produce
known Numbers and preserve existing numeric layouts. Plain length writes accept
valid unsigned integer literals through 2^32-1, retain the assigned Number result,
remove truncated elements and leave holes on subsequent growth. Erased TS wrappers
and parentheses are checked under the existing expression-depth budget.

Every Object.prototype intrinsic key and every Array.prototype String key except
the array's own `length` rejects for an array receiver, including shadowed keys.
Tests compare this boundary with the native intrinsic names. The additional Array
guard does not restrict ordinary objects' own keys such as `map` or `push`.
Prototype mutation and foreign references remain unavailable in this profile.

Dynamic keys, other numeric keys, spread, methods, descriptors, coercions, invalid
or dynamic length writes and optional access still reject. Element snapshots use
the general carrier: returning `items[1]` directly through a Number export still
requires a primitive-result proof/projection, and exporting an array requires a
reference-aware boundary. Those existing gap fixtures remain rejected; construction
and equality tests do not claim those gaps are closed. Managed captures and async
frames remain unsupported, with source-level capture diagnostics.

## Short-circuit values and defaults

Source `&&`, `||` and `??` select actual managed operands through the existing
ordered expression cursor and typed branch regions. The left operand is evaluated
once and retained as a snapshot. The right operand is validated as source in all
cases, but runs only when selected. Objects are truthy; String truthiness uses its
length; `??` tests only Undefined/Null, preserving empty text, false, zero and NaN.
`ManagedIsNullish` produces a predicate from the managed language-kind tag.

Logical results work in data properties, local bindings and ordinary helpers,
including mixed primitive/reference helpers and discarded expressions. Static
Number-only arguments retain f64. A Boolean/nullish argument to a helper that also
transports managed values uses the existing carrier, since ordinary source helper
parameters exclude the `General` pair. Boolean/nullish helper results in numeric
entry profiles use the same carrier when needed to retain their kind; other entry
profiles retain the original primitive-result planning. Metadata supplies no kind
proof. This does not add primitive projection from arbitrary managed results.

Managed logical assignments, function values in the carrier, captures/suspended
frames and public managed exports remain outside this increment. Unsupported
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

The current increment does not define an ambient global namespace. Unknown global
names, including unrepresented built-ins, remain explicit source rejections rather
than being assumed absent. That is an implementation gap: JavaScript's special
unresolvable-reference result and actual TDZ exception execution remain open in
the `typeof` rejection-inventory requirement. BigInt/Symbol operands, dynamic
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
Ignored `thisArg` and surplus arguments still evaluate in source order. Captured
function values and async helper frames are not admitted by this increment.
The flow query does not establish source validity: unknown-kind property receivers,
unsupported operations and every source branch still undergo ordinary validation.
A helper seen with both object and numeric callers cannot silently treat all inputs
as objects. Function aliases and public reference exports remain unsupported.

Logical argument checks use the callee's typed slots, and the Wasm header uses the
shared backend layout. Existing ordinary Number calls retain f64 signatures. Exported
inputs/results remain in the existing admitted profiles, and private closure
signatures still reject managed values.

Uncaptured source cells and computed-value snapshots use traced locals. A cell
store changes the binding, not an earlier loaded snapshot. The private IR now
supports traced captured cells and closure environments as described below. Source capture admission and async spill migration remain separate
steps; their source guards still reject managed captures/async modules explicitly.

The object/String GC kernel is emitted only for modules using managed kinds and
appended after source functions. Closure storage types precede closure function
types and add no functions. This keeps source call indices stable. Existing
numeric/primitive modules do not acquire GC types or runtime helpers merely from
this implementation being available.

## Traced closure backend

The synchronous private IR admits captured `ManagedValue`, `ManagedObject` and
`ManagedArray` cells. A module with any managed captured cell uses one GC cell
domain for all its captured bindings and closures. This matters when both numeric
and managed closures refer to the same mutable binding: both environments must
retain the same cell. A load still creates a value snapshot; storing a new reference
updates the binding without changing an earlier loaded value.

Each cell has unboxed i32/f64 fields and a traced reference field. An environment
is a GC array of cell references. The private closure pair contains a table index
and a traced environment reference; call arguments use fresh cell entries in a GC
frame array. Closure parameters/results retain their existing logical contracts,
including primitive pairs and returned/callback closures. Managed closure parameter
and result signatures remain rejected in this unit.

GC types precede explicit closure function types so type references do not cross
forward into a later recursion group. The source function indices remain stable.
Modules without managed captured cells retain their existing numeric arena, closure
layout and async behavior. Modules using the traced path need no numeric arena reset
wrapper. Ordinary Number arithmetic and locals retain f64 in both paths.

This is a backend prerequisite: source managed captures remain explicitly rejected.
Managed values still cannot enter the old async byte-spill frames. Traced suspension,
source capture admission, general function values in containers and public reference
ownership remain follow-up work. This implementation does not add a shared heap
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

Arrays, broader String operations, general function identities, managed captures,
suspended GC frames, public heap ownership and full coercion/prototype behavior
remain work under the original general-purpose JavaScript goal.
This source increment does not complete the value domain or justify counting
rejected programs as supported.
