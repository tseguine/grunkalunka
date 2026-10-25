# JSON implementation boundary

JSON-compatible source values, `JSON.parse` and `JSON.stringify` are the next
lowering priority. They are not yet admitted by the compiler. The existing
General ABI still admits only Undefined, Null, Boolean and Number.

`grunkalunka-json` supplies the first reusable part: a dependency-free Rust parser
for an owned JSON data tree. It accepts UTF-16 input, preserves string and key code
units including lone surrogates, and produces Null, Boolean, binary64 Number,
String, Array and Object values. Numbers preserve negative zero and can overflow
to infinity, as in ECMAScript JSON parsing. Objects replace duplicate values while
retaining each key's first insertion position; `__proto__` is ordinary data.
Integer index property enumeration is a later operation, not the vector order.

The parser requires explicit limits. Input length counts UTF-16 code units;
container depth is inclusive and may be configured from zero through 128. Scalars
have depth zero. Every visited value counts, including the object/array itself
and duplicate values that are subsequently replaced. The work quota charges the
entire input length before scanning, one per visited value, and
`min(existing_key_length, new_key_length) + 1` before every duplicate-key comparison.
The conservative comparison charge bounds the otherwise quadratic key search.
It is deterministic engineering accounting, not a CPU instruction, elapsed-time,
or allocator guarantee. Callers choose their limits; no platform defaults are set.

Errors distinguish invalid limits, syntax and input/depth/value/work exhaustion.
The error offset is an associated UTF-16 position and can identify the beginning
of an invalid token. It does not reproduce engine error messages. A resource
failure may occur before malformed input is recognized; it does not certify valid
syntax. Allocation failure retains Rust's ordinary process behavior.

Run the parser's native tests and build its library for core Wasm with:

```sh
cargo test --locked --offline -p grunkalunka-json
cargo build --locked --offline -p grunkalunka-json --target wasm32-unknown-unknown
```

The native oracle executes original UTF-16 JSON texts with Node's `JSON.parse`
and compares structural snapshots, exact number bits and string code units.
Separate tests check insertion metadata and quota boundaries. The Wasm library
build establishes target compatibility, not guest execution or compiler lowering.
The workspace's normal check runner includes the native tests.

Remaining units are an ECMAScript-compatible serializer (including number
formatting, integer key order and well-formed surrogate escaping), an import-free
guest runtime and checked heap transport, then compiler admission for strings,
arrays, objects and the builtins. Existing truthiness, equality, conversion and
addition operations need heap-kind semantics before admission. Reviver/replacer
callbacks, `space`, `toJSON`, property access, aliases, cycles, exceptions and
modern reviver source context remain requirements; an owned parser tree does not
provide them. Persistent heap lifetime/GC, closures and module state require
their own work. No conformance or lowering gap is closed by this parser alone.

See [ADR 0005](adr/0005-json-data-parser.md) for the decision and alternatives.
