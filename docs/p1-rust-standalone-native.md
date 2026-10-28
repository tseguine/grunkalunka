# Standalone Rust-to-Wasm native host controls (P1)

The bounded Rust Markdown/WIT guest showed repeatable but different
`wasm32-unknown-unknown` output bytes when separately compiled on Linux x64,
Windows x64 and native Apple Silicon ARM64. Earlier P1 experiments isolated
source-checkout paths, codegen-unit count, installed target libraries, Rust
metadata and diagnostic names. [The fixed-metadata cross-host
experiment](p1-native-markdown-metadata.md) does not claim that all native
compiler inputs are identical.

This separate experiment checks whether **much smaller direct-`rustc`
workloads** show comparable cross-host output differences. It deliberately
does not build the actual Rust Markdown guest, exercise WIT capabilities,
change the Grunkalunka JavaScript compiler, or measure performance.

## Two independent compiled fixtures

The [optional hosted workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-native-rust-standalone.yml)
builds these exact checked-in Rust sources on independent native Rust
installations on Linux x64, Windows x64 and Apple Silicon ARM64:

| Fixture | Source | Code path |
| --- | --- | --- |
| `core` | [no_std.rs](../tests/build/fixtures/p1-rust-standalone/no_std.rs) | `#![no_std]` with a panic handler, 32-bit Euclidean GCD, and wrapping multiply/add; uses `core` but no Rust `std` |
| `std` | [std_alloc.rs](../tests/build/fixtures/p1-rust-standalone/std_alloc.rs) | Euclidean GCD plus a real four-element `Vec<i32>` allocation and wrapping sum; uses the precompiled target `std` and allocator |

Each independent native compiler receives the original **canonical Git
blob bytes**, even when the host's regular checkout would normalize line
endings. The Node runner writes that source into a temporary private
directory, calls native `rustc` directly (not Cargo) and deletes the
temporary build products after collecting the report.

The requested Rust options are identical on each machine: Edition 2024,
`--crate-type=cdylib`, `--target wasm32-unknown-unknown`,
`-C panic=abort`, `-C opt-level=0`, `-C debuginfo=0`,
`-C codegen-units=1`, a fixed `-C metadata=p1standalonecontrol`,
and an identical **logical** source prefix obtained through
`--remap-path-prefix`. Compiler/LLVM version and native host are
recorded. This controls selected inputs, not the native compiler/linker
binaries themselves.

Every resulting Wasm file must pass `WebAssembly.validate` and
instantiate under Node/V8 with **zero host imports**. Native exports
must return the independent numeric goldens: four GCD cases, plus
three extra cases for the wrapping arithmetic or four-element sum.
These correctness checks are enforced regardless of the artifact hashes.

The [bounded collector and comparator](../tools/bench/native-rust-standalone.mjs)
record the exact Git-source SHA-256 and size, compiled Wasm SHA-256 and
size, code-section function-body SHA-256 vector and complete raw Wasm
section fingerprints. The Linux producer transfers **only a bounded
JSON receipt** through ephemeral Actions job outputs. Windows and macOS
independently compile each fixture and compare exact cryptographic
identities, changed standard/custom sections and code-body overlap
both by ordinal and ignoring order (retaining duplicate occurrences).
A difference in bytes is an **observation**, not a failed portability
gate; failed source validation, compilation or independently checked
goldens *does* fail.

## Reproduction

After installing the repository's stable Rust toolchain with the
`wasm32-unknown-unknown` target:

```sh
node --test tools/bench/tests/native-rust-standalone.test.mjs
node tools/bench/native-rust-standalone.mjs collect > host-witness.json
node tools/bench/native-rust-standalone.mjs compare linux-witness.json host-witness.json
```

No workspace Cargo build, deployment, WASM component packaging, uploaded
binary or benchmark report is created. The workflow is independent of
canonical three-host CI, and focused tests reject forged source identities,
incorrect flag inventories, different Rust versions, malformed fingerprints,
broken golden reports and unsupported native host roles.

## Interpretation limits

Byte-identical output from one fixture would show that the observed native
Rust versions can emit **the same core Wasm for that bounded source** under
those flags. It would not establish deterministic Rust compilation for the
larger Markdown guest, production applications, other dependency graphs or
other toolchain releases. A differing `std` fixture but identical
`no_std` fixture could indicate a contribution from standard-library
usage or greater linking complexity, but would not isolate the sole cause.
A differing `no_std` fixture would justify investigating native compiler
or linker behavior more directly.

Independent numeric goldens demonstrate only the tested function calls,
not full Rust semantic equivalence. Hashes do not measure runtime cost,
cold-start time or guest-attributable memory. The earlier full
Markdown/WIT correctness baseline remains separate.

## Initialized-data chunk localization

The exact-head [#137 three-host experiment](https://github.com/tseguine/grunkalunka/pull/137)
established that both direct-rustc fixtures were **byte-identical on
Linux x64 and Apple Silicon ARM64**. On Windows x64, all **5/5 core**
and **98/98 standard-library** function bodies matched Linux *at
their original ordinal*; both complete Wasm binaries had the same
size as their Linux counterparts but differed **only in standard
section 11 (data)**. All independent numeric goldens passed on
each platform.

The [bounded data inspector](../tools/bench/wasm-data-chunks.mjs)
extends the **same** independent native-host workflow to localize
these differences without publishing the initialized-data bytes or
the guest binaries. It parses core-Wasm section 11 data segments
for the currently observed active/passive segment forms and simple
\`i32.const\` or \`global.get\` offset expressions, then records:

- Complete SHA-256 of the raw data section and each segment's
  initialized bytes, together with the segment mode, memory index
  and SHA-256 of any offset expression.
- Full SHA-256 digests of successive **256-byte initialized-data
  chunks**, bounded to at most 4,096 chunks across all segments.
- Cross-host counts of exactly matching chunks and the first 24
  differing **chunk indices** for corresponding, equal-size segments,
  as well as segment-layout and count differences.

The comparison does not reveal raw data, reconstruct a binary or
infer whether differing chunks contain literals, pointers, hashes,
linker-generated values or equivalent encodings. A changed data
section may be semantically harmless (and the same numeric goldens
can still pass), but the resulting chunk offsets can narrow a
follow-up on actual linker or compiler inputs.

The parser rejects missing/truncated sections, unsupported
expressions/segment flags, oversized fingerprints, noncanonical
digest vectors and malformed segment boundaries rather than
quietly claiming complete coverage. It is not a complete Wasm
interpreter or proof of memory-state equivalence.

To run the additional local parser tests:

\`\`\`sh
node --test tools/bench/tests/wasm-data-chunks.test.mjs
\`\`\`

Actual native-host compilation and receipt comparison remain in
the existing #137 workflow, alongside unchanged functional goldens.
Differences are observations rather than portability or timing gates.
