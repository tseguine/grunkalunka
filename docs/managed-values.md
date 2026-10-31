# Managed values and source data objects

The private structured IR now has a GC-backed value carrier and a known-object
reference kind. Synchronous source lowering now constructs ordinary data objects
and uses their references, properties, equality and truthiness through this IR.
This is an increment toward [#300](https://github.com/tseguine/grunkalunka/issues/300),
not complete source object/array support. Exported Number/primitive signatures and
the existing async contract are preserved.

## Logical operations and representation

`ManagedValue` currently carries Undefined, Null, Boolean, Number, String or an
ordinary object reference. `ManagedObject` denotes a known ordinary object. Each has one
traced WasmGC reference lane; neither is a numeric handle into linear memory.
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

Property keys are lossless UTF-16 vectors, including NUL and lone surrogates.
The initial implementation compares GC-backed packed key arrays and stores entries
in a GC array. Replacing an existing entry preserves its position; appending a new
entry copies the reference array and updates the object's storage. This deliberately
simple implementation has linear lookup/append costs. It is not a chosen long-term
shape/cache/hash-table strategy or an ECMAScript enumeration implementation.

These are explicitly **own data-property** operations. They do not implement JS
prototype lookup, getters/setters, proxies, `__proto__` initializer semantics or
array exotic behavior. Future source lowering must implement the necessary semantic
operation or reject a case; it must not translate arbitrary `o[k]` into own-only
lookup and silently discard inherited behavior.

## Synchronous source admission

Fresh object literals admit ordinary initializer/shorthand properties with static
identifier or String keys, including computed String literals. Values can contain
the four primitive kinds, Strings or other constructed objects. Properties evaluate left to
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
the primitive ABI; observations must produce a supported primitive result. Arrays
and function values remain unimplemented. String methods, indexing, length,
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

## Calls, cells and lifetime boundaries

Ordinary synchronous named source helpers can accept/return managed references,
including String values, mixed primitive/reference arguments and property-value results. A finite
source-flow query joins actual arguments, local initializers/writes and returns by
resolved symbol identity until no kind facts change. Object-only flow uses a known
object reference; String-bearing or mixed object/non-object flow uses the managed carrier. TS type
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
store changes the binding, not an earlier loaded snapshot. The current captured
cell/environment and async spill formats are numeric linear-memory storage;
managed captures and async modules therefore reject explicitly. Supporting them
requires reference-aware GC frames/environments and lifetime tests, not merely a
new lane count or a numeric surrogate.

The GC type/helper fragment is emitted only for modules using managed kinds and
appended after source functions. This keeps source call indices stable. Existing
numeric/primitive modules do not acquire GC types or runtime helpers merely from
this implementation being available.

## Evidence and remaining work

Ordinary compiler tests construct typed IR and execute the emitted module in Node
against independent native-JS observations. They cover all four primitive kinds,
object identity under reboxing, multiple UTF-16 keys, missing values, overwritten
properties, saved snapshots, mixed reference/Number helper parameters and local
binding stores. Malformed IR, exported/private-closure signatures, captures and
async byte-spill boundaries have rejection tests.

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
