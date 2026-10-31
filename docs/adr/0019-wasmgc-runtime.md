# ADR 0019: Enable WasmGC for the general-value implementation

Date: 2026-10-31

Status: accepted target direction and bounded runtime prerequisite; source value
encoding and general heap integration remain incremental implementation work.

## Context

The maintainer selected **WasmGC first** for the compiler's general-JavaScript
prototype after comparing it with an owned linear-memory heap. The starting
baseline is `a8eae3a`, with Wasmtime 50.0.0 pinned and its GC features disabled.
The existing Node CI prerequisite is Node 24. Coordination and stream ownership
are recorded in [#300](https://github.com/tseguine/grunkalunka/issues/300).

The structured IR already separates snapshots from source cells and preserves
ordered effects. Its backend layout currently has only numeric lanes; closure
cells/environments and suspended frames use linear memory. The String/JSON path
bypasses that IR and owns JSON trees rather than shared mutable JS graphs.
These observations justify extending the representation, not replacing the IR.

## Decision

Use engine-managed WasmGC references for the new general-value path. Keep proven
Number operations as direct f64 operations and preserve current primitive export
protocols. Do not hide managed references in numeric payloads or raw byte spills.

Enable the pinned Wasmtime `gc-copying` feature. It also enables GC support and is
the only compiled collector, so the pinned engine's automatic selection uses the
copying collector. That collector supports cyclic graphs; the deferred reference
counting collector does not reclaim cycles, and the null collector does not reclaim
garbage. Collector latency and space costs still require workload measurements.

The first implementation change enables runtime execution of hand-built GC core
modules through the existing numeric boundary. It does not change source admission
or expose managed references to callers. Use Node's standard WasmGC support for the
differential tests; an older engine's partial experimental implementation is not
an equivalent target.

## Scope and non-goals

This unit changes a Cargo feature and adds canonical runtime tests. It adds no
package version, host import, new workflow, source lowering path or public object
ABI. Existing numeric/primitive source programs retain their generated encoding.
The current structured IR, evaluation order, captures and async semantics remain.

The eventual value domain includes all eight JS language kinds. Functions,
arrays and Promises are object kinds; strings and BigInts compare by value,
while Symbols and objects have identity. This direction does not select a complete
physical union, implement those kinds, or require every known primitive to be boxed.

## Alternatives considered

- A guest-owned linear-memory heap with an arena followed by tracing collection
  retains current numeric-only storage/composition, but adds collector/rooting work.
  The maintainer preferred prioritizing WasmGC instead.
- Deferred reference counting has different latency/throughput tradeoffs, but its
  current inability to collect cycles is a poor default for general JS graphs.
- A null collector can suit disposable bounded runs, but does not provide the
  reclamation behavior needed by the intended application direction.
- Host-JavaScript objects remain reference evidence, not the guest implementation.

## Consequences

Managed-reference locals, globals and GC container fields can keep graph edges
visible to the engine. Extending captures and coroutine frames therefore needs
reference-aware storage; the existing numeric-only linear-memory spill plan cannot
be reused unchanged. Saved-reference protection and owner lifetimes must be tested
across calls, collection and suspension before those source cases are admitted.

The strict String/JSON composer rejects GC types and generated tables/globals.
Execute the first GC modules directly. Integrating Rust String/JSON helpers later
needs an explicit transport/composition decision; this ADR does not relax the linker.
One-instance-per-invocation adapters also do not establish a shared application heap.

## Verification and evidence

An ordinary runtime test first failed because the pinned runtime rejected struct
types with GC disabled. Enabling the cached pinned feature made execution succeed.
No new dependency download or Cargo.lock change was needed.

The retained fixture allocates mutable struct objects and a reference array, checks
alias and fresh-object identity, mutates through an array edge, and constructs a
self-cycle. It returns through the existing numeric ABI. Native JS and Node-Wasm
compare the same observations over 64 input pairs, including signed zeros, NaN,
infinities and the smallest positive Number. Wasmtime also forces repeated copying
collections and checks the globally rooted array, object value and self-edge afterward.
This tests survival across collection, not a measured bound on garbage reclamation.
A valid GC module returning a reference is still rejected by the numeric host API.

Node 24.22.1 executes the complete fixture locally without flags. Local Node 20.10
accepts a basic struct type with its experimental flag but rejects this standard
recursive reference graph; it is not silently substituted or skipped. Existing
README/CI setup already selects Node 24. Runtime tests use the Node on PATH.

These are hand-built core-Wasm/runtime results, not source-compiled JS objects,
GC-backed closure/async frames, cross-component roots or performance evidence.
See the accompanying PR for exact checks on its final revision.

## Explicit boundary statement

This establishes the selected WasmGC target and executable runtime prerequisite.
It does not implement general JavaScript heap values. The next independently
observable slice is source object/array construction, property mutation, reference
passing and identity through the shared frontend/IR, compared in all three engines.

## First shared-IR representation increment (2026-10-31)

The object/property prerequisite extends the shared IR with `ManagedValue` and
`ManagedObject`, each lowered to one traced reference. An immutable carrier stores
a language-kind tag, an f64 payload and a nullable GC reference. Its current domain
is the four canonical primitive kinds plus ordinary objects. Known Number values
retain their direct f64 representation; the existing primitive wire pair is separate.
Object equality compares the wrapped object reference, preserving identity across
multiple carriers. Mutable GC property entries preserve earlier value snapshots.

This is a local, revisable representation decision, not acceptance of a complete
eight-kind union. A raw `eqref` alone would not retain primitive kind distinctions
or distinguish future by-value String/BigInt comparisons from identity comparisons.
An unboxed multi-lane union remains a possible optimization, but would spread kind
and reference-lane bookkeeping through calls and containers before the first source
slice. Boxing only at managed boundaries provides a simpler initial implementation
without changing known-number arithmetic. No performance claim selects it permanently.

Own data-property operations use lossless UTF-16 key arrays and linear GC-backed
entry storage. Shape/hash caches are deferred until executable source behavior is
established. Own lookup is not a substitute for prototype-aware JavaScript lookup;
source admission must provide the required semantics or reject the case. Arrays,
strings, symbols, functions and coercions require further implementation.

Ordinary internal helpers pass/return traced references. Exported signatures retain
the current profiles. GC captures, private closure signatures and async byte-spill
modules reject until their storage can trace references. The kernel is appended
after source functions to preserve internal call indices and emitted only when used.

Maintained typed-IR tests compare emitted modules with native JS in Node. Wasmtime
tests execute the shared helper kernel, including a live self-cycle across forced
copying collections. A separate manual check executes an actual emitted mixed-helper
artifact in both engines. Source object compilation remains the next increment.
See [the representation contract](../managed-values.md) for precise boundaries.

## Related work and follow-up

- [Shared backend layout](0014-backend-value-layout.md).
- Proposed [semantic/lifetime boundary](0011-semantic-values-and-lifetime-boundaries.md)
  and [activation ownership](0013-async-job-and-activation-ownership.md): retain their
  distinct status and reconcile the new reference representation at each seam.
- [WasmGC design](https://github.com/WebAssembly/gc/blob/main/proposals/gc/Overview.md),
  [pinned Wasmtime collectors](https://docs.rs/wasmtime/50.0.0/wasmtime/enum.Collector.html),
  and [ECMAScript value kinds](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-ecmascript-language-types).

## Summary

Use WasmGC for the general-value path, preserve specialized primitives, and grow
source support through executable slices. Keep current runtime evidence distinct
from the unimplemented object model, frame migration and public lifetime contracts.
