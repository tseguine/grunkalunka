# Experimental core-Wasm two-engine cost replay (P1)

From the repository root, after fetching locked Cargo dependencies and building
the workspace, run:

    cargo run -p grunkalunka-runtime --example core_engine_cost --locked --offline -- --samples 9 --iterations 1000 --warmup 100

This compiles the checked-in numeric GCD example once through the maintained
compiler API. The **same emitted core Wasm bytes** execute in Wasmtime and in
a separate Node/V8 subprocess; Rust pipes those bytes to the subprocess via
stdin. Neither engine recompiles source. The Node script neither fetches nor
builds the artifact. No imports or ambient capabilities are granted, and no
benchmark artifacts are uploaded, persisted or published.

The optional unique arguments bound samples to 2..20, calls per sample to
1..10,000, and warmup calls per engine to 0..10,000. Defaults are 9, 1,000
and 100. The byte transfer is bounded to 1 MiB. These are input/work bounds,
not a guarantee on process memory or wall-clock time. The probe is intended
for trusted local development, never production traffic.

## Measurement boundaries

The single report separates source-to-Wasm compilation from Wasmtime module
compilation, instance creation, first observed Wasm call and warmed call
batches. Node reports its own module compilation, instantiation, first call,
and warmed batches. Timings use each runtime's monotonic timer and retain raw
samples plus min/median/max in milliseconds per batch. Each engine processes
the identical eight fixed numeric input pairs in the same cyclic order.

Goldens for all eight pairs and warmup checksums are verified outside timed
batches. Batch checksums are verified immediately after each timed batch,
without accepting a silently wrong result. The GCD result must match the
fixed independently specified answer in both engines; there is no timing
pass/fail threshold. The subprocess exits nonzero on any mismatch. This
checks one closed numeric fragment, not general JavaScript conformance.

Measurements are sequential: the Wasmtime path precedes the Node subprocess,
and each has different startup, call/host-loop and JIT characteristics.
Node module creation is not equivalent to Wasmtime native compilation. Calls
include host-loop work and typed invocation overhead; neither path isolates
guest instructions or host-call overhead, and there is **no WIT boundary**
in this experiment. Do not interpret the results as portable engine speed
ratios, production throughput or a comparative benchmark score.

## Node/V8 process memory checkpoints

The Node subprocess now records five **instantaneous process-memory snapshots**:
after receiving the Wasm bytes but before module construction, immediately after
module compilation, after instantiation, after fixed goldens and warmup, and after
all timed batches. Each snapshot reports Node process.memoryUsage() fields as
bytes: resident set size (RSS), V8 heap total and used, external memory, and
ArrayBuffer allocations. Snapshots are collected **outside** timed intervals
and validated for well-formed byte counts without any memory threshold.

These values describe the **entire Node subprocess**, including Node/V8 itself,
JavaScript harness allocations, input buffers and Wasm runtime state. ArrayBuffers
may already be included in external bytes; these fields must not be added. A
change between snapshots cannot be attributed solely to Wasm, and RSS is not a
controlled peak or isolated artifact footprint. The garbage collector is not
forced, caches remain uncontrolled, and memory can decrease between samples.
Sampling itself may affect the subsequent phases.

This is **Node/V8-only memory evidence**, not Wasmtime host memory, a cross-engine
memory comparison or a cross-host ranking. The existing Rust core-engine replay
test executes the actual Node subprocess and checks that these fields are present.
The Node benchmark tests exercise well-formed observations and rejection of
missing, negative, nonintegral or inconsistent byte values.

## Provenance and limits

The output includes OS and architecture observed by Rust and by Node; Node
and V8 versions, CPU model when available, Node runtime flags, Wasm SHA-256,
emitted size, call counts, cases/checksum and raw samples. The Rust report
declares the pinned Wasmtime dependency and separately records compiler
elapsed time. Record exact Git commit, rustc version, compiler/build flags,
host CPU/virtualization, machine load and cache conditions alongside each
report. The Wasmtime version declaration must follow the workspace pin.

Neither subprocess creation nor Node's input transfer is timed as startup.
Wasm decoding/JIT cache and CPU scheduling are not controlled. Wasmtime
engine creation, Rust process startup, code cache, **Wasmtime host memory
footprint**, frontend-only cost, nonnumeric values and representative application
costs remain outside this slice. The Node-only snapshots above are not portable
memory comparisons. Collect multiple reports under stable conditions
before examining relative changes; the tool does not rank engines.

To check the real two-engine replay with minimal work:

    cargo test -p grunkalunka-runtime --test core_engine_cost_probe --locked --offline

The test uses the same Rust binary logic and Node script, checks option bounds,
an independently checked result for the emitted binary in both engines,
sample counts and nonnegative timing values. The canonical workspace checks
discover it without modifying the shared runner. CI verifies correctness, not
performance. This complements the private [WIT document-cost probe](document-cost.md)
and the earlier [numeric report](numeric-benchmark.md); it does not complete
the broader P1 portability-and-costs milestone.
