# Native-host Rust wasm32 target library identity (P1)

The P1 Rust Markdown guest is built separately on Linux x64, Windows x64
and native Apple Silicon ARM64 with the same target,
`wasm32-unknown-unknown`, and the same observed Rust release.
Its output differs across hosts in both function bodies and data, although
each host reproduces its own bytes across clean builds. This experiment
checks one potential input difference **before compiling any guest source**:
the target standard-library and other Rust target archive files installed
by each host's native Rust distribution.

## Bounded experiment

The [optional workflow](../.github/workflows/p1-wasm32-sysroot-identity.yml)
installs the repository's Rust toolchain and
`rustup ... --target wasm32-unknown-unknown` on Linux x64, Windows x64 and
native Apple Silicon ARM64, using Node 24.21.0 for the hash collector.

The [collector](../tools/bench/rust-wasm32-sysroot.mjs) obtains the exact
target directory from
`rustc --print target-libdir --target wasm32-unknown-unknown`,
then enumerates regular target archive/metadata files with extensions
`.rlib`, `.rmeta`, `.a`, `.o` and `.wasm` to a maximum depth of four.
It records sorted **relative file paths**, sizes and SHA-256 of file bytes,
including Rust `core` and `std` archives. It bounds the number, size
and total amount of data read; it rejects unexpected symlinks or missing
required libraries. It also records the native host, observed Rust release,
Git commit hash of rustc itself and LLVM version. Only a bounded JSON receipt
is sent between jobs through ephemeral GitHub Actions outputs.

The receiving hosts compare their own installed target archive inventories
to the Linux producer, distinguishing identical files, differing identical-
name files, names missing from the receiver and names absent on the producer.
Mismatches are **diagnostic observations** and are reported rather than turned
into CI failures. A different Rust release/compiler commit/LLVM version,
invalid receipt, unsafe path or failed input read fails closed.

After installing the target, run locally:

```sh
node --test tools/bench/tests/rust-wasm32-sysroot.test.mjs
node tools/bench/rust-wasm32-sysroot.mjs collect > host-sysroot.json
node tools/bench/rust-wasm32-sysroot.mjs compare linux-sysroot.json host-sysroot.json
```

The workflow does **not** compile Markdown, execute a Wasm guest, upload
artifacts, change the canonical CI matrix, touch source compiler code, or
measure runtime speed. No published benchmark results or external
dependencies are added.

## Evidence boundaries

Equal target-library archive files would **rule out their literal file-byte
differences as the direct explanation**, but would not make native Rust
compiler executables, host-specific code-generation decisions, codegen
units, dependency build scripts, source checkout paths, linker executables,
or other build inputs equal. Differing archives would establish one
candidate contributing input, not prove that any difference was actually
responsible for the Markdown guest's emitted bytes.

These inventories are observed host-local installation bytes; they are not
signed toolchain attestations, semantic equivalence claims or reproducible
build guarantees. The bounded Markdown/WIT tests provide separate functional
correctness evidence. The multi-host application and memory/cost boundaries
of P1 remain open.
