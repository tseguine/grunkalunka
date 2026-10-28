# P1 controlled Windows Markdown guest checkout normalization

The independently compiled Rust Markdown guests on Linux, Windows and
Apple Silicon produced different Wasm SHA-256 files in the same #98 CI run.
The [section-level experiment](p1-markdown-guest-sections.md) found differences
in the code, element, function, and data sections, not only custom metadata.
The [same-host clean rebuild experiment](p1-markdown-clean-rebuild.md) then
found stable output across two clean builds **within each host**.

Windows Git checked-out source and manifest files were not byte-identical
to Git's canonical blobs, while the Linux and macOS checkouts matched.
This could contribute to output differences, but does not prove causation.

## Controlled Windows experiment

The [optional workflow](../.github/workflows/p1-markdown-windows-eol.yml)
runs only on native Windows x64. Its
[helper](../tools/bench/windows-eol-markdown.mjs) creates a **disposable
independent Git checkout** of the same tested revision and uses **one physical
checkout path and one absolute Cargo target directory** for both builds:

1. Configure the temporary checkout with `core.autocrlf=true`; check out
   the exact source commit. Record Git blob and checkout hashes for six
   relevant tracked files, then compile the real `outline_guest` for
   `wasm32-unknown-unknown` with locked/offline Cargo, dev profile and
   `CARGO_INCREMENTAL=0`.
2. In the **same temporary checkout**, disable text conversion and forcibly
   rewrite files from the index with `core.autocrlf=false` and `core.eol=lf`.
   Verify the selected source and manifest checkout files now match their
   original Git blobs, while the canonical Git content and revision remain
   unchanged. Clean the Wasm target products in the same target directory,
   confirm the original guest was removed, and rebuild.
3. Record the two guest file sizes and SHA-256 digests. Classify whether
   identical Wasm bytes resulted from CRLF and LF inputs.

The helper never resets or changes the caller's real checkout or global Git
configuration; it removes only its own temporary directory. Native Windows,
Rust toolchain, Node and target are held constant in a single hosted job.
Both compiled artifacts are validated as bounded core Wasm. The pure tests
check that missing CRLF conversion, incomplete LF normalization, mismatched
Git sources and malformed Wasm fingerprints fail closed.

To reproduce on Windows with Node and the Rust Wasm target installed:

```sh
node --test tools/bench/tests/windows-eol-markdown.test.mjs
node tools/bench/windows-eol-markdown.mjs > markdown-eol.json
```

The dedicated workflow prints its final JSON receipt into the Actions log
without uploading source, binaries or benchmark artifacts. **Different output
is a diagnostic observation, not a failing gate.** Inconsistent source,
invalid output or a failed build is a failure.

## Interpretation boundaries

If the two guest hashes differ, Windows checkout text normalization is a
demonstrated contributor *in this controlled environment*, not necessarily
the sole reason for cross-host differences. If they match, the original
Windows/Linux difference must involve other build inputs or conditions
(or combined effects not isolated here).

This experiment is not proof of compiler determinism, Rust semantic
compatibility, cross-OS application portability, or measured performance.
Native Rust host libraries, installed compiler target distributions, build
scripts and machine load are not independently attested. The existing
Markdown/WIT native parity and capability tests remain correctness evidence.
