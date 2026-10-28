# P1 portability and cost evidence

P1 collects **bounded, correctness-checked observations**, not a completed
performance or cross-platform compatibility claim. Each experiment has its own
workload, timing boundary, error policy and reproducibility limits. The
[milestone table](milestones.md#compatibility-runtime-and-application-tracks)
remains the status authority. GitHub PRs referenced below are merged source and
verification receipts, not saved benchmark performance results.

## Implemented controls and replay

| Boundary or question | Evidence | What it establishes, and what it does not |
| --- | --- | --- |
| Source build API and basic numerical replay | [Numeric probe](numeric-benchmark.md), [saved-report comparator](benchmark-comparison.md) | One source invocation including process/file overhead, Node JS/Wasm execution with raw samples and provenance, and a guarded report comparator. No isolated frontend time or production score |
| Typed WIT string transport | [#44](https://github.com/tseguine/grunkalunka/pull/44), [document transport probe](document-cost.md) | Wasmtime WAT parse, component compilation/instantiation, native vs typed granted/denied calls and cleanup, with fixed work bounds. Native control and component do **not** perform identical low-level operations |
| One emitted core Wasm across engines | [#46](https://github.com/tseguine/grunkalunka/pull/46), [engine replay](core-engine-cost.md) | Compiler-emitted numeric GCD bytes in Node/V8 and Wasmtime with fixed independent goldens and raw phase/batch times. It does not isolate engine instruction speed or host-loop overhead |
| Application-shaped text | [#49](https://github.com/tseguine/grunkalunka/pull/49), [#53](https://github.com/tseguine/grunkalunka/pull/53), [real Markdown guest probe](markdown-component-cost.md) | Four bounded Markdown/WIT workloads, typed policy denials, native outline parity, cleanup and same-host repeatability of selected metadata. Timings are *not* asserted stable |
| Actual artifact identity and repeated receipts | [#58](https://github.com/tseguine/grunkalunka/pull/58), [#60](https://github.com/tseguine/grunkalunka/pull/60), [capture](markdown-capture.md), [paired comparator](markdown-comparison.md) | SHA-256 guest/runner/harness/source identities and strict sequential same-process matching. Does not independently attest build flags, cache state or comparable different hosts |
| Whole Node/V8 process memory | [#61](https://github.com/tseguine/grunkalunka/pull/61), [phase descriptions](core-engine-cost.md#nodev8-process-memory-checkpoints) | Five process-wide RSS/heap/external/ArrayBuffer observations around the actual Node core-Wasm replay. Not isolated Wasm allocations or measured process peaks |
| Linux Rust/Wasmtime process memory | [#64](https://github.com/tseguine/grunkalunka/pull/64), [phase descriptions](core-engine-cost.md#rustwasmtime-process-memory-observation-linux) | Five whole-Rust-process Linux VmRSS/VmHWM readings. Includes the compiler/engine process, with explicit **unavailable** output on Windows/macOS; not guest-only memory or portable memory comparison |
| Oxc parser and semantic controls | [#67](https://github.com/tseguine/grunkalunka/pull/67), [standalone Oxc probe](frontend-cost.md) | Fresh-allocator parse vs reused-AST Oxc semantic-building observations on checked JS/TS fixtures. Not the maintained compiler's lowering/IR/emission costs |
| Scalar typed WIT resource boundary | [#68](https://github.com/tseguine/grunkalunka/pull/68), [scalar WIT probe](scalar-wit-cost.md) | Real borrowed-resource result, host policy and arithmetic, with native control, independent goldens and call-count checks. Not a pure ABI or import crossing tax |
| Scalar core-Wasm host import | [#69](https://github.com/tseguine/grunkalunka/pull/69), [host-call calibration](host-call-calibration.md) | Native, guest-only and guest-to-host typed Wasmtime calls with fixed scalar wrapping outputs and host counters. Core-Wasm import is not WIT canonical ABI; do not subtract these series as if work were identical |
| Identical bytes on another host | [#76](https://github.com/tseguine/grunkalunka/pull/76), [portable GCD replay](portable-core-replay.md) | Produce a bounded manifest for one existing compiler-emitted Wasm artifact, then replay **the same SHA-256 bytes** through Node/V8 and optional Wasmtime CLI without source recompilation on the receiver. [#83](https://github.com/tseguine/grunkalunka/pull/83) additionally verifies the identical x64-produced bytes on native Linux ARM64 using Node/V8 and Wasmtime; this remains one numeric fixture |
| Native Linux ARM64 Markdown/WIT pipeline | [#84](https://github.com/tseguine/grunkalunka/pull/84), [ARM64 component smoke](p1-arm64-markdown.md) | Rebuilds the existing Markdown guest and runs the typed component's native parity, capability denial and cleanup checks on real Linux ARM64. Unlike #83, this does not demonstrate cross-architecture byte identity or application-wide compatibility |
| Repeated complete public compilation | [#77](https://github.com/tseguine/grunkalunka/pull/77), [public compiler API probe](source-compile-cost.md) | First and warmed in-process `grunkalunka_compiler::compile` batches for fixed affine JS/TS and GCD source, byte-identical WAT/Wasm, independently checked runtime goldens. No separate internal parse/semantic/lower/emit samples |

The [canonical CI workflow](ci.md) runs on Linux x64, Windows x64 and
native Apple Silicon macOS ARM64. The relevant code-bearing PRs above completed their exact-head
three-platform checks, which verify execution, correctness, bounds and report
structure. **CI does not retain the observed timing distributions, rank
machines or validate performance thresholds.** Some tools produce local
reports when explicitly run; the build system does not publish them.

## Open boundaries and proposed next measurements

- **Broader cross-architecture verification:** [#83](https://github.com/tseguine/grunkalunka/pull/83)
  has replayed one byte-identical x64-produced GCD artifact on native Linux
  ARM64 in Node/V8 and Wasmtime. [#84](https://github.com/tseguine/grunkalunka/pull/84)
  has independently built and tested the Markdown/WIT component on Linux ARM64.
  Neither establishes identical Markdown guest binaries across hosts, a general
  application distribution format or other OS/architecture combinations.
- **Comparable performance evidence:** capture retained raw samples under
  explicit CPU, toolchain, flags, workload, warmup, cache and build-artifact
  provenance. A passing CI timing observation is not a comparative score.
- **Compiler stage attribution:** the Oxc-only probe and repeated public
  compiler API have different boundaries. The maintained compiler does not
  yet expose separate parse, semantic, feature-admission, lower, IR verify,
  WAT emission and binary encode time distributions.
- **Portable memory measurements:** Node and Linux Rust memory snapshots
  describe different processes, APIs and lifecycle phases. Guest-attributable
  memory and a cross-platform Wasmtime host observation remain open.
- **Broader useful applications:** most experiments use narrow numeric or
  read-only document operations, not a complete app's build/startup,
  capability-mediated calls, end-user interactions or resource footprint.

Do not expand the assigned P1 scope into A1, browser implementation or
compiler language feature work by treating these gaps as authorization. The
maintainer explicitly selects high-level roadmap goals and priorities.
