# Identical explicit Rust crate metadata across independent native hosts (P1)

The [same-host crate metadata experiment](p1-markdown-crate-metadata-control.md)
found that the existing Rust Markdown guest's Wasm and diagnostic function
names can change drastically when `-C metadata` changes: the default
1,897-function artifact became a 1,896-function artifact with either
`metadata=p1controlalpha` or `metadata=p1controlbeta`, with substantially
different raw symbols, code bodies and data sections. That proves **this
build setting can affect the output**, not that it explains the distinct
Linux, Windows and Apple Silicon binaries.

## Independent native-host controlled comparison

The [optional native-host workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-native-markdown-metadata.yml)
independently compiles the **unchanged Rust `outline_guest`** for
`wasm32-unknown-unknown` on native Linux x64, Windows x64 and Apple
Silicon macOS ARM64, with identical canonical Git blob identities
and each host's actual checkout-byte identities recorded separately.
The native Rust version, compiler commit and LLVM release are checked.

Each host uses the [bounded collector](../tools/bench/markdown-native-metadata.mjs)
to create one private absolute Cargo target directory and build the same
Markdown guest **twice** under the same locked/offline Cargo inputs,
`CARGO_INCREMENTAL=0`, and dev profile:

| Mode | Exact requested Rust flags |
| --- | --- |
| Baseline | `RUSTFLAGS=""` |
| Fixed metadata | `RUSTFLAGS="-C metadata=p1crosshostfixed"` |

Before the second build, `cargo clean --target wasm32-unknown-unknown`
must remove the prior compiled guest. Each real guest must be valid,
bounded core Wasm, and the six tracked source/manifest/lockfile identities
must remain unchanged. The helper records the installed native Rust
release, compiler commit and LLVM version, Node host and architecture,
guest byte count and SHA-256, raw section hashes, and complete raw
function-body and function-name digest vectors.

The Linux producer sends an ephemeral **bounded gzip-compressed JSON
receipt** to the native Windows/macOS receivers; no Wasm guest binary
or original function-name text is transferred. The receiver independently
compiles both configurations and compares:

- Baseline Linux versus baseline receiver, to reproduce the original
  cross-host discrepancy at the exact tested revision.
- Fixed-metadata Linux versus fixed-metadata receiver, to test whether an
  identical explicitly requested metadata value changes cross-host
  agreement.
- Each host's baseline versus its own fixed-metadata build, to measure
  how strongly the same input perturbation changes bytes locally.

The comparator reports exact artifact equality, changed standard/custom
sections, shared raw diagnostic-name occurrences, shared name-plus-exact
body occurrences, and raw function bodies matching at their ordinal
and in an order-independent multiset. It records function counts and
explicitly leaves positional comparisons unavailable when counts differ.
These outcomes are not equality gates or benchmark rankings.

The large receipt is compressed only for transfer. Its expanded size,
compressed size, canonical base64 representation and fixed toolchain/source
conditions are checked before trusting it. Optional heuristically
normalized name vectors from earlier P1 experiments are **omitted** from
this transfer: this experiment measures only original raw hashes. The
existing focused regression tests reject missing variants, forged metadata
flags, mismatched Git revisions/toolchains, malformed raw digest vectors,
inconsistent optional fields and incorrect native host roles.

After installing the target and fetching locked Cargo dependencies, run
from the repository root:

```sh
node --test tools/bench/tests/markdown-native-metadata.test.mjs
node tools/bench/markdown-native-metadata.mjs collect > native-metadata.json
node tools/bench/markdown-native-metadata.mjs compare linux-metadata.json native-metadata.json
```

The helper deletes only its private Cargo target directory; it does not
modify the working checkout, canonical CI, compiler code, application
sources or dependencies. The independent workflow's output is retained
in hosted logs but not published as a persistent artifact.

## Interpretation and limitations

If a fixed explicit `-C metadata` value increases raw cross-host byte
agreement, that is evidence that crate-metadata configuration affects
the observed reproducibility, but **does not prove that this single flag
makes effective crate identities identical across host builds**. Cargo
may pass its own crate-specific disambiguation inputs, and this flag
can affect dependency crates as well as the Markdown guest.

If output differences persist, identical *requested* metadata flags are
insufficient under these conditions. Native host compiler distributions,
linkers, build scripts, encoded function-index references, build-environment
state, source checkout normalization, symbol generation and build paths
remain possible factors. Full Rust compiler invocations and Cargo
dependency fingerprint graphs are not independently attested by this
experiment.

The correctness baseline remains the separate native Markdown/WIT
component tests. Raw Wasm digest differences are not proof of observable
semantic differences, guest-attributed memory costs, or host performance
comparability. P1's wider portability and performance boundaries remain open.
