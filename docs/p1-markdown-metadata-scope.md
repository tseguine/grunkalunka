# Guest-only versus globally applied Rust crate metadata (P1)

The [same-host crate-metadata control](p1-markdown-crate-metadata-control.md)
found that passing `-C metadata` through `RUSTFLAGS` can substantially
change the Rust Markdown guest's diagnostic names, code and data sections.
The [three-native-host fixed-metadata experiment](p1-native-markdown-metadata.md)
then found that requesting one identical metadata value on Linux, Windows
and Apple Silicon **did not** make their independently built binaries
identical. Even under fixed flags, the default versus fixed body-overlap
effects differed on Windows and macOS.

An unresolved confounder is the **scope of the metadata change**:
`RUSTFLAGS` can affect the Markdown guest **and dependency crates**.
Cargo documents that `cargo rustc [options] -- [rustc args]` passes
extra arguments only to the **final compiler invocation** for the
selected package target, not its dependencies. This permits a narrower
controlled experiment. See the [official Cargo
reference](https://doc.rust-lang.org/cargo/commands/cargo-rustc.html).

## Three real compile modes on one native Linux runner

The [optional P1 workflow](../.github/workflows/p1-markdown-metadata-scope.yml)
builds the **existing unchanged** `grunkalunka-markdown --example outline_guest`
for `wasm32-unknown-unknown` with locked/offline Cargo, stable Rust,
the same checked-out source and development profile. It runs:

| Mode | Cargo command and metadata scope |
| --- | --- |
| Default | `cargo build -p grunkalunka-markdown --example outline_guest …` with `RUSTFLAGS=""` |
| Guest-only | `cargo rustc -p grunkalunka-markdown --example outline_guest … -- -C metadata=p1guestcontrol`, with `RUSTFLAGS=""` |
| Global | `cargo build -p grunkalunka-markdown --example outline_guest …` with `RUSTFLAGS="-C metadata=p1guestcontrol"` |

All modes share one fixed **private absolute `CARGO_TARGET_DIR`**
and `CARGO_INCREMENTAL=0`. Between modes, the helper runs
`cargo clean --target wasm32-unknown-unknown`, checks that the previous
guest was removed, and rebuilds. It independently verifies six tracked
source/manifest/lockfile identities after each compilation, and that the
native Rust toolchain remained unchanged. The helper deletes only its
private target directory when finished.

The [bounded collector](../tools/bench/markdown-metadata-scope.mjs)
validates each actual core-Wasm guest and uses the existing P1
section/defined-function-body and diagnostic-name SHA-256 analyzers.
It compares default/guest-only, default/global and guest-only/global
independently, reporting exact full-artifact identity, changed standard
and custom sections, raw shared function-name occurrences, shared
name-plus-identical-body occurrences, exact body matches by ordinal
and by multiset (retaining duplicates), and function counts.

The optional workflow retains only one compact JSON report in the
hosted log. It does **not** change any existing source file, Cargo
manifest, canonical CI, compiler implementation or Markdown/WIT
functionality. Report equality is not a performance or portability
pass/fail threshold; invalid input, missing/malformed executable
artifacts or failed compilation remains a test failure.

For local reproduction with the Rust wasm target and locked dependencies
installed:

```sh
node --test tools/bench/tests/markdown-metadata-scope.test.mjs
node tools/bench/markdown-metadata-scope.mjs > metadata-scope.json
```

## Interpretation and limits

If guest-only metadata produces relatively small changes but globally
applied metadata produces widespread body changes, that suggests
dependencies' metadata/linkage contributes materially to the earlier
same-host effect. It does **not** demonstrate that this was the cause
of differing native Windows/macOS outputs. If guest-only changes most
bodies, guest crate disambiguation or final linking may be enough to
cause large changes in this controlled environment.

`cargo rustc` is not a complete compiler invocation attestation:
the actual guest receives Cargo-generated flags and inputs **in addition**
to the explicitly supplied `-C metadata`. Cargo's own package hashing,
dependency identities, linker behavior and other host-specific inputs
remain unverified. The name section is optional, and raw function
names do not establish source function identity. Raw body differences
may arise from changed function-index immediates without observable
semantic changes. The experiment neither compares execution cost nor
isolates guest-attributable memory; the existing Markdown/WIT functional
checks remain separate evidence.
