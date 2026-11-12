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
