# ADR 0008: Strict core-Wasm runtime composition

Date: 2026-10-25

Status: accepted

## Context

The owned JSON guest runs without imports, but generated programs need to call its
functions while using the same memory. This is the next engineering step toward
source-level JSON calls under ADR0007. The delegated PoC representation decision
policy covers this unit; compiler source admission remains separate.

## Decision

Add `grunkalunka-wasm-link::compose` for a
declared runtime/generated module profile. It validates both binaries, checks their
profiles, resolves internal function imports, composes sections and validates the
result. It performs no IO. Inputs remain borrowed and output bytes are owned.

The runtime has no imports or start/tag sections, one standard-page nonshared
memory32 exported as `memory` at index0, and implicit final nonshared function types
without supertypes or descriptors. Struct/array/continuation types and explicit
recursion groups reject. Generated modules import one `runtime.memory` without a
maximum and with a minimum no greater than the runtime's. Function imports resolve
only to same-named runtime function exports with exact scalar signatures. Generated
function types use i32/i64/f32/f64/v128 parameters/results. Generated globals,
tables, elements, tags, start functions, memory definitions and nonfunction exports
reject. This profile can grow in later bounded units; it is not a general linker.

Keep runtime index spaces unchanged. Append generated types and defined functions,
mapping imported function indices to resolved runtime indices and defined indices
past the runtime functions. Reencode generated bodies so helper calls and block
types are mapped too. Check combined counts fit the Wasm u32 format before mapping;
subtract imported functions before adding the runtime base to avoid intermediate
overflow near the index boundary. Duplicate export names fail.

Preserve runtime function-body bytes, data segment encodings, and table/memory/
global/element section payloads. Runtime type/function/export section headers and
encodings can change as entries are merged. Preserve only `name`, `producers` and
`target_features` custom payloads, in their relative order at the end. These retain
runtime provenance; they do not attest a complete producer/feature manifest for
generated code. Generated name metadata is omitted. Reject other custom sections,
including relocation, linking and debug metadata whose offsets could become stale.

### Literal data

Generated data must be passive. Append its segments after runtime data and map all
generated data indices, including `memory.init` and `data.drop`. Emit one merged
data-count before code. This follows the binary format's count and validation order:
see [Wasm data-count section](https://webassembly.github.io/spec/core/binary/modules.html#data-count-section).

Generated wrappers prepare a checked guest-owned buffer, then copy their passive
UTF16 literal bytes into it. This avoids placing active data over Rust's stack,
static objects or allocator area, and avoids imposing the disposable prototype's
fixed literal pool on the maintained guest. The normal numeric compiler path does
not call this composer and retains its original artifact bytes.

### Limits and trust

Callers supply inclusive per-input byte, output byte, metadata-item and generated
data-byte quotas. Item limits cover each parsed type/function/import/export/data
record vector, runtime table/global/element/custom record vectors and combined
type/function/export/data counts. Function limits include generated imports before
resolution; data bytes count generated payload bytes, including across segments.
Zero quotas are valid and may reject. Empty composition still validates profiles
and bindings; it can return an exact copy of the runtime, dropping unused generated
types/import declarations/names. Combined metadata accounting is conservative even
for that identity case. Input byte quotas run before binary validation; other
quotas can precede later profile/binding errors.

These are structural engineering quotas, not allocator, CPU or elapsed-time bounds.
The generated module is trusted compiler output sharing the runtime's memory.
Validation does not protect runtime memory from arbitrary generated stores. Engine
memory/execution limits, compiler store proofs, guest pointer protocols and trap
recovery remain host/integration obligations. Opaque owned JSON roots are not a JS
graph, GC or public source ABI.

## Scope and non-goals

This covers composition of the declared runtime/generated core-Wasm profile and its
maintained JSON integration fixture. Compiler lowering, source admission, the normal
numeric emission path and its ABI stay unchanged. A JS heap/object model, aliases,
GC, callbacks, exception integration and a public source JSON ABI remain separate.

## Alternatives considered

Host JSON imports would lose the self-contained artifact and bind behavior to
ambient objects. Rewriting JSON in WAT would duplicate tested grammar/formatting.
A general linker, active-data relocation and type deduplication add unnecessary
index/ownership cases here. Appending indices, preserving runtime bodies and using
passive data keep the current composition inspectable. Unsupported profiles fail
instead of being approximated. Disposable probe code was not copied into this unit.

## Consequences

The result is one self-contained module with checked internal bindings, while the
runtime retains its original indices and body bytes. Costs include parsing both
inputs, reencoding generated code and validating the result. Appended types can
duplicate runtime types. Unsupported metadata and module profiles require future
explicit decisions. Generated instructions retain access to shared runtime memory;
this decision provides no isolation or trap recovery between the two parts.

## Verification and evidence

Native TDD exercises identity, malformed binaries, quotas, exact binding, helpers,
block types, data/drop mapping, metadata and runtime table/global/element retention.
The real maintained JSON guest is composed with generated fixtures and the same
final bytes execute in Node and Wasmtime: alternating literal roundtrips, repeats,
numeric and primitive-pair calling shapes, instances and host-owned copies. Retained
runtime exports also pass the original guest oracle. Three completed fault
injections detect incorrect function, type and data index mapping. Two additional
mutation checks were inconclusive because Windows prevented replacement of the
test executable; those attempts do not count as behavioral evidence. Normal
profile/binding rejection tests remain part of the suite.
Run `node scripts/check-json-composition.mjs`;
the complete check includes it and the original guest check. Finite regression
evidence is not source-lowering or ECMAScript conformance evidence.

## Explicit boundary statement

This introduces validated composition for trusted generated code and one owned
JSON guest. It does not introduce general linking, a JS graph heap or source JSON
lowering. The source value representation and callable JSON semantics require
further decisions and tests.

## Related work and follow-up

ADR0005 and ADR0006 cover JSON data parsing and serialization; ADR0007 defines the
owned guest transport. Follow-up work includes source string/object/array
representation, source call lowering, and the wider ECMAScript JSON behaviors that
cannot be represented by isolated owned data roots.

## Summary

Keep runtime indices stable, resolve exact function bindings, and append mapped
generated functions and passive data. Validate the resulting single module and
reject unsupported profiles. Source-level JSON support remains unfinished.
