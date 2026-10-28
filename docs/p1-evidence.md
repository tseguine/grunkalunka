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
| Same-byte core-Wasm replay across hosts | [#76](https://github.com/tseguine/grunkalunka/pull/76), [#83](https://github.com/tseguine/grunkalunka/pull/83), [#90](https://github.com/tseguine/grunkalunka/pull/90), [#91](https://github.com/tseguine/grunkalunka/pull/91) | One x64-produced GCD artifact executes unchanged in native Linux ARM64 and Windows x64 Node/V8 and Wasmtime; a separate fixed affine artifact executes unchanged on Linux ARM64 with twelve IEEE-754 goldens. These are bounded numeric fixtures, not general application portability |
| Actual cross-host replay receipts | [#88](https://github.com/tseguine/grunkalunka/pull/88), [#97](https://github.com/tseguine/grunkalunka/pull/97), [receipt comparison](portable-replay-comparison.md) | Synthetic negative tests plus real Linux x64/ARM64 GCD producer/receiver receipts with matching artifact SHA-256, eight goldens and both native engines. Receipts are observations, not signed attestations or host performance rankings |
| Independent native compiler output identity | [#98](https://github.com/tseguine/grunkalunka/pull/98), [native build identity](p1-native-build-identity.md) | Three native CLI builds on Linux x64, Windows x64 and Apple Silicon macOS ARM64 independently compiled the same Git source into identical 614-byte GCD core Wasm. This does not prove reproducibility for other input programs, Rust-compiled Markdown guests or platform-wide builds |
| Native Linux ARM64 Markdown/WIT pipeline | [#84](https://github.com/tseguine/grunkalunka/pull/84), [ARM64 component smoke](p1-arm64-markdown.md) | Rebuilds the existing Markdown guest and runs the typed component's native parity, capability denial and cleanup checks on real Linux ARM64. Unlike #83, this does not demonstrate cross-architecture byte identity or application-wide compatibility |
| Rust Markdown guest output variation and controls | [#101](https://github.com/tseguine/grunkalunka/pull/101), [#106](https://github.com/tseguine/grunkalunka/pull/106), [#107](https://github.com/tseguine/grunkalunka/pull/107), [section analysis](p1-markdown-guest-sections.md), [same-host rebuilds](p1-markdown-clean-rebuild.md), [Windows LF control](p1-windows-markdown-eol.md) | Independently compiled Markdown guests differ across native Linux, Windows and Apple Silicon in standard function/element/code/data sections and custom names. Each host reproduced its own exact bytes after a clean rebuild. Windows checkout CRLF-to-LF normalization **did not change** its emitted guest bytes. None of these observations establishes a runtime bug or identifies the remaining host-dependent cause |
| Defined-function body fingerprint localization | [#110](https://github.com/tseguine/grunkalunka/pull/110), [#113](https://github.com/tseguine/grunkalunka/pull/113), [section/body analysis](p1-markdown-guest-sections.md) | Each native Rust Markdown guest has **1,897** defined-function bodies; raw bodies differ at the same ordinal for 1,313 Windows and 1,374 Apple Silicon bodies relative to Linux. An exact SHA-256 multiset comparison (ignoring function ordinal but retaining duplicate bodies) still leaves **952 Windows** and **984 Apple Silicon** body occurrences unmatched. Reordering alone cannot explain those literal byte differences; neither result proves semantic incompatibility |
| One-codegen-unit Rust Markdown control | [#115](https://github.com/tseguine/grunkalunka/pull/115), [CGU experiment](p1-markdown-cgu-control.md) | Native Linux, Windows and Apple Silicon independently rebuilt the unchanged Markdown guest with default dev settings and then `-C codegen-units=1` in isolated same-path target directories. Positional body differences versus Linux decreased from **1,313 to 476 on Windows** and **1,374 to 757 on Apple Silicon**, but emitted guest artifacts remained unequal. This is evidence that codegen-unit configuration affects these byte observations, not proof of the remaining cause or semantic differences |
| Installed wasm32 Rust target-library identity | [#116](https://github.com/tseguine/grunkalunka/pull/116), [sysroot fingerprint experiment](p1-rust-wasm32-sysroot.md) | All **54** installed `wasm32-unknown-unknown` target-library files (103,517,873 bytes total) matched byte-for-byte across Linux x64, Windows x64 and native Apple Silicon ARM64 under observed Rust 1.99.1 and LLVM 23.1.1. This eliminates literal differences in those target files as a direct cause of the Markdown guest artifact divergence, not native host compiler, linker or build-environment differences |
| Rust diagnostic function-name correlation | [#119](https://github.com/tseguine/grunkalunka/pull/119), [named-body experiment](p1-markdown-named-body-correlation.md) | All **1,897** Markdown guest functions had recorded diagnostic names on each host, but only **195 raw name occurrences** matched Linux on either Windows or Apple Silicon. Among those 195, **89** had matching raw bodies and **106** had changed body bytes. Compiler-emitted diagnostic names are not stable source-level identities, and these observations do not establish semantic incompatibility |
| Repeated complete public compilation | [#77](https://github.com/tseguine/grunkalunka/pull/77), [public compiler API probe](source-compile-cost.md) | First and warmed in-process `grunkalunka_compiler::compile` batches for fixed affine JS/TS and GCD source, byte-identical WAT/Wasm, independently checked runtime goldens. No separate internal parse/semantic/lower/emit samples |

The [canonical CI workflow](ci.md) runs on Linux x64, Windows x64 and
native Apple Silicon macOS ARM64. The relevant code-bearing PRs above completed their exact-head
three-platform checks, which verify execution, correctness, bounds and report
structure. **CI does not retain the observed timing distributions, rank
machines or validate performance thresholds.** Some tools produce local
reports when explicitly run; the build system does not publish them.

## Open boundaries and proposed next measurements

- **Broader cross-host application portability:** [#83](https://github.com/tseguine/grunkalunka/pull/83),
  [#90](https://github.com/tseguine/grunkalunka/pull/90) and [#91](https://github.com/tseguine/grunkalunka/pull/91)
  demonstrate same-byte GCD/affine core-Wasm on selected Linux ARM64 and
  Windows x64 receivers; [#84](https://github.com/tseguine/grunkalunka/pull/84)
  exercises a separately compiled Markdown/WIT guest on native Linux ARM64.
  These do not establish a distributed application format or broad JS/TS behavior.
- **Rust guest cross-host reproducibility:** in the exact-head
  [#98 canonical CI](https://github.com/tseguine/grunkalunka/actions/runs/40679064524),
  the Markdown/WIT guest passed on all three hosts but emitted different bytes
  despite reported Rust 1.99.1. [#101](https://github.com/tseguine/grunkalunka/pull/101)
  localized differences to standard function, element, code and data sections
  plus custom names. [#106](https://github.com/tseguine/grunkalunka/pull/106)
  found stable bytes across two clean rebuilds on each individual host.
  [#107](https://github.com/tseguine/grunkalunka/pull/107) showed that
  Windows checkout CRLF conversion **alone** does not cause its different
  output. [#110](https://github.com/tseguine/grunkalunka/pull/110)
  found positional raw-body differences in **1,313 of 1,897** defined functions
  on Windows and **1,374 of 1,897** on Apple Silicon relative to Linux.
  [#113](https://github.com/tseguine/grunkalunka/pull/113) then found
  **952 Windows** and **984 Apple Silicon** body occurrences without an exact
  raw body match anywhere in Linux, even ignoring order and retaining duplicates.
  Reordering alone cannot explain the literal differences. In
  [#115](https://github.com/tseguine/grunkalunka/pull/115), explicit
  `-C codegen-units=1` reduced the number of positional mismatches to
  **476 on Windows** and **757 on Apple Silicon**, without making cross-host
  Wasm bytes identical. [#116](https://github.com/tseguine/grunkalunka/pull/116)
  found all 54 installed wasm32 target-library files byte-identical across
  hosts. [#119](https://github.com/tseguine/grunkalunka/pull/119) showed that
  only 195 of 1,897 raw compiler-emitted function-name strings match Linux
  on either receiving host. Native compiler/linker behavior, name-mangling
  details, encoded index references, paths and other build inputs still
  require investigation. No functional fault has been established.
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
