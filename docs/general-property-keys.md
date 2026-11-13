# General boxed property keys (initial runtime path)

[ADR 0022](adr/0022-general-js-ts-lowering-first.md) establishes guest-owned
runtime operations as the default when no static specialization proof exists.

The IR now carries a `ManagedKey::Value` computed key and normalizes its runtime
value in guest Wasm. Undefined, Null, Boolean, Number and String use JavaScript
ToPropertyKey spelling (including Number formatting and UTF-16 String identity).
For the initial front-end path, computed reads and writes on
`Object.create(null)` dictionaries use this operation when the source-key kind
cannot be proven; the existing known-key fast paths are retained. Receiver, key
and value are evaluated once in order; data identity is preserved.

This is deliberately a **partial general operation**, not a statement of
full JavaScript compatibility. Object/Array/function key coercion, Symbol
property keys, inherited property lookup, accessors, proxies, Array length writes
and arbitrary receivers require additional guest semantic operations and
catchable completion propagation. Unsupported boxed key kinds currently
terminate at an explicit guest operation boundary rather than silently
delegating to host JavaScript. These paths do not count as successfully
executing programs in the lowering-coverage census.

Source-backed compiler checks and native-JS/Node-Wasm/Wasmtime comparisons cover
mixed primitive/String keys, effects and TypeScript erasure.

## Boxed Array reads

A follow-up routes runtime-boxed primitive/String keys to the existing
sparse-Array getter, including canonical numeric indices and `length`.
It preserves source order and boxed values without a static key-kind proof.
Dynamic Array writes are still guarded until the guest has catchable
length-error semantics. Ordinary-object prototype Get/Set and accessors
require a separate fallible general operation and are not claimed here.

## Boxed keys for `Object.hasOwn`

The ordinary own-property membership intrinsic uses the same guest
ToPropertyKey subset when a computed key has no source-proven primitive or
String category. This path works with source-known ordinary objects,
null-prototype dictionaries, arrays and supported String/Number/Boolean
receivers. It distinguishes inherited prototype names from own fields, array
holes from stored Undefined, and String UTF-16 indices from missing names.
Receiver evaluation still precedes key conversion and extra argument effects.

Unlike general property Get/Set, `Object.hasOwn` only asks about own data.
No prototype getter or setter is invoked. Known object/Array/function keys
remain guarded until a fallible ToPrimitive implementation exists. Hosted
tests compare original JS with compiled Wasm for changing runtime key kinds,
extra arguments, TypeScript erasure and inherited-name misses.

## Boxed-key `in` over null-prototype dictionaries

The general key normalization operation also handles the left operand of
`key in dictionary` when the right operand is positively known to be a
null-prototype dictionary. It evaluates the key before the saved receiver,
then checks actual own-property membership with the same guest table as
`Object.hasOwn`. A property added using that boxed key can be tested for
presence without restricting its source expression to one static kind.

This increment does not change `in` for ordinary objects, Arrays or
prototype-bearing receivers: those still require full prototype membership
semantics for runtime-dependent keys. Source-known object/function keys
remain guarded pending ToPrimitive/completion support. JS/TS-vs-Wasm
tests exercise mixed primitive/String keys, absent names, saved effects
and identity-independent own-name lookup.

## Boxed dictionary deletion

The general String/primitive key operation now also feeds the existing
`delete dictionary[key]` path when the receiver is a null-prototype
dictionary. Stored data properties in this admitted domain are configurable,
and an absent key returns true without changing unrelated fields. A write
and subsequent delete of the same mixed-kind runtime key preserve receiver
identity and source evaluation order entirely in guest Wasm.

General object-key ToPrimitive, ordinary-object prototype behavior,
non-configurable descriptors and dynamic Array length deletion remain outside
this bounded operation; the existing source guards are retained. Tests
compare unchanged JS and erased TypeScript against both Wasm engines.

## General boxed `in` with guest-owned prototype membership

The existing `ManagedHasProperty` operation now accepts the shared
`ManagedKey` carrier. Its emission preserves the existing proven String
fast path exactly; the boxed-key fallback calls the same guest ToPropertyKey
conversion used by Get/Set/Has/Delete. Runtime String keys then test both
own properties and the guest's already-maintained intrinsic prototype-name
tables on ordinary objects and Arrays. The same operation respects the
null-prototype dictionary subtype, where inherited names remain absent.

This removes the static key-kind requirement for `in` even when the
primitive-or-String choice changes at runtime, without needing an alternative
control-flow implementation or ambient host-JavaScript lookup. Known
object/function keys still need fallible ToPrimitive/completions; arbitrary
prototype mutation, accessors, Symbols and proxies are not claimed.

## Boxed delete on ordinary own-data objects

The existing managed own-property delete operation now also accepts
runtime-boxed primitive/String keys on source-proven ordinary data objects.
These objects and null-prototype dictionaries both use configurable own
entries; deleting an absent or inherited name returns true and does not
change their prototypes. This is a reusable general key-normalization path,
not a new source-specific inference rule.

Array deletion still requires a key proof that excludes the non-configurable
`length` property, while object/Array/function key coercion still needs a
fallible ToPrimitive operation. Added comparisons exercise missing/inherited
keys, aliases, evaluation order and TypeScript erasure.

## Optional computed own-data reads

The same guest-owned boxed-key normalization can now be selected for an
optional computed member read when the non-nullish receiver is a proven
null-prototype data dictionary or Array, even when the source key does not
have a single provable kind. The owned expression cursor evaluates the
receiver once and only evaluates/converts the key in the non-nullish branch.
The general path is shared with ordinary computed own-property reads rather
than maintained as another AST-specific converter.

This does not add generic access to inherited properties, optional calls or
object-valued keys requiring ToPrimitive. The existing positive-proof paths
remain the preferred specialization. Rust admission/rejection tests and
native-JS/Node-Wasm/Wasmtime checks cover sparse indices, Array length,
nullish key suppression, mutation of the receiver during key evaluation,
and TypeScript type-erasure behavior.

## Computed object-binding keys

The same boxed runtime key selector is now reused by computed object binding
patterns over source-known data dictionaries and Arrays with safe own keys. It supplies the
already-existing managed selection and default/rest machinery rather than
requiring a separate source-kind proof before the pattern can be lowered.
Saved receivers, computed-key evaluation order and exclusions from object
rest continue through the typed IR.

Array intrinsic names, prototype-bearing ordinary objects, coercion of object/function keys,
unresolved receivers, and user-defined accessors retain their previous
guards. Source and native-JS-vs-Wasm tests cover sparse Array and length
names, mixed primitive/String choices, defaults, rest exclusion, effects
and TypeScript erasure.

## Logical writes with boxed dictionary keys

Null-prototype data dictionaries now use the same guest-owned boxed runtime
key normalization for computed `&&=`, `||=` and `??=` as for plain assignment.
The existing logical-write cursor snapshots the receiver and key once, reads
the old own value, and evaluates the right-hand expression only when selected.
Undefined, Null, Boolean, Number and String keys can alternate at runtime
without a single source-proven kind. Both key conversion and the conditional
write remain in guest Wasm.

Ordinary objects still require their existing proof for property lookup,
and Arrays retain the length-write restriction. Keys that can be objects,
Arrays or functions are not admitted by this primitive-only conversion.
Source admission tests and JS/TS differential runtime comparisons cover
the operators, saved key effects and source-reference guards.

## Numeric compound updates with boxed dictionary keys

Numeric property updates on null-prototype data dictionaries now use the same
boxed runtime key operation as direct Get/Set and logical assignment. Prefix
and postfix updates, and the existing numeric compound operators, snapshot
the receiver and key exactly once before reading the old numeric value.
The compiler retains positive primitive-value and arithmetic admission
checks; a runtime-selected primitive/String key alone does not imply that
the stored field has safe numeric coercion.

The extension keeps array length, inherited object properties, and reference
keys needing ToPrimitive outside the partial operation. Native JavaScript,
Node-Wasm and Wasmtime differential tests cover mixed key kinds, assignment
operators, side effects and TypeScript erasure.

## Optional delete with boxed object keys

Optional computed deletion on known data objects can now select a boxed
primitive/String key at runtime. The key is normalized only after the receiver
passes the nullish check, and only once, using the same guest operation as
non-optional object deletion. The saved receiver is used even if evaluating
the key mutates its source binding; deletion of missing or inherited names
returns true without a prototype lookup.

Array length remains non-configurable, so unproven Array keys still cannot
take this fallback. Reference-valued keys requiring fallible ToPrimitive
also remain guarded. Compiler and native-JS/Wasm differential tests cover
nullish short-circuiting, key effects, ordinary and null-prototype objects,
and TypeScript erasure.

## Computed keys in object assignment patterns

Object assignment patterns now share the boxed runtime key selector used by
object binding declarations. A computed source key may select between supported
primitive/String kinds at runtime when the right-hand source is a proven
null-prototype data dictionary. The saved source object, ordered key evaluation,
assignment destination and object-rest exclusion remain on the existing
owned cursor; key conversion is guest-owned.

Array source keys retain the same inherited-name guard as binding declarations.
Ordinary prototype-bearing objects and keys needing fallible ToPrimitive remain
outside this partial own-data operation. Compiler and unchanged native-JS versus
Wasm comparisons cover defaults, rest, key effects and TypeScript erasure.

## Computed assignment destinations

Object and Array assignment patterns can also write to a computed member
destination on a proven null-prototype dictionary when its key alternates
between primitive and String values. The planner now carries the validated
runtime key kind with the destination instead of re-inferring a narrower kind
when executing the saved assignment. The receiver and key are evaluated once,
in the original assignment order, and the selected value is written using
the same guest-owned dynamic Set operation as plain assignment.

This does not enable unchecked Array length assignment or arbitrary prototype
writes, and source-proven object/function key alternatives still require a
fallible ToPrimitive path. Compiler and JS/TS differential tests cover dynamic
destination keys, defaults, object patterns, effects and reference guards.
