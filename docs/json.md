# JSON implementation boundary

JSON-compatible source values, `JSON.parse` and `JSON.stringify` are the next
lowering priority. They are not yet admitted by the compiler. The existing
General ABI still admits only Undefined, Null, Boolean and Number.

`grunkalunka-json` supplies a Rust parser and compact serializer for an owned JSON
data tree. The parser uses only the Rust standard library. It accepts UTF-16
input, preserves string and key code units including lone surrogates, and produces
Null, Boolean, binary64 Number, String, Array and Object values. Numbers preserve
negative zero and can overflow
to infinity, as in ECMAScript JSON parsing. Objects replace duplicate values while
retaining each key's first insertion position; `__proto__` is ordinary data.
The serializer orders array index keys separately from this vector order.

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

Compact serialization returns UTF-16 output with ECMAScript number text (`-0`
becomes `0`; NaN and infinities become `null`). It escapes JSON controls and lone
surrogates while preserving valid pairs. Array index keys from `0` through
`4294967294` appear in ascending numeric order; other keys retain creation order.
Noncanonical numeric spellings and `4294967295` are ordinary string keys.
Manually constructed objects with duplicate keys return `DuplicateKey`; parser
output already has unique keys. This is an internal data invariant, not a JS error.

`StringifyLimits` requires output-unit, container-depth (zero through 128), visited
value and work quotas. Output length is checked before each append. Temporary
ordering metadata is bounded by the value/work quotas; allocator capacity and
process memory are not promised to equal the output quota. Work charges one per
visited value, one per output unit, each quoted string/key's input length, each
object key's length again for index classification, and
`min(previous_key_length, key_length) + 1` for every duplicate comparison. An
object with `n > 1` keys also pays `n * ceil(log2(n))` before ordering; smaller
objects pay zero ordering charge. This is a deterministic ordering allowance,
not a measured comparison or instruction count. Errors distinguish invalid limits,
quota exhaustion and duplicate keys. A quota can stop validation first.

Finite-number formatting uses pinned `ryu-js` 1.0.3 through its safe buffer API,
with Apache-2.0 selected and no optional features. Its archive checksum matches
the lockfile and published registry. This adds no enabled transitive dependencies.

Run native JSON tests and build the library for core Wasm with:

```sh
cargo test --locked --offline -p grunkalunka-json
cargo build --locked --offline -p grunkalunka-json --target wasm32-unknown-unknown
```

The native parser oracle executes original UTF-16 JSON texts with Node's `JSON.parse`
and compares structural snapshots, exact number bits and string code units.
Separate tests check insertion metadata and quota boundaries. The Wasm library
build establishes target compatibility, not guest execution or compiler lowering.
The workspace's normal check runner includes the native tests.

Serializer oracles compare actual output code units with pristine Node
`JSON.stringify`, including 100,064 raw binary64 inputs, 1,227 structured cases,
all 65,536 code units, nesting and index boundaries. These deterministic finite
comparisons are regression evidence, not exhaustive language conformance.

`grunkalunka-json-guest` now executes this library in an import-free core-Wasm
module with bounded UTF16 buffers, at most 64 live owned roots, nonreused instance
IDs and explicit release. `node scripts/check-json-guest.mjs` builds it offline
and runs the identical artifact in Node and Wasmtime; normal checks include it.
The Rust `wasm32-unknown-unknown` target must already be installed. This is owned
JSON data transport, not compiler admission or a JavaScript heap. See
[ADR 0007](adr/0007-bounded-json-guest-transport.md) for the protocol, quotas,
host obligations, status codes and alternatives.

`grunkalunka-wasm-link` can now combine that guest with generated core-Wasm
functions into one import-free module. It resolves internal runtime calls and
remaps function/type/data indices; passive UTF16 literals copy into checked guest
buffers. `node scripts/check-json-composition.mjs` checks the same composed bytes
in both engines, including the retained guest oracle. These generated fixtures
are handwritten Wasm, not source-lowered JS/TS. See
[ADR 0008](adr/0008-strict-core-wasm-composition.md) for the strict profile,
metadata policy, quotas and memory-sharing trust boundary.

Remaining units include compiler-generated runtime calls and admission for
strings, arrays, objects and the builtins.
Existing truthiness, equality, conversion and
addition operations need heap-kind semantics before admission. Reviver/replacer
callbacks, `space`, `toJSON`, property access, aliases, cycles, exceptions and
modern reviver source context remain requirements; an owned parser tree does not
provide them. Persistent heap lifetime/GC, closures and module state require
their own work. No conformance or lowering gap is closed by this parser alone.

See [ADR 0005](adr/0005-json-data-parser.md) and
[ADR 0006](adr/0006-compact-json-serialization.md) for the decisions and alternatives.
