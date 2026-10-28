# Optional second core-Wasm cross-architecture replay: affine binary64 (P1)

This checks a different computation from the [GCD portability probe](portable-core-replay.md):
the maintained compiler emits a small numeric affine function
`(a * 2 + b) / 3` from the checked-in
[affine JS fixture](../examples/numeric-workflow/affine.js). The independent
[verifier](../tools/bench/verify-portable-affine.mjs) runs the **same Wasm bytes**
through Node/V8 and the native Grunkalunka Wasmtime `eval` command, comparing
twelve fixed binary64 cases. These include signed zero, one-third rounding,
subnormal underflow, overflow, infinities and NaN. Non-NaN outputs must match
their exact 64-bit patterns; NaN output payloads are intentionally not fixed.

## Reproducing a local single-host check

Build the existing CLI first. Use a *new* destination path: compilation refuses
to overwrite files.

```sh
cargo build --locked -p grunkalunka-cli
./target/debug/grunkalunka compile examples/numeric-workflow/affine.js /tmp/affine-new.wasm
node --test tests/build/portable-affine.test.mjs
node tools/bench/verify-portable-affine.mjs /tmp/affine-new.wasm \
  --runner ./target/debug/grunkalunka
```

On Windows use the CLI's `.exe` suffix and another temporary path. The test
script checks the literal independent oracle against the source and exercises
failure cases, but synthetic tests alone do not establish compiler execution.

The verifier accepts one pre-existing Wasm file (64 KiB maximum) and the native
CLI path. It neither compiles JS nor runs a JS source oracle. It rejects modules
with imports and requires an exported `run` function; Node and Wasmtime must
both pass all twelve golden results, and the artifact SHA-256 must not change.
The native command has a 60-second subprocess watchdog and bounded output.
The report contains engine results, hash, native host OS/architecture and no
timing, speedup or memory claims.

## Native Linux ARM64 execution

The separate [optional workflow](../.github/workflows/p1-affine-arm64.yml)
uses standard Linux x64 and native Linux ARM64 hosts with Node 24.22.1. Its
x64 job compiles **one** module from the repository source, checks both engines
and emits its bounded literal base64 bytes, SHA-256 and source hash through
ephemeral job outputs. The ARM64 job verifies all three transfer identities,
builds only its native Wasmtime CLI, and executes the **transferred x64-produced
module** in Node/V8 and Wasmtime. It does **not** compile affine source on ARM64.

The workflow runs for PR changes to this experiment's four source files or
by explicit manual dispatch. It does not change canonical CI jobs, post
benchmark reports, retain generated artifacts, create a release or upload
external data. Actual ARM64 evidence requires a completed successful hosted
run; an x64-only local replay does not establish this.

This is a second *numeric core-Wasm* portability case, not a general JS/TS
language, browser, WIT component, whole-application or cross-OS claim.
Compiled bytes have an observed SHA-256, not a signed provenance attestation.
No measured costs or cross-host performance rankings are implied.
