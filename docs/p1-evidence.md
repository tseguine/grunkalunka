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
| Rust legacy diagnostic hash-suffix controls | [#121](https://github.com/tseguine/grunkalunka/pull/121), [#123](https://github.com/tseguine/grunkalunka/pull/123), [name-format analysis](p1-markdown-named-body-correlation.md) | Across Linux, Windows and Apple Silicon, none of the **1,897** Markdown guest function names matched either tested legacy terminal hash pattern (readable `::h` with 16 lowercase hexadecimal digits or mangled `17h` with 16 digits and `E`). Each host had **1,888 v0-mangled-like** and **9 other** names. Both controls left raw cross-host name overlap at **195**. These are narrow syntactic exclusions, not authenticated v0 parsing or evidence of equivalent code |
| Native Linux source-checkout path control | [#127](https://github.com/tseguine/grunkalunka/pull/127), [checkout/remap experiment](p1-markdown-checkout-path-control.md) | Two physically distinct clean Git checkouts of the same Markdown guest produced **identical 735,367-byte Wasm** on one Linux x64 host. With identical logical `--remap-path-prefix` destinations, the two independently built artifacts remained identical; within either checkout, remapping changed only the Wasm diagnostic `name` section and left **all 1,897 code bodies unchanged**. This rules out the physical checkout path *alone* as a sufficient cause on that host, not cross-OS source-path interactions or compiler differences |
| Rust crate-metadata perturbation on one Linux host | [#130](https://github.com/tseguine/grunkalunka/pull/130), [metadata control](p1-markdown-crate-metadata-control.md) | Rebuilding one unchanged Rust Markdown guest on native Linux with `-C metadata=p1controlalpha` or `p1controlbeta` changed the **default 1,897 defined functions to 1,896** and altered standard code/data/function/element sections and diagnostic names. Versus default, only **151** raw name occurrences, **25** name-plus-exact-body occurrences and **814/854** raw body occurrences ignoring order matched. This demonstrates same-host sensitivity to requested metadata, not that native Windows/macOS builds used different effective crate metadata |
| Fixed requested Rust metadata across native hosts | [#135](https://github.com/tseguine/grunkalunka/pull/135), [native metadata identity control](p1-native-markdown-metadata.md) | All three native hosts independently compiled the same Markdown guest both with default flags and with identical requested `RUSTFLAGS="-C metadata=p1crosshostfixed"`. **No pair of independent native guest artifacts became byte-identical.** Compared with Linux, exact raw bodies matching anywhere changed from **945 to 887 on Windows** and **913 to 955 on Apple Silicon**; exact shared diagnostic-name occurrences remained **195** on both. Common requested metadata is insufficient, but effective per-crate Cargo metadata was not yet authenticated |
| Standalone native Rust Wasm controls | [#137](https://github.com/tseguine/grunkalunka/pull/137), [standalone Rust witnesses](p1-rust-standalone-native.md) | Two independently compiled direct-`rustc` fixtures (`no_std` arithmetic with **5 functions** and `std` allocation with **98 functions**) passed seven independent numeric goldens each on Linux, Windows and Apple Silicon. Linux and Apple Silicon emitted byte-identical core Wasm for both; Windows matched **all function bodies at their original ordinals** but differed only in standard section 11 (data). This is limited to two controlled Rust sources, not the larger Cargo Markdown graph |
| Guest-only versus global metadata scope | [#138](https://github.com/tseguine/grunkalunka/pull/138), [Rust metadata scope control](p1-markdown-metadata-scope.md) | On one native Linux host, `cargo rustc … -- -C metadata=p1guestcontrol` for the final Markdown guest changed **only the diagnostic `name` section**, preserving all **1,897** raw bodies and standard Wasm sections; applying the **same requested value globally via `RUSTFLAGS`** changed executable sections, reduced the function count to **1,896**, and preserved only **789** order-independent raw bodies. This isolates the breadth of global Cargo/dependency compilation, not a specific offending dependency or cross-OS cause |
| Actual Cargo-generated per-crate metadata inputs | [#139](https://github.com/tseguine/grunkalunka/pull/139), [real Rust invocation inventory](p1-native-cargo-rustc-metadata.md) | Native Cargo `-vv` builds on Linux x64, Windows x64 and Apple Silicon ARM64 confirmed that **four of five observed crate/target groups** received different generated `-C metadata` and `-C extra-filename` arguments, including native `build_script_build` and wasm32 `pulldown_cmark`, `grunkalunka_markdown`, and `outline_guest`; only `unicase` matched. These generated values remained different even when the **same explicit** `-C metadata=p1crosshostfixed` argument was appended. This establishes a real differing compiler input, not which difference caused the final artifact bytes or runtime behavior |
| Standalone Rust initialized-data localization | [#141](https://github.com/tseguine/grunkalunka/pull/141), [native Wasm witness and data-chunk guide](p1-rust-standalone-native.md) | Independently built native Rust fixtures retained **5/5** and **98/98** identical raw function bodies on Windows compared with Linux, while data section 11 differed. The no-std fixture has **one differing 256-byte chunk containing 105 initialized bytes**; the std allocation fixture has **one differing chunk (index 10 of segment 0) among 16 chunks containing 4,010 initialized bytes**. Apple Silicon remained byte-identical to Linux. These are bounded chunk/section identities, not inferred data semantics |
| Cargo build-script generated-output identity | [#144](https://github.com/tseguine/grunkalunka/pull/144), [Cargo JSON build-script guide](p1-native-cargo-build-script.md) | The actual `pulldown-cmark@0.13.5` build script emitted **zero generated files/bytes, cfg values, rustc-env values or linker directives** on each of Linux x64, Windows x64 and Apple Silicon ARM64. Those observed outputs matched across hosts even though final guest binaries and Cargo-generated compiler metadata differed. This rules out differing emitted build-script output contents for that observed package, not differing crate metadata, native compiler behavior, or other inputs |
| Actual native Cargo metadata replacement control | [#147](https://github.com/tseguine/grunkalunka/pull/147), [native override experiment](p1-native-cargo-metadata-override.md) | Replacing four native-host-dependent **Cargo-generated** wasm32 `-C metadata` inputs with fixed crate-specific values increased raw body multiset overlap against Linux from **945 to 1,429** on Windows and **913 to 1,428** on Apple Silicon (out of 1,896 normalized functions). Function and element sections then matched Linux; code, data and diagnostic names still differed. Replacing the native build-script metadata did not change the resulting Wasm. This is a causal, selected-input control, not total build reproducibility or a production configuration |
| Two additional Wasm dependency metadata controls | [#152](https://github.com/tseguine/grunkalunka/pull/152), [remaining metadata experiment](p1-native-cargo-remaining-wasm-metadata.md) | `bitflags` and `memchr` **already had identical Cargo-generated metadata on all three hosts**. Replacing these two values after the first four changed code bytes within each native host but **did not** improve cross-host raw body overlap (still **1,429** Windows and **1,428** Apple Silicon) or make any complete guest identical. `-C extra-filename`, other native compiler/linker inputs and cross-host semantic behavior remain untested |
| Canonical P1 regression migration | [#157](https://github.com/tseguine/grunkalunka/pull/157), [#163](https://github.com/tseguine/grunkalunka/pull/163), [normal P1 test coverage](p1-ci-tests.md) | The prior **19 dedicated experimental PR workflows** were retired after the P1 unit/receipt suites and native GCD/affine portability checks became standard dependency-selected tests on each native CI host. The original optional cross-host artifact-transfer and metadata experiments are still reproducible manually, but **are not** ongoing automatic cross-host verification or an extra merge gate |
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
  on either receiving host. Subsequent [#121](https://github.com/tseguine/grunkalunka/pull/121)
  and [#123](https://github.com/tseguine/grunkalunka/pull/123) found **zero**
  matches for two specific legacy Rust hash-suffix spellings, while identifying
  **1,888 v0-mangled-like** symbols on each host.
  [#127](https://github.com/tseguine/grunkalunka/pull/127) separately found
  that different physical source checkouts on the *same Linux host*
  produced byte-identical guests; explicit Rust path remapping changed only
  diagnostic names, not code bodies. Native compiler/linker behavior, v0
  crate identity, encoded index references, cross-OS source paths and other
  build inputs still require investigation. In [#130](https://github.com/tseguine/grunkalunka/pull/130),
  changing only requested Rust crate metadata on a single Linux host changed
  diagnostic names **and** standard Wasm sections. That establishes metadata
  sensitivity, not that metadata was the actual cross-OS cause.
  [#135](https://github.com/tseguine/grunkalunka/pull/135) then showed that
  identical requested metadata flags across Linux, Windows and Apple Silicon
  **did not make guest binaries identical**; raw body overlap decreased on
  Windows and increased on Apple Silicon. Cargo's actual generated per-crate
  `-C metadata` and other host compiler inputs require independent verification.
  [#137](https://github.com/tseguine/grunkalunka/pull/137) established identical
  direct-rustc function bodies on all native hosts for two small Rust controls;
  Linux and Apple Silicon's entire artifacts matched, while Windows differed
  only in initialized data. [#138](https://github.com/tseguine/grunkalunka/pull/138)
  showed that applying requested metadata **only to the final Markdown guest**
  changed diagnostic names but not executable sections, whereas applying it
  **globally** changed much of the compiled Rust guest. These are bounded
  reproducibility controls, not evidence of semantic incompatibility.
  [#139](https://github.com/tseguine/grunkalunka/pull/139) then captured actual
  Cargo rustc invocations on all three native hosts: **four of five** observed
  crate/target groups received different generated `-C metadata` and
  `-C extra-filename` arguments despite the same explicit requested flag.
  Thus the cross-host compiler inputs really differ; subsequent #147 and #152
  experiments below isolate part, but not all, of their effect on emitted Wasm.
  [#141](https://github.com/tseguine/grunkalunka/pull/141) localized
  Windows-only differences in two smaller Rust fixtures to **one initialized
  data chunk per fixture**, with all raw function bodies identical; the
  actual [#144](https://github.com/tseguine/grunkalunka/pull/144) Cargo JSON
  experiment showed **identical empty build-script generated outputs**
  across all three hosts. Neither identifies the remaining metadata/linker
  cause or demonstrates a functional defect.
  [#147](https://github.com/tseguine/grunkalunka/pull/147) subsequently
  replaced the actual generated wasm32 metadata of four selected crates
  (rather than appending another flag). That increased exact raw body
  overlap with Linux from **945 to 1,429** on Windows and **913 to 1,428**
  on Apple Silicon, while equalizing the standard function/element sections.
  Neither the native build-script metadata substitution nor the subsequent
  [#152](https://github.com/tseguine/grunkalunka/pull/152) substitution of
  the already-cross-host-equal `bitflags` and `memchr` generated metadata
  resolved the remaining code/data/name differences. In particular, the
  four selected wasm32 metadata differences contributed causally under
  this experimental wrapper; they did **not** explain the entire artifact
  difference. Cargo `-C extra-filename` and other native/compiler/linker
  inputs remain live hypotheses. [#163](https://github.com/tseguine/grunkalunka/pull/163)
  retired the experimental Actions runners after corresponding canonical
  P1 tests were added; historical cross-host findings are not continuous
  CI guarantees.
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
