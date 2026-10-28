# Cargo build-script output identity across native hosts (P1)

The bounded P1 Rust Markdown guest has different binary bytes when independently
built on Linux x64, Windows x64 and Apple Silicon ARM64, despite correct
Markdown/WIT functional tests and repeated builds on each host. The
[actual rustc argument inventory](p1-native-cargo-rustc-metadata.md)
observed that Cargo passes **different generated `-C metadata` and
`-C extra-filename` values** for the host build script and three wasm32
crate/target groups, whereas `unicase` had matching arguments.

A follow-up asks a narrower question: **do the build scripts actually emit
different generated source/byte files, cfg values or environment values
across native hosts?** A different build-script *compiler fingerprint*
does not necessarily mean its outputs differ.

## Structured Cargo build-script evidence

The [optional three-host workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-native-cargo-build-script.yml)
runs on native Linux x64, Windows x64 and Apple Silicon ARM64.
The [bounded collector](../tools/bench/native-cargo-build-script.mjs)
builds the existing, unchanged
`grunkalunka-markdown --example outline_guest` for
`wasm32-unknown-unknown` with `cargo build --message-format=json`,
locked/offline Cargo dependencies, `CARGO_INCREMENTAL=0`, empty
`RUSTFLAGS` and one fresh private absolute Cargo target directory.

Cargo's documented JSON
[build-script-executed messages](https://doc.rust-lang.org/cargo/reference/external-tools.html#build-script-output)
provide the executed package identity, output directory, cfgs, linked
libraries/search paths, and `rustc-env` values. The collector resolves
package IDs against that host's `cargo metadata --format-version 1`
result, then reads only generated **regular files beneath the private
`OUT_DIR`**, rejecting symlinks and unsafe paths.

For each build-script package it records a bounded and sorted manifest of
**relative generated filenames**, byte lengths, and full SHA-256
digests. The cfg strings, linked-library/search strings, and environment
**values** are recorded only as hashes; environment **variable names**
are retained as identifiers. Neither raw files, executable artifacts,
full native build logs, raw environment values nor absolute OUT_DIR
paths are transmitted. A successful actual Markdown guest compilation
is required, and the output guest's SHA-256/length is recorded.

The Linux producer sends only a bounded **JSON receipt** via ephemeral
GitHub job output. The receiving hosts independently run the same Cargo
build, compare the generated relative file manifests, and separately
report whether cfg, env or link-input hashes match. Different outputs
are *observations*, not CI failures. If no files were generated, the
manifest is explicitly empty; that is not interpreted as an absent
build-script execution.

Local reproduction with the repository stable Rust Wasm target and
locked dependencies available:

```sh
node --test tools/bench/tests/native-cargo-build-script.test.mjs
node tools/bench/native-cargo-build-script.mjs collect > build-scripts.json
node tools/bench/native-cargo-build-script.mjs compare linux.json build-scripts.json
```

The script deletes its private target directory and does not alter Cargo
manifests or workspace source files. This workflow is separate from
canonical Linux/Windows/macOS CI and never publishes benchmark results.

## Limits

This captures **observable build-script outputs and generated file bytes**,
not all native compiler or linker inputs. The JSON build-script message may
represent a cached execution (Cargo documents this); the fresh private
target directory is used to limit that ambiguity. A matching generated
file does not imply that Cargo uses identical metadata fingerprints or
host toolchains. Differing env-value hashes may be caused by expected
host-specific paths or other nonsemantic values; they must not be
presented as proof of source behavior changing.

A different generated file would identify one candidate contributing
source input, not establish that it causes the cross-host executable
differences. A matching manifest would rule out literal generated-file
byte differences for those observed build scripts, while leaving
Cargo metadata disambiguation, linker behavior, native host cfgs and
dependencies open. No performance, memory attribution or JS/TS
compatibility claims follow from this diagnostic.
