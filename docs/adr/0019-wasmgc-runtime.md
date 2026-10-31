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
is the four canonical primitive kinds plus ordinary objects and Strings. Known Number values
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
symbols, functions and coercions require further implementation.

Ordinary internal helpers pass/return traced references. Exported signatures retain
the current profiles. GC captures, private closure signatures and async byte-spill
modules reject until their storage can trace references. The kernel is appended
after source functions to preserve internal call indices and emitted only when used.

Maintained typed-IR tests compare emitted modules with native JS in Node. Wasmtime
tests execute the shared helper kernel, including a live self-cycle across forced
copying collections. A separate manual check executes an actual emitted mixed-helper
artifact in both engines. This IR-only increment preceded the source admission below.
See [the representation contract](../managed-values.md) for precise boundaries.

## Source data-object increment

Synchronous literal/alias/property lowering uses the existing evaluator and these
managed IR operations. Managed truthiness retains the four primitive kinds and
truthy objects. Exports preserve the current Number/primitive interfaces.

For this first source increment, every object originates in a fresh literal, host
inputs are numeric, and prototype mutation is unsupported. Static keys matching
any Object.prototype intrinsic reject, including shadowed keys; remaining static
reads can faithfully use own-data lookup and return Undefined for missing keys.
Native intrinsic-name tests exercise this premise. Full prototype lookup remains
required future work. The alternative of implementing prototype/function intrinsics
before any source execution was deferred; treating missing inherited values as
Undefined was rejected as observably incorrect. This temporary guard does not
change the intended general JavaScript target.

Managed property values are usable for identity/equality/truthiness and managed
storage, not implicitly projected into the primitive export or numeric coercions.
Arrays, managed frames, unknown receivers and full coercions remain explicit
follow-ups. See the representation contract for
the source subset and maintained three-engine differential evidence.

## Source helper reference transport

Ordinary synchronous named helpers now choose reference parameters/results from
finite source-flow facts over resolved actual arguments, local initializers/writes
and owned returns. Object-only flow keeps the known-object reference; mixed flow
uses the immutable managed carrier. Primitive-only helpers keep their existing
planning and f64 specialization. Annotations are not evidence of runtime kinds.
The query uses heap worklists and monotone finite sets, including recursive call
cycles; it does not replace complete source validation or prove prototype safety.

The alternative of boxing every helper parameter would simplify signature choice
but needlessly replace existing numeric fast paths. A full JS type solver or
per-call monomorphization is not required to execute the present reference slice;
those may be revisited when this conservative whole-helper signature loses useful
specialization. Unknown receivers/coercions still reject instead of trusting a
narrowed observation. Shared IR/backend transport needs no new runtime API or host
operation. Maintained native-JS/Node-Wasm/Wasmtime tests exercise mutation, identity,
forwarding/recursion, mixed values, call order and explicit boundaries.

## Strings in the shared managed value domain

String literals now use the existing GC value carrier with a private String kind
and a packed UTF-16 array. Equality compares code units, preserving NUL and lone
surrogates without normalization; truthiness distinguishes empty from nonempty text.
The source flow query carries Strings through ordinary helper parameters/results.
Known Number paths remain specialized, and object identity still uses references.
The older linear-memory String/JSON wire representation is distinct; public exports
and composition require explicit conversion instead of tag/pointer reinterpretation.

Reusing the packed-key array/equality code avoids a second text encoding or host
String authority. Arrays are mutable during private construction only and are never
exposed to source mutation. Literals over V8's 10,000-element fixed-constructor
limit use default allocation plus initialization through a traced local; existing
long property keys use that path too. A separate String array type, passive-data
literal pooling and interning remain possible refinements. The present direct
construction is simple and executable; no performance claim selects it permanently.
The source-size and public fuel budgets remain unchanged. Heavy semantic tests use
a bounded core-engine budget and verify that the normal public budget still traps
cleanly. See the pinned [V8 limits](https://raw.githubusercontent.com/v8/v8/13.6.233.17/src/wasm/wasm-limits.h).

Methods, length/index reads, concatenation, coercions, public String references and
reference-aware closure/async frames remain explicit work. Maintained JS/TS tests
compare native JS, Node-Wasm and Wasmtime, and forced copying collections retain
String content through object-property edges.

## Sparse array backend operations

The next private reference kind is an array header with an unsigned 32-bit length
and an immutable reference to an ordinary-object property holder. This reuses the
verified sparse own-property kernel and its traced entries. Canonical array-index
keys grow length, and a validated immediate resize filters indexed entries on
shrink. Holes remain absent entries; explicit Undefined remains a stored value.
Array boxing uses the existing Object identity kind, not a new JavaScript value
kind. The header identity is distinct from the private property holder.

A dense element vector was rejected for this first implementation because a single
large index or length must not require a correspondingly large allocation. A shared
property-table refactor would remove the extra holder allocation but would also
change ordinary-object code in this unit; defer that optimization until useful.
Descriptor semantics, prototype lookup and dynamic length conversion are separate
operations. Generic writes to `length` are rejected rather than approximated.

The foundation is deliberately separate from source array admission. Native-JS and
Node execution of emitted IR check index boundaries, length, identity, snapshots and
helper transport; Wasmtime kernel tests retain sparse entries, cycles and String
edges through actual copying collections. Existing Number layouts and managed
capture/async/public-boundary exclusions remain in force. See the
[array contract](https://tc39.es/ecma262/multipage/ordinary-and-exotic-objects-behaviours.html#sec-array-exotic-objects).

## Source array increment

The source-array increment now uses this backend through the existing expression
cursor and finite helper-flow query. Known arrays retain their own reference kind;
mixed values box without losing identity. Length reads and validated literal writes
retain Number results. Array construction, element order and elisions use explicit
continuations rather than native recursion. Static String/u32-literal keys reuse
the shared UTF-16 kernel.

Own-data operations remain faithful under fresh-reference admission and explicit
exclusion of intrinsic prototype names, with Array-specific names guarded only
for array receivers. Full prototype/property coercion support remains future work.
Primitive element-result projection, public reference exports and GC-aware
capture/async frames are still required; they are not implied by source arrays.
Source capture validation now rejects managed captures before IR validation.

Maintained source tests compare native JS, Node-Wasm and Wasmtime, including
identity, receiver snapshots, helper transport, holes, index/length boundaries,
erased metadata and explicit guards. Small-stack tests exercise source and key
depth. The previous backend-only boundary described above is superseded by this
source increment; the backend's own-property scope and lifetime exclusions remain.

## Traced synchronous closure storage increment

A synchronous typed-IR module that captures managed values uses GC struct cells
and GC reference-array environment/call frames. All captured cells in that module
share this domain, including numeric cells and cells holding closure pairs. A
closure's private environment lane becomes a traced reference; its code lane
remains an i32 table index. Numeric/primitive locals and arithmetic retain their
specialized lanes. Modules without managed captures retain their prior encoding.
This is an implementation decision within the maintainer's WasmGC direction.

This module-wide choice prevents a mutable binding captured by two kinds of
closure from splitting into independent linear-memory and GC cells. Cells retain
unboxed i32/f64 fields plus a nullable traced edge. Environment entries refer to
shared cells; argument entries are fresh value cells. This simple representation
allocates an array and per-argument cells, leaving typed frame structs and allocation
optimization for measured follow-up. No throughput or reclamation-latency claim is
made. The engine traces cycles and environments after a defining call returns.

Alternatives considered:

- A rooted reference-table handle bridge preserves the numeric closure ABI, but
  adds manual root scope, release, trap/reentrancy and cycle-lifetime rules. It is
  deferred in favor of directly traced edges.
- Migrating every numeric closure and suspended async frame simultaneously would
  remove the dual backend sooner, but expands the change across independent
  semantics and obscures regressions. Retaining legacy modules limits that risk.
- Per-closure storage selection would preserve more old layouts inside a mixed
  module, but requires a separate shared-cell bridge. A module-wide cell domain is
  the simpler correct initial implementation.

The verifier now accepts typed synchronous managed captures while retaining capture
initialization, mutability and exact-type checks. Source capture guards, managed
closure parameter/result guards, public reference boundaries and async GC spill
rejection remain in force. Private function identity in the managed value carrier
and a shared application heap are not introduced.

Emitted typed-IR/native-JS/Node tests cover rebinding, shared mutation, nested and
returned closures, primitive/callback frames, lexical iteration cells and argument
limits. A shared-fragment Wasmtime fixture checks aliasing, cycles and an old
object/String snapshot across forced copying collection; it does not establish
source capture support. A separate manual check runs an actual emitted closure IR
artifact through the production Wasmtime numeric adapter; it is not the maintained
lifetime fixture. Exact revision/check counts belong in the implementation PR. Source capture admission
and reference-aware async suspension are the next distinct integration steps.

## Synchronous source capture admission

Source closures now admit captured object/array/String bindings and managed property
snapshots through the existing typed capture mapping and traced backend. Removing
the former unconditional source guard requires no new IR operation, dependency or
ABI. A load remains a snapshot, environment entries retain shared cells, and a
factory's returned closure keeps the binding live after the factory returns.

The existing supported selection, callback and factory contracts retain primitive
arguments/results and function-pair transport. This increment does not admit managed
closure arguments/results, general function identities in containers, public
references, prototype/coercion semantics or suspended managed frames. Unsupported
operations in unused bodies still reject; TS metadata supplies no runtime proof.
This supersedes the backend-only source-capture boundary in the preceding section.

Maintained source comparisons run native JS, Node-Wasm and Wasmtime, including
shared rebinding/mutation, saved values, per-iteration cells and factory lifetimes.
A test-only generated-WAT root/observer retains an actual constructed closure pair
past source return, then forces copying collections before calling its unchanged
generated body. The adapter changes test observability only; production artifacts
gain no reference export or shared host heap. Clearing roots is tested without
claiming a measured reclamation/latency bound.

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
