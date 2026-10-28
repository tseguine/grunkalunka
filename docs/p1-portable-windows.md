# Same-byte Linux-to-Windows numeric core-Wasm replay (P1)

The [GCD portable-artifact helper](portable-core-replay.md) can replay the
maintained compiler's numeric GCD module on native Node/V8 and Wasmtime without
recompiling the source. The optional
[Linux x64 to Windows x64 CI workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-gcd-windows.yml)
tests **operating-system portability** of the exact artifact, complementing
the established Linux x64 to Linux ARM64 replay in #83.

## Execution and evidence boundary

The Linux x64 producer uses native Rust and fixed Node 24.22.1, compiles
`examples/numeric-workflow/gcd.js` into core Wasm once, then runs
`tools/bench/portable-gcd.mjs` to prepare a source/artifact manifest and
validate all eight independent GCD goldens in Node/V8 and Wasmtime.

The Windows 2025 x64 receiver checks native Windows, Node x64 and the same
Node patch version. It builds only the native Wasmtime CLI, **not GCD JS**.
The two jobs pass the bounded, canonical base64-encoded literal Wasm bytes
and manifest through ephemeral GitHub Actions job outputs, with an explicit
SHA-256. The receiver verifies the exact decoded artifact hash and size,
producer architecture, checked-in source hash, and canonical byte transfer
before replaying both Node/V8 and Wasmtime. Both engines must match all eight
fixed goldens, with the original artifact rehashed afterward.

This PR workflow is optional: it runs when its own definition changes or on
explicit manual dispatch. It does not add an artifact upload, public release,
package, deploy, retained benchmark data or dependency. The required Linux,
Windows and Apple Silicon macOS CI suite is unchanged. Job logs contain the
receiver receipt, including the independently built Windows Wasmtime CLI hash.

A successful exact-head hosted run would support **only** this Linux-produced
numeric core-Wasm module's identical-byte execution on Windows x64; the same
source separately compiled on Windows is *not* the experiment. The two jobs
are on different OSes but both x64, so this is not another CPU-architecture
claim. Artifact hashes are evidence of observed byte identity, not signed
provenance or an attestation. No general language compatibility, component
portability, runtime memory attribution, engine performance equality or
speed ranking follows from a passing check.
