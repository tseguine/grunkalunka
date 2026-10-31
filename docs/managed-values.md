# Managed values and source data objects

The private structured IR now has a GC-backed value carrier and a known-object
reference kind. Synchronous source lowering now constructs ordinary data objects
and uses their references, properties, equality and truthiness through this IR.
This is an increment toward [#300](https://github.com/tseguine/grunkalunka/issues/300),
not complete source object/array support. Exported Number/primitive signatures and
the existing async contract are preserved.

## Logical operations and representation

`ManagedValue` currently carries Undefined, Null, Boolean, Number or an ordinary
object reference. `ManagedObject` denotes a known ordinary object. Each has one
traced WasmGC reference lane; neither is a numeric handle into linear memory.
Known Number computations still use f64, and the existing `General` primitive pair
is separate. `ManagedPrimitive` bridges its four canonical primitive kinds;
extended async Error/Promise tags cannot be copied into this private tag space.

The immutable value carrier has a kind, numeric payload and nullable GC reference.
Ordinary objects have mutable GC-backed property storage. Boxing an object keeps
the object's identity: two carriers for one object compare equal, while fresh
objects remain distinct. Primitive comparisons retain kind differences, NaN and
signed-zero rules. Future String/BigInt equality must compare values rather than
carrier references; Symbols, functions and other object kinds need their own
construction/identity operations before admission.

| Operation | Contract |
| --- | --- |
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
the four primitive kinds or other constructed objects. Properties evaluate left to
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
managed logical-value joins, source helper reference parameters, managed captures
and reference-aware async frames remain separate work. Managed properties cannot be returned directly through
the primitive ABI; observations must produce a supported primitive result. General
String/array/function values are not admitted by this increment. Unsupported branches
are checked even when a particular input would not execute them.

## Calls, cells and lifetime boundaries

Ordinary internal helper functions can accept/return managed references. Logical
argument checks use the callee's typed slots, and the Wasm header uses the shared
backend layout. Existing ordinary Number calls retain f64 signatures. Exported
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
conditional reference values, truthiness, UTF-16 keys and rejection boundaries.
The canonical compatibility corpus includes source object programs and preserves
the exact formerly rejected object-discard sources as executed evidence.
Small-stack probes exercise both source depth diagnostics and existing constructor
checks without increasing native stacks or source limits.

Runtime tests include the actual shared GC helper fragment and execute it in
Wasmtime. They check Number bits, property snapshots and a self-referencing object
graph through repeated copying collections. A separate recorded verification ran
an actual emitted mixed-helper IR artifact in both Node and Wasmtime. That manual
artifact check does not substitute for maintained source differential tests.

Node requires standard WasmGC support; the project already selects Node 24. Use
the pinned Wasmtime copying collector from
[the runtime prerequisite](https://github.com/tseguine/grunkalunka/pull/301).
No new package version, host authority, public reference ABI or dedicated CI
workflow is introduced. Ordinary Cargo tests and existing package dependencies
select the compiler and runtime consumers of the shared helper file.

Source helper reference passing, arrays, strings, general function identities,
managed captures, suspended GC frames, public heap ownership and full coercion/
prototype behavior remain work under the original general-purpose JavaScript goal.
This source increment does not complete the value domain or justify counting
rejected programs as supported.
