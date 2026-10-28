# Rust crate metadata as a controlled Markdown Wasm reproducibility input (P1)

The Rust Markdown guest reproducibility experiments established that
native Linux, Windows and Apple Silicon independently emit different
`wasm32-unknown-unknown` binaries despite successful Markdown/WIT tests.
The emitted bytes are repeatable on each host, and the installed target
libraries match across all three. The number of differing raw function bodies
decreased when using one codegen unit, while most v0-like diagnostic function
names still differed across native hosts.

Previous [physical source-path testing](p1-markdown-checkout-path-control.md)
found that changing checkout directory alone did not change the guest, and
explicit source-path remapping changed only the `name` custom section.
The remaining source of the cross-host differences is unknown.

## One-host metadata perturbation

The Rust compiler's [codegen documentation](https://doc.rust-lang.org/rustc/codegen-options/index.html#metadata)
describes `-C metadata` as an input to symbol-mangling disambiguation.
The [optional Linux experiment](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-markdown-metadata-control.yml)
uses that documented control to test **how much symbol and executable output
changes when that specific input changes**.

In one native Linux x64 Git checkout at one exact revision, the
[collector](../tools/bench/markdown-crate-metadata-control.mjs) builds
the unchanged `grunkalunka-markdown --example outline_guest` for
`wasm32-unknown-unknown` three times:

| Variant | Exact requested `RUSTFLAGS` |
| --- | --- |
| Default | Empty string |
| Alpha | `-C metadata=p1controlalpha` |
| Beta | `-C metadata=p1controlbeta` |

All variants use locked/offline Cargo, `CARGO_INCREMENTAL=0`,
the unchanged development profile, one **fixed absolute private
`CARGO_TARGET_DIR`**, and the same installed native Rust compiler.
Between variants, the script cleans the Wasm target products, verifies
the previous guest was removed and then rebuilds. It checks six selected
Git source/manifest/lockfile identities after each compilation. At the
end it removes its private target directory without modifying the source
checkout or the repository's normal `target/`.

Each guest is independently required to be valid, bounded core Wasm.
The inspector records artifact SHA-256 and size, section SHA-256 digests,
defined-function raw body SHA-256 digests, exact compiler-emitted
diagnostic function-name digests and the count of named functions.
The report compares default with alpha, default with beta, and alpha
with beta, independently reporting exact raw name occurrence overlap,
same-name/exact-body overlap, raw bodies matching at their ordinal or
in an order-independent multiset, and which Wasm sections differ.
**No raw names, executable artifacts or benchmark timing reports are uploaded.**

After installing the repository Rust Wasm target and fetching dependencies:

```sh
node --test tools/bench/tests/markdown-crate-metadata-control.test.mjs
node tools/bench/markdown-crate-metadata-control.mjs > metadata-control.json
```

The optional workflow retains the condensed receipt only in its job log
and does not alter canonical CI. The unit tests reject missing variants,
false metadata flags, unknown source/toolchain/target combinations and
malformed function name/body receipts.

## Interpretation boundaries

If the metadata variants cause widespread diagnostic name changes,
that supports crate disambiguation as a mechanism that can **create
similar-looking byte differences on the same host**. If executable bodies
change as well, it indicates that changing metadata can affect more than
custom symbol names in this build pipeline. Neither outcome establishes
that the actual Linux/Windows/macOS builds used different crate metadata,
or that this was responsible for their observed differences.

The flag is passed to Cargo through `RUSTFLAGS`, affecting eligible
dependency crates as well as the Markdown guest. Build-script behavior,
linker choices, crate-specific flags and host toolchains are not all
independently attested. Similar counts do not prove matching causes.
Raw body hashes can change because of function-reference renumbering
without changing function behavior. No full v0 demangler, opcode
normalizer, semantic comparison, CPU benchmark or performance threshold
is introduced. The separate Markdown/WIT tests remain the correctness
baseline; no functional fault has been established.
