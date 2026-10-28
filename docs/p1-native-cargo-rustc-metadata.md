# Actual Cargo-generated Rust crate metadata across native hosts (P1)

The Rust Markdown guest is reproducibly different when built by native
Linux, Windows and Apple Silicon compilers. The
[same-host metadata experiment](p1-markdown-crate-metadata-control.md)
demonstrated that requesting different `-C metadata` values through
`RUSTFLAGS` can alter both raw names and executable code. The
[three-host fixed-request experiment](p1-native-markdown-metadata.md)
found that **the same requested metadata value** is not sufficient for
byte-identical independently compiled Wasm. The
[guest-only/global scope experiment](p1-markdown-metadata-scope.md)
also isolates how Cargo's dependency compilation participates.

What remains unknown is whether Cargo's **actual per-crate** compiler
arguments differ across hosts, even when the explicit `RUSTFLAGS`
text is the same.

## Capture actual native Cargo Rust compiler invocations

The [optional workflow](../.github/workflows/p1-native-cargo-metadata.yml)
runs independently on Linux x64, Windows x64 and native Apple Silicon
ARM64. Each host builds the unchanged
`grunkalunka-markdown --example outline_guest` for
`wasm32-unknown-unknown` twice in one private absolute Cargo target
directory, using locked/offline inputs and `CARGO_INCREMENTAL=0`:

| Mode | Explicit requested Rust flags |
| --- | --- |
| Default | Empty `RUSTFLAGS` |
| Fixed | `-C metadata=p1crosshostfixed` |

Between modes the helper cleans Wasm target products and checks that the
old guest was removed. It also verifies a fixed six-file canonical Git
source/lockfile snapshot after both compiles, and records the native Rust
release, commit, LLVM version, Node host and compiled guest SHA-256/size.

Unlike earlier P1 workflows, the [bounded collector](../tools/bench/native-cargo-metadata.mjs)
uses Cargo's **verbose output** (`cargo build -vv`) to observe the
single-line *actual rustc invocation records*. It extracts **only**:

- The parsed crate name, compilation target triple and crate-type.
- The ordered `-C metadata=...` values actually passed to that rustc
  process, including both Cargo-generated values and any explicit
  `RUSTFLAGS` contribution.
- The ordered `-C extra-filename=...` values observed for that crate.

For example, a future receipt could show a crate with both a Cargo
generated metadata value and the shared explicit
`p1crosshostfixed` argument. The collector compares those argument
lists by crate/target/type across hosts, using a multiset where a crate
has multiple compiler invocations, and reports exactly which groups
differ under the default and fixed modes.

The **raw verbose logs and full native rustc command lines are not
included in the JSON receipt**, nor are absolute source paths or the
compiled Wasm binary. The Linux producer transfers only its bounded
JSON inventory through ephemeral GitHub Actions job output. Each
receiving native host builds independently, rejects incompatible Rust
toolchains, revisions and canonical Git blobs, and reports actual
matching/differing per-crate flag groups together with guest output
identities. Different flags or Wasm bytes are diagnostic observations,
not pass thresholds.

To run locally after installing the Rust Wasm target and fetching
locked Cargo dependencies:

```sh
node --test tools/bench/tests/native-cargo-metadata.test.mjs
node tools/bench/native-cargo-metadata.mjs collect > native-cargo-metadata.json
node tools/bench/native-cargo-metadata.mjs compare linux-cargo-metadata.json native-cargo-metadata.json
```

## Interpretation and limitations

This provides **observed compiler argument metadata**, not a full
reproducible-build attestation. The parser relies on the format of
Cargo's current human-readable `-vv` output. That output is **not a
stable machine-readable API** and can change independently of compiler
behavior; missing required commands or unsupported formats fail
closed instead of silently reporting matching inputs.

Cargo can pass multiple `-C metadata` flags; the tool preserves their
order without assuming which ultimately determines symbol identity.
File hashes or extra filename values can reflect package feature
selection and other inputs, not just operating system. Matching parsed
arguments cannot establish identical native linker/compiler binaries,
source environments, outputs, or execution semantics.

Conversely, different actual per-crate metadata values across hosts
would identify a concrete previously uncontrolled **compiler-input
difference**. It would not by itself prove that this difference caused
the Markdown guest's raw Wasm divergence. The existing Markdown/WIT
correctness checks remain the functional baseline, and no timing
scores, memory measurements or performance ranking is introduced.
