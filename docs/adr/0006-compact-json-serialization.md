# ADR 0006: Compact serialization of owned JSON data

Status: accepted for the 0.0.1 implementation.

## Context

The reusable parser now produces lossless UTF-16 JSON trees. A matching data
serializer can establish number formatting, escaping and property order before
the compiler admits heap values. General `JSON.stringify` additionally observes
callbacks, property operations and other JS value kinds; an owned tree cannot
implement those behaviors by itself.

## Decision

Serialize the six owned data kinds to compact UTF-16 output. Follow ECMAScript
JSON quoting, preserve surrogate pairs and escape lone surrogate code units with
lowercase hexadecimal. Preserve array order. For objects, order canonical array
index keys numerically, then retain the other keys' insertion order. The maximum
array index is `4294967294`; `4294967295` and `-0` are ordinary string keys.

Require unique object keys. The parser already enforces this by replacing
duplicates. Since the public data enum also permits manual construction, return
a distinct internal `DuplicateKey` error rather than emitting ambiguous repeated
properties. This does not change parsing or define a language-level exception.

Use pinned `ryu-js` 1.0.3 for finite binary64 formatting through its safe API.
Render negative zero as `0` and nonfinite numbers as `null`. Select Apache-2.0
from the release's `Apache-2.0 OR BSL-1.0` expression. Default features are off;
the optional `no-panic` dependency is disabled. The published manifest has no
build script. The archive checksum is
`04d056b875a9d2e6cb9a61d127afee9ac5999b9f87bcb32079d1318e505be714`.
No existing dependency versions change.

Use one checked output writer instead of first building unbounded child strings.
Require explicit output-unit, value, depth and work quotas, with an implementation
depth ceiling of 128. Meter key comparisons before performing them, classify each
key once, and charge an ordering allowance before allocating sort metadata.
The exact accounting and error boundary are in [JSON notes](../json.md).
These quotas do not promise allocator capacity or CPU instruction counts.

## Alternatives

- **Rust's ordinary float display:** does not provide ECMAScript's exponent
  notation policy. It is not a substitute for verified JS number formatting.
- **Implement decimal formatting ourselves:** gives control but adds substantial
  rounding and shortest-representation risk. Reuse a focused, permissively
  licensed implementation and verify actual output against Node.
- **Reuse a UTF-8 JSON serializer:** still needs a lossless surrogate path and
  verified number/property-order behavior. The existing UTF-16 data representation
  makes the small structural writer straightforward.
- **Preserve vector order for all object keys:** preserves insertion metadata but
  disagrees with ordinary JS property enumeration for array index names.
- **Implement full JS callbacks in the data serializer:** requires object identity,
  callable values and ordered observable property operations that this tree lacks.
  Keep those runtime requirements visible rather than approximate them here.

## Verification and limits

Native tests compare exact output code units with Node for raw binary64 values,
the full code-unit range, surrogate sequences, index boundaries and nested trees.
Separate tests pin quota boundaries and malformed manual objects. Semantic mutants
must expose number, escaping, ordering and metering errors. Wasm library builds,
full workspace checks, complete-diff review and unchanged prior compiler artifacts
are integration gates. Finite differential samples are not a proof over all values.

This adds no compiler source admission, heap ABI or JS callback execution.
Replacer functions/arrays, `space`, `toJSON`, unsupported value kinds, cycles,
modern raw-JSON values, guest execution and exception mapping remain separate work.

Sources: [ECMAScript JSON.stringify](https://tc39.es/ecma262/multipage/structured-data.html#sec-json.stringify),
[ordinary property keys](https://tc39.es/ecma262/multipage/ordinary-and-exotic-objects-behaviours.html#sec-ordinaryownpropertykeys),
[array index definition](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#array-index)
and [ryu-js](https://github.com/boa-dev/ryu-js).
