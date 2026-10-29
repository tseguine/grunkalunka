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

After the initial and warmed-batch measurements, both engines also take
`samples` repeated fresh-instance observations **from the same already
compiled module**, retaining independent instantiation and first-call
timings (raw samples, min/median/max). Each repetition creates a new Wasmtime
Store and Instance or a new Node WebAssembly Instance; no source or Wasm
module recompilation occurs. Store creation and export lookup are outside the
timed instantiation/first-call operations. Each fresh instance must pass
the same eight independent numeric goldens, with golden checks outside the
instantiation timer. The existing 2..20 sample bound also bounds these
repetitions, with no extra CLI flags or runner jobs.

These are **post-warmup** observations, not cold application startup.
Wasmtime Store setup and Node module/instance runtime behavior differ, and
both may reuse caches; this is not a machine-independent engine speed ratio.

To account separately for setup excluded from the fresh-instantiation timer,
the same Wasmtime probe now also measures `samples` empty-host controls:
construction of a **new `Engine::default()`**, then construction of an empty
`Store::new(&engine, ())` using that engine. Each has its own raw,
independently bounded sample series and min/median/max under
`wasmtime.hostSetup.engineCreation` and `storeCreation`. The controls
run **after** the existing five Wasmtime whole-process memory checkpoints
and the separate fresh-instance golden checks, but before the Node subprocess.
No guest module is compiled or called in these empty-host samples. Engine
and Store lifetimes may share process caches; destruction is not timed.
Node/V8 does not expose an equivalent JavaScript Engine/Store constructor,
so there is **no Node-vs-Wasmtime host-setup ratio** or comparable standalone
process startup result.

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
all timed batches **before the fresh-instance repetitions**. Each snapshot
reports Node process.memoryUsage() fields as
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

## Rust/Wasmtime process memory observation (Linux)

On Linux, the Rust-side replay also records process-wide resident memory at
five phase boundaries: after the original JS source has been compiled and the
Wasmtime Engine created but before compiling the Wasm module, after module
compilation, after instance creation, after independent goldens and warmup, and
after the measured batches but **before the fresh-instance repetitions and
separate Node subprocess**.
The observations use `/proc/self/status`: `VmRSS` for current resident bytes
and `VmHWM` for the process-lifetime peak resident bytes (both converted from
kibibytes). Reports retain the raw five phase snapshots in
`wasmtime.processMemory`.

These are whole **Rust process** observations. In particular, the first sample
occurs *after* frontend/lowering compilation and engine initialization, so even
the high-water mark can reflect earlier work. Its peak is a Linux process-lifetime
counter, **not** the maximum memory consumed by one Wasmtime invocation or a
guest allocation. Taking samples allocates temporary strings; caches, OS
accounting, allocator behavior, concurrent threads and previous compilation are
uncontrolled. The five reported `VmHWM` readings must be nondecreasing, while
`VmRSS` may rise or fall.

On Windows and macOS, where this narrow implementation does not support
equivalent measurements, the `wasmtime.processMemory` envelope explicitly
reports `source: "unsupported-on-this-host"` and `snapshots: null`; zero is
**never** substituted for unavailable data. The cross-platform checks confirm
the explicit unsupported result rather than claiming cross-host RSS
comparability. The existing Node subprocess snapshots use different APIs,
lifecycle phases and process workloads; they are **not directly comparable**
to these Rust process observations.

No OS-monitoring package, syscall binding, Wasm memory tracker or benchmark
performance gate is added. Native tests validate Linux status parsing, units,
missing/duplicate/overflow values, complete phase sets and peak ordering.
Actual Linux CI executes the same Wasmtime replay and requires non-null
Rust-process samples; Windows and macOS require explicit absence.

## Provenance and limits

The output includes OS and architecture observed by Rust and by Node; Node
and V8 versions, CPU model when available, Node runtime flags, Wasm SHA-256,
emitted size, call counts, cases/checksum and raw samples. The Rust report
declares the pinned Wasmtime dependency and separately records compiler
elapsed time. Record exact Git commit, rustc version, compiler/build flags,
host CPU/virtualization, machine load and cache conditions alongside each
report. The Wasmtime version declaration must follow the workspace pin.

Neither subprocess creation nor Node's input transfer is timed as startup.
Wasm decoding/JIT cache and CPU scheduling are not controlled. Full application startup, Rust process launch, code-cache behavior, **cross-
platform and guest-attributable Wasmtime memory**, frontend-only cost,
nonnumeric values and representative application costs remain outside this
slice. The process
snapshots above do not create portable engine memory comparisons. Collect multiple reports under stable conditions
before examining relative changes; the tool does not rank engines.

To check the real two-engine replay with minimal work:

    cargo test -p grunkalunka-runtime --test core_engine_cost_probe --locked --offline

The test uses the same Rust binary logic and Node script, checks option bounds,
an independently checked result for the emitted binary in both engines,
sample counts, independent fresh-instance goldens, and nonnegative timing
values. The canonical workspace checks
discover it without modifying the shared runner. CI verifies correctness, not
performance. This complements the private [WIT document-cost probe](document-cost.md)
and the earlier [numeric report](numeric-benchmark.md); it does not complete
the broader P1 portability-and-costs milestone.
