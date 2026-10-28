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

The canonical replay tests on hosted Linux and Windows x64 and the
native Apple Silicon macOS ARM64 gate confirm their respective environments.
The dedicated [#83](https://github.com/tseguine/grunkalunka/pull/83)
Linux x64-to-ARM64 job additionally transferred the **identical producer bytes**
and verified all eight GCD goldens in native ARM64 Node/V8 and Wasmtime.
This is verified cross-architecture execution for one bounded numeric artifact,
not broad ARM64 compiler or application compatibility. Equal emitted binaries establish artifact
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
are added. P1 remains in progress pending broader workload portability and
comparable measured application costs.

## Optional x64-to-ARM64 hosted execution

The dedicated [P1 cross-architecture workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-portable-arm64.yml)
is separate from the required Linux/Windows x64 and native Apple Silicon macOS ARM64 CI. It uses a standard
`ubuntu-24.04` x64 producer and `ubuntu-24.04-arm` Linux ARM64 receiver,
with Node **24.22.1** selected on both. Only changes to that workflow itself
trigger this optional PR smoke; it can also be dispatched manually after merge.
The independently maintained canonical gates now run on Windows x64, Linux x64
and native Apple Silicon macOS ARM64; this optional workflow does not modify them.

The producer builds the existing native compiler, compiles the checked-in GCD
source once, validates the identical output in Node/V8 and native Wasmtime, and
passes the exact Wasm bytes, manifest, SHA-256 and bounded **producer replay
receipt** through ephemeral GitHub job outputs. The ARM64 receiver verifies
those transferred identities, replays the same bytes through its own native
Node/V8 and Wasmtime, and uses the strict [saved-receipt comparator](portable-replay-comparison.md)
on both actual host reports. The resulting receipt-pair summary checks the
observed x64/ARM64 host identities and both golden results without comparing
their timings.
No artifact upload action, release or persistent benchmark record is added.
The ARM64 job verifies the transferred hash, its native aarch64 architecture,
and Node's ARM64 identity. It builds an ARM64 native Wasmtime CLI, then replays
the **same x64-produced bytes** in Node/V8 and Wasmtime without recompiling
GCD source. The exact binary64 golden results and receipt hashes are checked,
and the receiver report is printed into the job log.

The successful [exact-head #83 workflow](https://github.com/tseguine/grunkalunka/actions/runs/40652492744)
established **only that core-Wasm GCD fixture** executing on Linux aarch64
through Node and Wasmtime. It would **not** establish
general JS/TS compatibility, a complete ARM64 Grunkalunka platform, macOS
ARM64 support, comparable performance or guest-attributed memory.
Timings are observations, never thresholds. Workflow job outputs have size
limits and secret filters; a missing/redacted transfer fails as **no evidence**.
The workflow is not a substitute for broader canonical checks and does not
resolve the separate [macOS Intel/ARM runner steering](https://github.com/tseguine/grunkalunka/issues/79).
