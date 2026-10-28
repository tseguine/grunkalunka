# Remaining Wasm dependencies: staged Cargo metadata control (P1)

This experiment extends the [five selected-crate normalization study](p1-native-cargo-metadata-override.md)
without changing the three modes or their compiler substitutions from
[PR #147](https://github.com/tseguine/grunkalunka/pull/147).

That first experiment found that native Linux, Windows and Apple Silicon
independently built the unchanged Rust Markdown guest from the same source
with **four observed generated wasm32 per-crate metadata inputs replaced**.
This raised exact raw body overlap against Linux from **945 to 1,429** on
Windows and **913 to 1,428** on Apple Silicon. The native build-script
metadata contributed no further difference under that control. The
resulting Wasm artifacts were *not* identical.

The actual wrapper log also revealed two additional wasm32 Cargo dependency
compilations—**`bitflags` and `memchr`**—whose original generated metadata
were deliberately passed through unchanged. This bounded follow-up tests the
incremental effect of substituting just those two additional inputs.

## Four controlled variants

On each independent native host, the same temporary compiled Rust
`RUSTC_WRAPPER` executable and private Cargo target directory are used
throughout. Each variant starts after a **complete private Cargo clean**,
including native host build products, with locked/offline dependencies,
`CARGO_INCREMENTAL=0` and serialized compilation (`-j 1`).

| Mode | Cargo-generated metadata replaced |
| --- | --- |
| `passthrough` | None; records the actual generated metadata values |
| `target` | Original four selected wasm32 crates: `unicase`, `pulldown_cmark`, `grunkalunka_markdown`, `outline_guest` |
| `target_plus_deps` | Same four, **plus `bitflags` and `memchr`**, but not the native build script |
| `all` | Original four, **plus native `build_script_build`**, retaining the original #147 behavior and leaving `bitflags`/`memchr` untouched |

The new mode assigns stable diagnostic strings
`p1bitflags` and `p1memchr` only in `target_plus_deps`, **replacing
their one original sixteen-digit generated `-C metadata` input**.
The wrapper never changes `-C extra-filename`, `--extern`, feature
cfgs, native linker options or the real compiler executable.

Every mode logs all actual crate calls, including any additional
unselected crates. The report explicitly requires all seven named
compiler groups, validates exact original/effective metadata and its
consistency throughout the host's four builds, and rejects unexpected
targets, missing or duplicate records or malformed input. Any other
actual compiled crates remain untouched.

The [existing optional native workflow](https://github.com/tseguine/grunkalunka/blob/d8e5bdc6d413722ebcc90878744a1a2be18e5096/.github/workflows/p1-native-cargo-metadata-override.yml)
rebuilds the unchanged Markdown guest on Linux x64, Windows x64 and native
Apple Silicon ARM64. The bounded JSON receipt records exact artifact
SHA-256 and size and complete standard/custom Wasm section and raw
function-body fingerprints. Independent receivers compare each mode to
Linux and report the **incremental `target` versus `target_plus_deps`**
effect within each host, without imposing artificial equality thresholds.

## Limits

This intervention is intentionally **not** a production Cargo metadata
policy or an override for the maintained compiler. It tests whether two
specific observed wasm32 dependencies contribute additional
host-dependent raw bytes after the first four selected crate metadata
values have been normalized. Even matching all seven selected generated
metadata values would **not** normalize Cargo's `-C extra-filename`
hashes, host-specific linker behavior, native compiler binaries, target
cfgs, build environments or arbitrary other dependencies.

Both the before/after artifact identities and body overlap counts are
diagnostic evidence, not proof of JavaScript compatibility, guest
semantics, reproducibility for other crates or measured performance.
No binary or source text is uploaded, no canonical CI check is weakened,
and the experimental wrapper remains confined to isolated temporary
builds. The original Markdown/WIT correctness tests remain separate.
