# ADR 0009: Owned raw UTF-16 String roots

Date: 2026-10-25

Status: accepted

## Context

ADR0007 transports owned JSON roots and ADR0008 composes generated functions with
the guest. Generated string literals need a raw UTF-16 construction/readback path
before source lowering can use that runtime. Serializing each literal into JSON
text and parsing it again adds unnecessary escaping and grammar work.

The external Oxc extraction study and its independent replay preserve lone
surrogates through `JSStr::encode_utf16()`. This decision supplies the guest side
of that boundary. It does not change ADR0002's four-kind primitive ABI.

## Decision

Add native `JsonGuest::string_from_input()` and `string_units(id)`. The factory
copies the current initialized input units into an owned `JsonValue::String` root.
It uses the same 64-live-root quota and monotonic nonzero u32 IDs as parsing.
IDs are never reused; exhaustion remains terminal. Both constructors share one
table and counter, and release reclaims the common live quota.

Readback accepts factory-created and parsed String roots. It copies their raw
units into the separate output buffer and returns that buffer, without JSON
quotes, escaping or Unicode replacement. Validate the ID first; a missing/released
ID is `InvalidHandle`, while a live non-String root is `NotString`.

Both new operations clear output before work. Failure preserves all roots and
the input. Successful construction also preserves input, allowing repeated use.
Readback leaves the root unchanged. Host copies remain independent of later guest
mutations. Strings cannot exceed 32,768 units because both constructors use the
existing input bound; raw readback therefore fits the 65,536-unit output bound.
JSON quoting may still exceed its output quota; raw readback of that root remains
valid after such a failure.

| New export | Signature | Result |
| --- | --- | --- |
| `gk_json_string_from_input` | `() -> i32` | Nonzero owned root ID, or zero with status |
| `gk_json_string_units` | `(i32) -> i32` | Status; raw units available through existing output queries on success |

Add status 9 for a live non-String root. Existing statuses 0–8 retain their meaning.
Mutating calls record their outcome and reset the parser offset to zero except
for parser errors. Query calls retain the last status. Input preparation still
clears input on failure; a subsequent factory call therefore creates an empty
String if root resources permit. Callers must check preparation before writing.

## Scope and non-goals

This is an extension of owned data transport. There is no compiler admission,
String wire tag, pointer payload, generic heap ABI, alias graph or collector
decision. Existing `ValueInstance` rejects unknown primitive tags as before.
Equality, concatenation, property access, coercions and source calls remain separate.
No dependencies or source resource limits change.

## Alternatives considered

Parsing a quoted JSON literal can construct the same value, but requires extra
encoding and couples raw-string construction to JSON syntax and output expansion.
Returning pointers into root storage would add borrowing and lifetime obligations;
the copied output reuses the established checked-buffer protocol. A separate String
table would duplicate root ownership and quota accounting. The shared owned table
is sufficient without implying JS object identity or a graph layout.

## Consequences

Literal emission can copy passive UTF-16 data into checked guest input and obtain
an owned String root directly. Construction and readback each copy at most the
input quota. These bounds are engineering quotas, not allocator capacity or exact
instruction guarantees. IDs remain instance-local; cross-instance misuse cannot
generally be detected. Rust allocation failure/panics and engine traps retain the
existing behavior and host obligations from ADR0007.

## Verification and evidence

Native TDD covers empty/raw/surrogate data, input overwrite, parsed String roots,
wrong-kind and stale IDs, shared root quotas, terminal ID exhaustion, failed input
preparation, and readback after JSON output expansion failure. The actual guest
runs in Node and Wasmtime against the same seven raw String oracle records,
including all 65,536 code units in input-sized chunks, with pristine Node JSON quoting
as a second observation. Each engine checks 500 String lifecycles, mixed constructor
quotas, copies and instances, plus the original 1,130 JSON oracle records. The
composed artifact also runs the retained guest checks.

Run `node scripts/check-json-composition.mjs` or the complete check runner. Finite
regression evidence does not establish source lowering or full JSON conformance.

## Explicit boundary statement

This introduces lossless owned String construction and inspection inside the data
guest. It does not expose Strings through the primitive host decoder or implement
JS heap behavior. Source value representation and generic operation semantics
still require explicit decisions and verification.

## Related work and follow-up

ADR0005/0006 define JSON data parsing/serialization; ADR0007 defines root transport;
ADR0008 defines composition. Next work includes a heap-capable host profile and
typed source/IR/backend integration for String literals, with audits of every
generic operation before additional kinds can reach it.

## Summary

Reuse the owned JSON root table and checked buffers to transport raw UTF-16 Strings
without JSON parsing. Preserve code units, lifecycle rules and the current compiler
boundary; reject wrong kinds explicitly.
