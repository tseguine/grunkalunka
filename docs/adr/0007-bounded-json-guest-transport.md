# 0007: Bounded owned JSON guest transport

Status: accepted for the PoC engineering unit; no compiler or JavaScript heap ABI
adoption. The user delegated representation decisions with recorded alternatives.

## Context and decision

The UTF16 parser and serializer are tested native libraries. Generated code needs
an executable guest boundary before source admission can use them. A disposable
experiment combined real Rust JSON and handwritten generated functions into one
import-free core-Wasm module. That established feasibility for a strict module
profile, not a maintained linker or a JavaScript heap. This unit is freshly written.

Add `grunkalunka-json-guest` with a safe native `JsonGuest` and wasm32-only exports.
Each module instance owns input/output buffers and at most 64 live JSON roots.
The root table uses a BTreeMap keyed by monotonically increasing nonzero u32 IDs.
IDs never reuse, including after release; after u32 exhaustion, creation fails.
No child identity, mutation, cycles, closures or collection policy is selected.
IDs belong to one instance; passing an ID from another instance is a protocol error
that this integer representation cannot generally detect.

Input is initialized UTF16, at most 32,768 units. Parser limits are depth128,
4,096 visited values and 1,048,576 work units. Output is at most 65,536 UTF16 units,
with the same serializer depth/value/work profile. These are engineering quotas
for this guest profile, not new ECMAScript restrictions or exact allocator bounds.
The host must separately bound Wasm memory and execution when integrating untrusted
work. Allocation failure and Rust panics remain traps/process behavior.

Input preparation clears old input and output, including failure. Parse, stringify
and release clear output before their work; errors preserve other roots. Parsing
at the root limit can fail before syntax validation. Querying status, lengths and
pointers does not clear the last status. Successful mutations reset status/offset.
The error offset is the parser's associated UTF16 position, not engine text.

## Core-Wasm protocol

The guest has an exported nonshared memory, no imports, no callbacks and no host
provided raw pointer. All lengths are code units. Host code calls input preparation,
checks pointer/status/range/alignment, writes only that initialized input region
between synchronous calls, then parses to an opaque root ID. Serialize by ID, read
output pointer/length and make an owned host copy before the next mutating call.
Release IDs explicitly. Pointers are memory32 offsets, not native addresses.

| Export | Signature | Result |
| --- | --- | --- |
| `gk_json_input_resize` | `(i32) -> i32` | Input pointer, or zero on error; zero-length success has a nonzero sentinel pointer |
| `gk_json_input_len` | `() -> i32` | Prepared input length |
| `gk_json_parse` | `() -> i32` | Nonzero root ID, or zero on error |
| `gk_json_stringify` | `(i32) -> i32` | Status code |
| `gk_json_release` | `(i32) -> i32` | Status code |
| `gk_json_output_ptr` | `() -> i32` | Output pointer; ignore data when length is zero |
| `gk_json_output_len` | `() -> i32` | Output length |
| `gk_json_status` | `() -> i32` | Last mutation's status |
| `gk_json_error_offset` | `() -> i32` | Associated parser position; zero for other outcomes |

Status codes: 0 success, 1 input quota, 2 live-root quota, 3 exhausted IDs,
4 invalid/released ID, 5 JSON syntax, 6 parser quota, 7 serializer quota,
8 internal data/config invariant. These are transport statuses, not JS exceptions.
ADR0009 extends this transport with raw String construction/readback and status 9
for a live non-String root; the original statuses and operations retain their meaning.

Wasm exports use a per-instance `Mutex<Option<State>>`; native tests run the safe
state directly. This module is nonshared, synchronous and import-free, so there is
no supported concurrent host access or callback reentry. There are no unsafe Rust
blocks; symbol export attributes are explicit unsafe declarations. A host can
access all exported guest memory: this is a cooperating host protocol, not a
security barrier against a malicious host writing arbitrary memory.

## Alternatives and consequences

A host JSON import would share engine facilities but lose the self-contained
artifact and make cross-engine semantics depend on ambient host objects. Rewriting
the parser in WAT would duplicate tested grammar and number-formatting work.
Caller-provided raw pointers would require validating arbitrary guest slices and
their ownership; preparing a guest-owned buffer keeps that surface smaller.
Reusing IDs could mistake a stale ID for a different root; a generational table
would add a wrap/retirement protocol without a current need. Monotonic IDs keep
release simple while live metadata stays bounded by the root quota.

A full JS graph/GC would support aliases and cycles but is a separate semantic
unit. This owned-root guest cannot stand in for JS heap identities. Compiler
admission, heap operators, property access, source-generated runtime composition,
callbacks and JS exception mapping remain requirements.

## Verification

Native TDD covers input initialization, independent roots, quotas, release,
nonreuse and terminal u32 exhaustion; failed operations clear output. Actual Wasm
runs in Node and Wasmtime against original UTF16 input and pristine Node JSON
output, including every code unit, finite binary64 samples, lone surrogates,
integer-key order, duplicate keys, depth/value/work/output/input boundaries,
500 repeated lifecycles and independent instances. The checker uses the same
artifact and preserves its hash plus oracle records under `target/json-guest`.
Run `node scripts/check-json-guest.mjs` or the complete `node scripts/check.mjs`.
Finite regression evidence does not prove full ECMAScript JSON conformance.
