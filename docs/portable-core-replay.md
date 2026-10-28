# Replaying one emitted core-Wasm artifact on another machine (P1)

The [two-engine numeric cost probe](core-engine-cost.md) already validates one
Grunkalunka-compiled numeric artifact in Wasmtime and Node/V8. Its standard
runner *compiles a fresh artifact on each invocation*, so two machines do not
necessarily execute identical bytes. This bounded helper separates preparation
from replay: a copied `.wasm` file is verified by SHA-256 and checked against
fixed independent GCD goldens, **without invoking a source compiler on the
receiving host**.

From a repository checkout with the native compiler already built:

```sh
# Prepare on the original host (use a new destination each time).
./target/debug/grunkalunka compile examples/numeric-workflow/gcd.js /tmp/gcd.wasm
node tools/bench/portable-gcd.mjs prepare /tmp/gcd.wasm --build-label debug \
  > /tmp/gcd-manifest.json

# Copy the same two files to a second host or architecture. Replay in Node/V8
# plus a locally available Grunkalunka Wasmtime CLI, but DO NOT recompile Wasm.
node tools/bench/portable-gcd.mjs replay /tmp/gcd.wasm /tmp/gcd-manifest.json \
  --runner ./target/debug/grunkalunka --samples 5 --iterations 100 --warmup 20
```

For Windows use the native compiler executable's `.exe` suffix. The `compile`
command is an **explicit preparatory step**; `prepare` reads and checks the
existing artifact but never invokes the compiler. `replay` reads that artifact
and its manifest, then runs the maintained Node/V8 core replay subprocess and,
when supplied, the native CLI's existing `eval` mode. The CLI reads hexadecimal
binary64 pairs through stdin and evaluates the **same artifact bytes** through
Wasmtime. The caller chooses output files and redirects the manifest explicitly;
these scripts write only a JSON result to stdout and never download, publish,
install, overwrite, or upload artifacts. Paths containing spaces/Unicode are
handled through native argument arrays, not shell interpolation.

If Wasmtime is unavailable on the receiver, choose `--node-only` **instead of**
`--runner`. The result explicitly reports `not-run`; it cannot be counted as a
two-engine portability observation. The options are mutually exclusive, and
neither may be omitted. `--samples` (2..20), `--iterations` (1..10,000) and
`--warmup` (0..10,000) feed the unchanged Node core replay only. Defaults are
5, 100 and 20. The original fixture's eight expected results, including signed
input cases, are checked outside timing loops on both engines. The incoming
Wasm file is bounded to 1 MiB and manifest to 64 KiB; subprocesses have a
60-second watchdog and 1 MiB output limit. No guest imports are allowed.

## Evidence and interpretation

The manifest retains an observed source fixture fingerprint, Wasm bytes and
SHA-256, fixed numeric input/output bit patterns, caller-provided build label,
producer OS/architecture/CPU/Node details, and Git revision when available.
It is **not** cryptographic evidence of how the original artifact was compiled
or signed hardware attestation. Replay requires the exact artifact hash and
unchanged independent goldens; it records receiver conditions, Node/V8 raw
startup/warmed samples and whole-process memory snapshots, and the actual
Wasmtime CLI executable hash and result checks when selected. The received
artifact is rehashed after both engine checks to reject observed replacement.

A successful replay on today's hosted Linux, Windows and macOS x64 checks
confirms three **x64 environments**, not ARM64 support. Collect this same pair
of files on an actual ARM64 host and retain the receiver report before claiming
cross-architecture execution. Equal emitted binaries establish artifact
identity, **not** equal runtime costs or cross-machine speed rankings. Node's
startup, cache state, flags, JIT tiering, CPU load and memory footprint remain
uncontrolled. Native CLI `eval` checks correctness only; its process launch
is not a Wasmtime performance measurement. No ratios or performance gates are
computed. This is one narrow portable numeric fragment, not general JS/TS,
Component Model, filesystem capability, or production application support.

The canonical benchmark integration test compiles the real GCD source through
the maintained CLI, prepares the manifest and verifies Node and Wasmtime against
exact binary64 goldens. It also checks explicit Node-only mode, malformed
manifest data and changed Wasm bytes. No shared CI runner or new dependencies
are added. P1 remains in progress pending actual ARM64 observations, broader
workloads and comparable measured application costs.
