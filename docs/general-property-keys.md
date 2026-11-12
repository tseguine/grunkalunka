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
