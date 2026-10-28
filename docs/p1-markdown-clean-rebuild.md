# Same-host Markdown guest clean rebuild comparison (P1)

The [cross-host Markdown guest section experiment](p1-markdown-guest-sections.md)
observed different standard Wasm sections (including function, code and data)
as well as custom metadata when the same Rust Markdown guest was compiled on
Linux, Windows and Apple Silicon. The independent GCD compiler experiment
[#98](https://github.com/tseguine/grunkalunka/pull/98) did not show that
variation for its much smaller numeric fixture. The cross-host comparison alone
does not reveal whether the Markdown differences are stable for each host.

## One-host controlled experiment

The [optional CI workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-markdown-clean-rebuild.yml)
runs **independently on three standard hosted runners**: Linux x64, Windows
x64 and native Apple Silicon macOS ARM64. On each runner, the
[bounded helper](../tools/bench/markdown-clean-rebuild.mjs) performs:

1. Create one fresh private temporary Cargo target directory. Compile
   `grunkalunka-markdown --example outline_guest` for
   `wasm32-unknown-unknown` using locked/offline Cargo inputs and a fixed
   `CARGO_INCREMENTAL=0`; record the resulting file size and SHA-256.
2. Invoke **the identical Cargo build command** without changing files or
   directory, to observe unchanged-input reuse. Record the file again.
3. Call `cargo clean --target wasm32-unknown-unknown` in that *same*
   private target directory. Explicitly verify that the guest file is gone.
   Then rebuild with the original command, profile, toolchain and target
   directory, and record the artifact a third time.

Every compiled guest is checked for bounded file size and valid core Wasm.
The script checks that tracked source, manifest and lockfile snapshots
remain unchanged across all three phases, including recording both Git blob
hashes and checkout hashes. This makes host checkout normalization visible
without pretending it did not occur. The actual native Rust release/host,
Node version, Cargo version, `RUSTFLAGS`, target and profile are recorded.
Both clean builds use the **same absolute Cargo target directory** on that
runner, avoiding a differing build-output path as an immediate confounder.

The result distinguishes `unchangedInvocationEqual` from
`cleanBuildsEqual`, with three classifications: stable within-host,
changed after cleaning, and changed even without cleaning. These are
**observations, not performance metrics**; no duration, ratio or ranking is
reported or used as a gate. A clean-build hash mismatch does not cause the
diagnostic job to fail: it is the result being investigated. Missing
artifacts, invalid Wasm, inconsistent source inputs and failed actual builds
*do* fail.

Run locally after installing the Rust target and fetching locked dependencies:

```sh
node --test tools/bench/tests/markdown-clean-rebuild.test.mjs
node tools/bench/markdown-clean-rebuild.mjs > markdown-rebuild.json
```

The script creates, cleans and deletes **only its own temporary Cargo target
directory**; it does not delete an existing workspace `target/`, overwrite
a user's saved guest, upload artifacts, or alter repository sources. It
prints one JSON receipt on stdout. The hosted workflow prints its report
to GitHub Actions logs but saves no benchmark files.

## What it cannot establish

A passing identical-byte result on one runner shows that two clean rebuilds
under that observed environment produced the same guest bytes. It does not
prove deterministic Rust/Cargo output across different runner instances,
build machines, different Rust releases, different target directory *paths*
or all input programs. Cargo's downloaded registry sources, compiled host
tools, and installed target standard library are retained; the clean step
removes Wasm-target build products, not the entire host environment.

Conversely, a within-host mismatch would be evidence to investigate, not
automatically a compiler semantics defect. No raw Wasm binary is retained
or signed. The existing Markdown/WIT component and native-outline tests
remain the functional correctness authority; this experiment is about
**output identity**, not runtime conformance or comparable host costs.
