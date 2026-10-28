# Native Rust Cargo-generated metadata substitution (P1)

The [actual native Cargo rustc argument experiment](p1-native-cargo-rustc-metadata.md)
established that Linux x64, Windows x64 and native Apple Silicon ARM64
receive **different generated \`-C metadata\` values** for four of the five
Rust Markdown guest crate/target groups, even when every host appends the
same explicitly requested extra metadata argument. The earlier
[global versus guest-only metadata experiment](p1-markdown-metadata-scope.md)
also established that globally changing metadata affects many standard Wasm
sections, whereas applying metadata only to the final guest did not.

This experiment goes one step further: **replace Cargo's generated
per-crate metadata value** under an instrumented build, then observe how
much cross-host Rust Markdown guest Wasm identity actually changes.

## Experimental compiler wrapper — not production configuration

The [temporary native rustc wrapper](../tools/bench/fixtures/p1-native-cargo-metadata-wrapper.rs)
uses Cargo's documented
[\`RUSTC_WRAPPER\` interface](https://doc.rust-lang.org/cargo/reference/config.html#buildrustc-wrapper).
Cargo invokes the wrapper with the path to its *actual* native rustc
executable as the first argument. The wrapper forwards that same compiler
and all of its original arguments, changing **only one** actual
Cargo-generated \`-C metadata=...\` codegen argument per expected crate.

The wrapper accepts precisely five observed crate/target groups and
requires one sixteen-lowercase-hex-generated metadata value on each:

| Compiled crate | Target | Replacement value |
| --- | --- | --- |
| \`build_script_build\` | Native host | \`p1script\` |
| \`unicase\` | \`wasm32-unknown-unknown\` | \`p1unicase\` |
| \`pulldown_cmark\` | \`wasm32-unknown-unknown\` | \`p1pulldown\` |
| \`grunkalunka_markdown\` | \`wasm32-unknown-unknown\` | \`p1markdown\` |
| \`outline_guest\` | \`wasm32-unknown-unknown\` | \`p1guest\` |

A missing/extra generated metadata flag or an unknown compiled crate
fails closed. Cargo's \`--extern\`, \`-C extra-filename\`, target,
feature configuration, source, linker arguments, and every other
compiler flag are passed **unchanged**. The wrapper preserves real
rustc exit codes, and leaves compiler version/probe invocations alone.

The [independent three-host workflow](../.github/workflows/p1-native-cargo-metadata-override.yml)
compiles this wrapper locally with the installed host rustc, runs its
native Rust unit tests, and then builds the *unchanged* existing
\`grunkalunka-markdown --example outline_guest\` three times on each
host:

| Mode | Wrapper's action |
| --- | --- |
| Pass-through | Forward all compiler arguments unchanged, but log original generated metadata |
| Target-only | Replace generated metadata for the four wasm32-target crates; keep native build-script metadata |
| All | Also replace the native \`build_script_build\` generated metadata |

All three modes use exactly **the same wrapper executable path and
the same absolute private \`CARGO_TARGET_DIR\`** on their runner. The
collector runs \`cargo clean\` (including host *and* wasm target products)
between modes and checks that the previous compiled guest was removed.
The builds use locked/offline dependencies, the same Git checkout and
Rust toolchain, empty \`RUSTFLAGS\`, \`CARGO_INCREMENTAL=0\` and
**serialized \`-j 1\` compilation** to keep the short wrapper records
unambiguous. Neither the project build configuration nor the
developer's normal \`target/\` is changed.

A compact temporary wrapper record gives only crate name, target,
actual original generated metadata and effective forwarded metadata
value. All five expected invocations are required in each mode, and
the original generated values must remain **identical across the three
modes within each host**. Otherwise the experiment fails rather than
mistaking changed Cargo input generation for the effect of replacing it.

The collector verifies seven tracked Git source/toolchain/wrapper
inputs before and after the builds. It checks every actual compiled
guest is bounded valid core Wasm, then fingerprints its original
standard/custom sections and complete defined-function body SHA-256
vector using the existing P1 Wasm inspector. Linux transfers only a
bounded, compressed **JSON report**, never the actual executable
binary or raw compiler/source paths, to the two independently
compiling native receivers.

## Cross-host comparisons

The report independently compares Linux against Windows and Apple
Silicon for **each** of the three modes, including:

- How many generated metadata values differ *before* the wrapper,
  and how many of the actual forwarded metadata values still differ.
- Whether the complete independently compiled guest artifact bytes
  agree, which standard/custom sections differ, and how many raw
  function bodies match at their ordinal or ignoring ordering.
- The pass-through/target-only and target-only/all differences
  *within* each host, preserving the control's separate effect.

A match or mismatch is an observation, **not** a CI gate or
performance score. Actual compiler/build failures, unsupported
crate sets, malformed native records, changed Git inputs,
unexpected compiler versions and invalid Wasm do fail.

After installing the Rust wasm target and fetching Cargo dependencies:

\`\`\`sh
node --test tools/bench/tests/native-cargo-metadata-override.test.mjs
node tools/bench/native-cargo-metadata-override.mjs collect > host-control.json
node tools/bench/native-cargo-metadata-override.mjs compare linux-control.json host-control.json
\`\`\`

## Limitations

This is an intentionally invasive *diagnostic build* only. It changes
rustc's identity inputs under a temporary wrapper and must **not**
be silently adopted as a normal or production build configuration.
It does not replace Cargo's generated \`-C extra-filename\` values,
Cargo package/features/fingerprint logic, native build scripts' other
host inputs, final linker flags or native rustc/LLVM executables.

If raw body/artifact agreement improves after substitution, that is
evidence for a causal contribution of the altered metadata input
*under this controlled build*, **not** proof that metadata is the sole
cause of the original host differences. If disagreement remains,
the remaining host-specific compilation/linking inputs and generated
filename hashes are still live hypotheses.

The selected fixed replacement strings are **experimental
disambiguators**, not stable crate identities for general Cargo
dependency graphs. The wrapper's allowlist covers the existing
five-crate Markdown guest experiment only; it does not support
multiple versions, packages with identical crate names, arbitrary
build scripts, more elaborate features, or external application builds.
Raw Wasm byte differences alone do not prove different runtime behavior,
and the separate Markdown/WIT capability and native functional tests
remain the correctness baseline. There are no runtime-speed, guest
memory or cross-platform cost claims here.
