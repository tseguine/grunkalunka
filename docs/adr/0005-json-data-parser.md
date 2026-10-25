# ADR 0005: Reusable UTF-16 JSON data parser

Status: accepted for the 0.0.1 implementation.

## Context

The user prioritized JSON-compatible values and both `JSON.parse` and
`JSON.stringify` within the broader JS/TS lowering goal. Current General values
have four immediate primitive kinds. JSON introduces strings, arrays and objects;
existing operators assume the four-kind representation and cannot safely admit
heap kinds without further work.

ECMAScript strings are sequences of 16-bit code units, including lone surrogates.
The JSON grammar is narrower than JavaScript expression syntax. JSON parsing uses
binary64 numbers, preserves negative zero, can overflow to infinity, and replaces
duplicate property values. Ordinary Rust UTF-8 strings alone cannot represent the
full string domain.

## Decision

Implement the grammar and owned data representation in a small reusable Rust
crate, with UTF-16 vectors for text and keys. Keep it independent of the compiler,
Wasmtime and ambient host JSON implementations. First verify it natively against
original JSON texts in Node, and build the same library for
`wasm32-unknown-unknown`. Guest execution and compiler admission are separate units.

Use an owned tree for this data stage. Object entries retain first insertion
metadata and replace duplicate values in place. This is not a general JS object
model, and does not implement integer key enumeration, prototypes, identity or
cycles. A bounded linear key search keeps the implementation small while explicit
work charges limit hostile repeated comparisons.

Require caller-provided input, depth, visited-value and work quotas. Limit recursive
container depth to 128 even if the caller requests more. Distinguish configuration,
resource and syntax errors; do not confuse quota exhaustion with a language-level
`SyntaxError`. Document error offsets without promising engine-specific diagnostics.
The quota definitions and unresolved runtime work live in [JSON notes](../json.md).

## Alternatives

- **Use UTF-8 strings throughout:** fits Rust text APIs, but loses lone surrogate
  code units or requires another lossless representation. UTF-16 directly matches
  the source string domain.
- **Delegate JSON to a host import:** gives a convenient oracle, but makes emitted
  artifacts depend on ambient host JSON and does not establish portable guest
  semantics. Host engines remain verification oracles.
- **Handwrite the parser in WAT:** avoids a Rust guest build, but duplicates semantic
  code and makes native tests and maintenance harder. Rust already targets core Wasm.
- **Adopt a conventional JSON tree crate immediately:** would still require verified
  handling of UTF-16, binary64 overflow, duplicate metadata and bounded work. The
  present parser needs no external dependencies. Specialized libraries remain an
  option where they solve a verified requirement, such as number formatting.
- **Build a general JS heap first:** could unify identity and property operations,
  but entangles grammar verification with lifetime, aliases and compiler changes.
  A reusable parser can be checked independently and later materialized in the heap.

## Consequences and verification

The data tree allocates through Rust's ordinary allocator and is not a guest ABI.
A successful Wasm library build alone proves neither execution nor heap transport.
The earlier disposable arena experiment informed feasibility, but its handwritten
fixture is not production code or proof of source lowering.

Native tests cover grammar, strings and keys, exact number bits, duplicates,
nesting and resource boundaries; a deterministic Node differential corpus adds
rounding and malformed-text comparisons. Mutation probes must show that critical
grammar, replacement and quota defects are detected. Full workspace checks and
unchanged prior compiler artifacts remain integration gates.

Sources: [ECMAScript JSON.parse](https://tc39.es/ecma262/multipage/structured-data.html#sec-json.parse),
[JSON.stringify](https://tc39.es/ecma262/multipage/structured-data.html#sec-json.stringify)
and [String type](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-ecmascript-language-types-string-type).
