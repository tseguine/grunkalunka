# Controlled Linux Markdown guest source-checkout paths (P1)

The independently compiled Rust Markdown guest consistently differs in its
Wasm body and data sections between Linux, Windows and Apple Silicon, even
though its native functional checks pass. The [native symbol-name
analysis](p1-markdown-named-body-correlation.md) found 1,888 v0-style names
per host and only 195 exact-name matches between Linux and either receiver.
Previous experiments established stable within-host clean rebuilds, identical
installed wasm32 target libraries, and substantial sensitivity to the
number of Rust code-generation units. These observations do **not** identify
the remaining source of the cross-host differences.

## One-host source-path control

The [optional dedicated Linux workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-markdown-linux-checkout-path.yml)
and [standalone collector](../tools/bench/markdown-linux-checkout-path.mjs)
test one variable without modifying the main source checkout.

Using a **single Linux x64 runner**, the collector:

1. Creates two independent disposable local Git repositories under
   different absolute source paths. Both fetch and detach to the **exact
   tested head SHA**, with checkout conversion disabled. It checks six
   selected source, lockfile and toolchain inputs against their original
   Git blob SHA-256 values, and checks they remain unchanged after builds.
2. Uses a **single fixed absolute `CARGO_TARGET_DIR`** and
   `CARGO_INCREMENTAL=0` across every build. In both source directories,
   builds the existing `grunkalunka-markdown --example outline_guest`
   for `wasm32-unknown-unknown`, with locked/offline Cargo and the
   unchanged development profile. It cleans Wasm target products and
   verifies the guest has been removed between runs.
3. Repeats the same two-source-directory builds with precisely one
   rustc flag, `--remap-path-prefix=<that checkout>=/p1/grunkalunka`,
   supplied as an encoded Cargo Rust flag. Each source checkout maps to
   the **same logical source prefix** in its own compilation. No other
   flag or dependency changes are requested.

The four builds are *default A, default B, remapped A, remapped B*.
Every resulting guest is bounded, checked as valid core Wasm, and
fingerprinted by full SHA-256, section hashes, and raw defined-function
body hashes. The report separately compares the two source directories
under default settings, the two directories under remapping, and each
directory's default versus remapped output. Positional and order-free
exact body overlap is included, without treating different function
indices as source identities. No guest binary is uploaded.

To reproduce on Linux x64 after installing the Rust wasm32 target and
fetching locked dependencies:

```sh
node --test tools/bench/tests/markdown-linux-checkout-path.test.mjs
node tools/bench/markdown-linux-checkout-path.mjs > checkout-path.json
```

The script creates, cleans, and removes **only its temporary Git
repositories and private Cargo target directory**. It does not reset,
overwrite, or change the invoking checkout, global Git configuration,
compiler code, package dependencies or canonical CI.

## Interpretation and limitations

If default builds differ between paths, the physical source checkout
location contributes to observed raw output differences *on one host*.
If the two remapped outputs then match, Rust's source-path remapping is
a plausible remedy for at least that controlled contribution. If both
pairs match, a changed checkout location alone did not reproduce the
cross-host difference on this runner.

The `--remap-path-prefix` flag is a **best-effort textual remapping of
source paths**, not a complete normalization of linker paths, host
environment variables, Cargo crate identities or generated inputs.
See the [rustc documentation](https://doc.rust-lang.org/rustc/remap-source-paths.html).
The local comparison cannot establish whether Windows-style or macOS
paths cause their cross-host differences. It does not decode v0 mangled
names, normalize function-index immediates, prove semantic equivalence,
measure performance, or declare general build reproducibility.
A changed hash remains an observation rather than a pass/fail threshold.
Actual compilation or provenance-check failures still fail closed.
