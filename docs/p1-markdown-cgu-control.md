# Markdown guest single-codegen-unit control (P1)

The P1 Markdown guest experiments established different native-host Wasm bytes
despite successful functionality tests and repeatable clean builds on each host.
The [function-body experiment](p1-markdown-guest-sections.md) found 1,897
defined-function bodies per host, with many differing raw bytes even after
ignoring the function ordinal. This does **not** distinguish changes caused by
function/codegen-unit ordering from other Rust code-generation differences.

## Bounded paired configuration experiment

The [optional hosted workflow](../.github/workflows/p1-markdown-cgu-control.yml)
compiles the **existing, unchanged** Rust `outline_guest` for
`wasm32-unknown-unknown` with Node 24.21.0 and the repository's active stable
Rust compiler, independently on Linux x64, Windows x64 and native Apple
Silicon ARM64.

On **each host**, the [collector](../tools/bench/markdown-cgu-control.mjs)
uses one private temporary absolute `CARGO_TARGET_DIR`:

1. Build the guest from the checked-out repository using locked/offline Cargo
   and development profile with `CARGO_INCREMENTAL=0` and
   `RUSTFLAGS=""` (the existing default codegen-unit setting). Record
   bounded, validated core-Wasm section hashes, the complete ordered vector
   of raw function-body SHA-256 digests, output size and SHA-256, and selected
   tracked source/lockfile identities.
2. Run `cargo clean --target wasm32-unknown-unknown` in that same private
   target directory; verify the old guest file was removed. Rebuild the same
   sources with the same absolute directory but
   `RUSTFLAGS="-C codegen-units=1"`. Record the equivalent receipt.
3. Check that tracked source/manifest checkout bytes and canonical Git blobs
   remained unchanged between the configurations and that Rust's installed
   release/host did not change. Remove the entire private target directory
   when done. Nothing writes to the repository's usual `target/`.

The Linux job transfers only its bounded **JSON receipt** as an ephemeral
GitHub Actions job output (not the compiled guest or a benchmark artifact).
The Windows and macOS jobs independently build both configurations, compare
their default build with the Linux default build, then compare their
single-codegen-unit build with the Linux single-codegen-unit build using
the existing P1 section/body comparator. Every comparison retains separate
within-host and cross-host observations. Negative tests reject mismatched
source revisions, Rust versions, incomplete receipts, forged variant flags
and malformed function-body fingerprints.

Local reproduction after installing the Rust Wasm target and fetching locked
dependencies:

```sh
node --test tools/bench/tests/markdown-cgu-control.test.mjs
node tools/bench/markdown-cgu-control.mjs collect > host-cgu.json
node tools/bench/markdown-cgu-control.mjs compare linux-cgu.json host-cgu.json
```

## Interpretation

The question is whether **explicitly requesting one Rust codegen unit**
substantially changes the number of exact raw function-body matches between
independently built native hosts. If cross-host similarity improves, codegen
unit partitioning or order becomes a plausible contributing factor; it does
not prove a specific source of nondeterminism. If it does not improve,
codegen-unit count alone is not a sufficient explanation. Neither result
establishes semantic equivalence or a Rust/compiler defect.

The flag changes the compilation profile of affected Rust crates and
dependencies, not just the Markdown guest. Different native Rust toolchain
distributions, codegen backends, file paths, build scripts, host libraries
and environment variables remain potential confounders. The report records
the **requested** `RUSTFLAGS`, not an independently authenticated invocation
trace. Default vs single-CGU output from one host is also not compared as
equal semantic work by any performance metric. There are **no timing
thresholds, speed rankings or enforced cross-host binary-identity gates**;
mismatching artifacts are diagnostic output. Failed builds, malformed Wasm,
invalid receipts or changed sources still fail the workflow.

This is an additional P1 reproducibility experiment, not new JS/TS
compatibility, host abstraction, an A1 feature or application cost evidence.
