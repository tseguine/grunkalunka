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
workspace version, debug/release profile, exact call controls, first compile
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
fields, then exercises invalid settings. Existing
Linux/Windows/macOS CI runs this test; the PR does not change the shared runner,
compiler, dependencies, or CI platform matrix.

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
verifiable command line. The report does not cryptographically attest
the compiler executable or capture all provenance. This experiment complements
[portable replay of the same artifact](portable-core-replay.md) without claiming
full P1 completion. It neither releases nor persists build artifacts or
changes any other stream's implementation.
