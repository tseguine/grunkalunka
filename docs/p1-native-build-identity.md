# Independent native-compiler GCD artifact identity (P1)

The same-byte [Linux x64-to-ARM64 replay](portable-core-replay.md) and
[Linux-to-Windows replay](p1-portable-windows.md) demonstrate that the
**transferred** compiler-emitted Wasm works on other hosts. They do not
demonstrate that running the **native Grunkalunka compiler independently** on
those hosts produces the same Wasm artifact.

This separate [optional workflow](../.github/workflows/p1-native-gcd-identity.yml)
tests that narrower reproducibility question. It builds the existing
`grunkalunka-cli` independently on standard Linux x64, Windows x64 and
native Apple Silicon macOS ARM64 runners. It uses Node 24.22.1 everywhere
and logs each native Rust host/toolchain; the Rust `stable` channel is not
pinned to a specific compiler release.

## Controlled input and required result

Every job retrieves the original
`examples/numeric-workflow/gcd.js` bytes from the **same checked-out Git
tree** using `git show HEAD:...`. Compilation happens from those canonical
bytes in a disposable `.js` file, so Windows `core.autocrlf` cannot
silently alter the tested input. Each job invokes its **own built native
compiler**, never recompiling or evaluating the producer's binary as a
substitute.

The Linux producer sends the bounded literal Wasm bytes, source SHA-256
and artifact SHA-256 through ephemeral GitHub job outputs. Each other
host independently compiles the canonical source, verifies its source
identity, and requires its entire emitted Wasm byte sequence to be
**identical** to the Linux artifact. A different compiled byte is a failed
experiment, even if the executable's result happens to be correct.
The native host and Node architecture are checked, including rejection
of Rosetta on the Apple Silicon runner.

All three jobs also run the existing strict
[portable GCD replay](portable-core-replay.md) on their locally compiled
Wasm, independently checking eight numeric goldens in Node/V8 and native
Wasmtime. Four negative tests reject changed source, changed artifact bytes,
wrong SHA-256, noncanonical base64 and excessive transfer size. No
new compiler flags, alternate build tools or source language admission
is introduced.

The implementation lives in
[`native-build-identity.mjs`](../tools/bench/native-build-identity.mjs),
and can be reproduced from a checked-out repository after building
the CLI:

```sh
node --test tests/build/native-build-identity.test.mjs
node tools/bench/native-build-identity.mjs produce ./target/debug/grunkalunka
```

Use the `.exe` suffix on Windows. In `verify` mode the helper requires
`EXPECTED_SOURCE_SHA256`, `EXPECTED_WASM_SHA256` and
`EXPECTED_WASM64` environment values obtained from the actual producer
receipt; missing or malformed values are rejected. The helper writes only
its JSON report to stdout and removes disposable local files.

## Interpretation and limits

The successful [exact-head #98 run](https://github.com/tseguine/grunkalunka/actions/runs/40679064547)
established that independently built native CLI compilers on Linux x64,
Windows x64 and Apple Silicon ARM64 all emitted the **same 614-byte GCD Wasm**,
SHA-256 `85524b17227917bbfafa71fce3129a2003c8a84e21ba9c32c0cc7f4183980bce`. Unlike a same-byte *replay*, this experiment
compiles the source separately on each host. Passing numeric execution
does not alone satisfy the stricter artifact identity check.

The observation is not a guarantee of build reproducibility for all
programs, compiler versions or platforms. Rust stable may advance;
compiler and dependency provenance are observed rather than attested.
File content equality does not prove independent compiler binary
identity, Wasm semantically equivalent output for other inputs, or
Node/Wasmtime performance equivalence. No timing scores are compared,
no deployment is performed and no artifact upload or published
benchmark report is created. Canonical required CI is unchanged.
