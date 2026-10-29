# Repeated public compiler API cost observations (P1)

The [numeric Node benchmark](numeric-benchmark.md) records **one** source-text
build API invocation including native process startup, file I/O and temporary
cleanup. The [standalone Oxc frontend probe](frontend-cost.md) separately times
Oxc allocator/parse and semantic construction on a reused AST, **not** the real
Grunkalunka numeric compiler. This bounded experiment fills part of the gap:
repeat **the actual public `grunkalunka_compiler::compile` API in one Rust process**,
without modifying or instrumenting the production compiler's internals.

After fetching the workspace's locked Cargo dependencies, run:

```sh
cargo run -p grunkalunka-runtime --example source_compile_cost --locked --offline -- \
  --workload affine-ts --samples 5 --iterations 5 --warmup 3
```

Supported fixed workloads are `affine-js`, `affine-ts` (the checked-in paired
source fixtures) and `gcd-js` (the existing Euclidean source fixture). Input
source text and language mode are fixed, not user-supplied; arbitrary full
language syntax is not admitted. Select unique option/value pairs: 2..12
samples, 1..32 compilations per timed sample, and 0..32 warmup compilations.
Defaults are 5, 5 and 3. These caps constrain the number of compiler calls,
not total native memory or process execution time.

## What is actually measured

The first compilation is recorded as a single independent startup observation.
After it finishes, the emitted WAT and Wasm are instantiated in Wasmtime once
and checked against four or eight **independently specified** expected numeric
results, including signed inputs and exact binary64 bits. A module importing
host capabilities fails rather than silently gaining authority.

Separate warmup compilations must emit **identical WAT and Wasm bytes** to the
preflight artifact. Each timed batch invokes the *full, public in-process
compiler API* repeatedly, retaining each emitted artifact in an owned vector.
After the timer stops, all artifacts must match the original WAT and Wasm bytes.
After that validation succeeds, a **separate per-batch release timer** measures
the explicit drop of the retained WAT/Wasm artifacts. The report retains its raw
`artifactRelease.samplesMs` observations and min/median/max separately from
the compilation batch times; it never adds one series to the other or silently
changes the existing measured compiler-API boundary. The preflight golden
artifact remains alive throughout both series.

This preserves full artifact identity as a correctness check without including
the byte comparisons inside the timed interval. The timer includes parsing,
semantic analysis, numeric admission and lowering, IR verification, WAT
emission, binary encoding and allocation/retention of the generated artifacts.
It excludes process startup, filesystem transport, Wasmtime compilation and
execution, and artifact byte comparisons. Retaining every artifact within a
batch affects allocation and memory cost; these totals are **not** directly
comparable to the standalone Oxc stages or subprocess-based build API.

The JSON result reports source, WAT and Wasm byte sizes **and SHA-256 digests
of the actual preflight source/WAT/Wasm bytes**, host OS/architecture,
workspace version, debug/release profile, and the **currently observed**
`rustc -vV` first-line release (or explicit null when unavailable), exact call
controls, first compile
elapsed milliseconds and every timed batch's raw milliseconds plus min/median/max.
Results retain source language and fixture identity. The separate artifact
release observation measures Rust destructor/deallocation work in this process,
not all downstream allocator or operating-system memory reclamation. It is not
part of the public `compile` API timing and is not directly comparable to
other compiler implementations. There are **no timing thresholds, speed
rankings, winner selection or implied speedup ratios**.
The normal Rust workspace test runs tiny 2-sample profiles for all three
fixtures, checks Wasmtime goldens and source-mode correctness, artifact
repeatability, independently bounded compilation/release samples and output
fields, then exercises invalid settings. The existing canonical suite runs this test during Linux/Windows
verification and native Apple Silicon's scheduled/manual full runs or
fallback. A healthy macOS baseline-reference PR gate does not imply
new PR-specific macOS test execution. No shared runner, compiler,
dependency or CI-platform changes are introduced.

## Whole-process source compilation memory checkpoints

The same real public-compiler probe now also records five **whole-Rust-process**
RSS and process-lifetime peak-RSS observations, outside all compiler-API,
artifact-release and Wasmtime calls being timed:

1. Before the first source compile, after selecting the bounded checked-in fixture.
2. After the first real compiler call, before Wasmtime golden verification.
3. After checking independent emitted-artifact goldens in Wasmtime.
4. After verified warmup compiles and disposing of their retained artifacts.
5. After all measured batches, byte verifications and explicit artifact drops.

On Linux, the existing P1 `/proc/self/status` reader reports current
`VmRSS` and high-water `VmHWM` converted from KiB to bytes. Its
well-formedness and monotonic-peak validation are shared with the core-Wasm
and Markdown component probes. The normal Rust integration tests require
actual Linux/Windows observations for each of `affine-js`, `affine-ts` and
`gcd-js`, preserving exact phase labels and numeric golden checks.
On Windows, the same five phases use
`Kernel32!K32GetProcessMemoryInfo` to record current and process-peak
**working-set bytes**, labeled `windows-psapi-working-set`. The existing
`rssBytes`/`peakRssBytes` field names remain for receipt compatibility,
but they do **not** mean Linux RSS on Windows. Neither OS's entire-process
readings isolate compiler allocations or make the platforms comparable.
macOS still explicitly reports `unsupported-on-this-host` and
`snapshots: null`, not zero memory.

These numbers include the **entire process** and allocations from Rust's
compiler, source/output buffers, Wasmtime verification, allocator caches
and this test harness. The fourth/fifth phases are taken after artifact
drops, but lower RSS is neither required nor assumed. The observations
do **not** measure only the compiler's heap, peak size of one compilation,
guest-attributable Wasm memory, process startup or performance across
platforms. Sampling itself may affect subsequent allocator/cache behavior.
No memory threshold, new OS library or extra GitHub Actions job is added.

## Boundaries and next evidence

There is no independent breakdown of **Grunkalunka's own** parser/semantic,
feature admission, lowering, IR verification or emission phases: this public API
exposes only their combined result, and no stage instrumentation is added here.
The selected programs are small numeric functions, not a production application
or general JS/TS throughput measurement. Warm caches, allocation, destructor
costs, scheduling and compiler optimizations are uncontrolled. This report alone
cannot compare machines, architectures or alternative implementations.

For reproducibility, retain exact Git revision, Rust and toolchain versions,
profile and build flags, CPU/platform information, raw samples and cache/load
conditions alongside the result. The hashes establish recorded byte identity for these inputs and outputs,
not that the bytes were produced by a particular compiler binary or from a
verifiable command line. The newly observed `rustc` version is obtained
**after all timing and memory snapshots** from the executable currently on
PATH; it is not authenticated as the compiler that built the probe or its
dependencies. It cannot recover the original compiler flags or target triplet. The report does not cryptographically attest
the compiler executable or capture all provenance. This experiment complements
[portable replay of the same artifact](portable-core-replay.md) without claiming
full P1 completion. It neither releases nor persists build artifacts or
changes any other stream's implementation.
