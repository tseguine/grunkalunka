# Actual Markdown guest WIT-cost observation (P1)

The private Markdown guest already tested in [capability-backed outlines](document-outline-component.md)
is a controlled application-shaped workload for P1. This probe uses the
**same compiled core-Wasm guest and WIT adapter** with explicitly authorized
in-memory document text. It measures more work than the
[string transport control](document-cost.md), without claiming production
Markdown editor performance.

From the repository root, after fetching locked dependencies and the target:

```sh
cargo build -p grunkalunka-markdown --example outline_guest --target wasm32-unknown-unknown --locked --offline
cargo run -p grunkalunka-runtime --example markdown_cost --locked --offline -- \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm \
  --workload unicode --samples 5 --iterations 20 --warmup 5
```

The command accepts exactly one guest path followed by unique option/value pairs.
Workloads are empty, unicode (three headings with CRLF and multibyte text),
dense (24 headings), and wide (one 3,600-byte UTF-8 label). Every source stays
within the existing 4,096-byte transport limit and 64-heading result limit.
Options bound samples to 2..12 (default 5), per-batch iterations to 1..100
(default 20), and warmup to 0..100 (default 5). This caps input and host calls,
**not** total process memory, elapsed wall time or scheduler activity.

## Observations and independent correctness checks

The JSON report retains the selected source and native-comparable outline
rows as hex, component/guest byte lengths, OS and architecture, the workspace's
pinned Wasmtime dependency, fuel policy and separate timing observations:

- One WAT guest-embedding operation and component compilation, outside call loops.
- Fresh Store/component instantiation samples with one precompiled component
  and one linked host; destruction is excluded from each measurement.
- Repeated complete WIT inspect calls under granted and denied profiles,
  including host dispatch, Rust guest Markdown parsing, original byte positions,
  canonical ABI list/string lifting, allocations and post-return cleanup.

The denied profile returns a typed policy error without a backing read or parse.
The granted profile returns owned headings and normalized native-comparable rows.
Before measuring, known fixture headings and denial/unknown-grant behavior are
checked. After samples, the probe verifies backing-read and cleanup counts and
confirms returned records still exist after dropping their component instance.
Assertions are **outside** timing loops. Sample order alternates granted and
denied, with raw milliseconds and min/median/max. No performance threshold,
ranking or ratio is enforced.

The canonical node scripts/check-document-outline.mjs checks four small profiles
and independently compares every reported outline against the same library's
native outline_guest executable using the supplied **source bytes**, not a
second implementation of Markdown. This validates projection parity; it
does not independently certify CommonMark. Existing document authority, quota
and malformed-result suites are separate evidence.

## Rust/Wasmtime whole-process memory checkpoints

The same real Markdown/WIT cost probe now reports five **process-wide**
Rust/Wasmtime snapshots, independent of its timing intervals: after reading
the guest and creating the Engine (before embedding), after component
compilation and linker setup, after fresh-instantiation calibration and
creation of the granted/denied instances, after warmup, and after verified
call batches but **before disposing of the two instances**.

On Linux, `/proc/self/status` supplies current `VmRSS` and
process-lifetime high-water `VmHWM` (converted from KiB to bytes), under
the named `processMemory.snapshots` phases. The existing bounded parser
and monotonic-high-water validation from the core-Wasm cost probe are reused,
not silently reimplemented. Linux canonical integration checks require all
five real observations, nonnegative integral byte values, current RSS no
greater than peak, and nondecreasing peak measurements. On Windows the same
five phases use the separately verified native
`Kernel32!K32GetProcessMemoryInfo` API and report
`source: "windows-psapi-working-set"`, containing whole-process
**working-set** readings in the original `rssBytes`/`peakRssBytes`
receipt fields. Windows now requires real nonzero native observations,
not fabricated Linux RSS. The two operating systems have different
accounting semantics, so their values are not directly comparable.
macOS explicitly retains `unsupported-on-this-host` and
`snapshots: null`, not zero memory.

These observations describe the **entire Rust process**, including the
Markdown guest, the Wasmtime runtime, JIT caches, host allocations,
linker, input/output and the measurement harness. The first peak may
already include earlier compiler/cache work; even differences between
snapshots cannot be attributed to guest memory. Source/workload bounds
still do not bound host RSS, and no performance, resource budget or
cross-host ranking is enforced. Snapshots are outside the original
measurement timers. They complement, rather than replace, the separate
GCD core-Wasm RSS observations and Node/V8 process memory data.

## Repeatability and local report provenance

The canonical document check also invokes the **same Unicode workload twice in
separate cost-probe processes** on one CI host. It reruns the native reference for
each reported source, compares the two complete fixture projections, sampling
configuration, declared environment and artifact sizes, and verifies that the
actual guest Wasm file's SHA-256 stays unchanged. It does **not** require equal
timings or establish controlled thermal, cache or scheduler conditions. The
CI logs record the guest SHA-256, but do not retain either timing JSON report.

To retain raw observations for manual investigation, run the example command
above **twice with exactly the same guest file and options**, redirecting each
JSON stdout stream to a different local file. Record these observations
alongside both files from the same checkout:

```sh
git rev-parse HEAD
rustc -Vv
cargo -V
node -p "JSON.stringify({node:process.version,v8:process.versions.v8,cpu:require('node:os').cpus()?.[0]?.model??null})"
node -p "require('node:crypto').createHash('sha256').update(require('node:fs').readFileSync('target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm')).digest('hex')"
```

Also record the actual Cargo profile/RUSTFLAGS, CPU/virtualization details,
machine load and whether build products were reused. For an interpretable
same-workload comparison, require matching fixture source/output bytes,
samples/iterations/warmup, OS/architecture, Wasmtime dependency, guest binary
SHA-256 and build conditions; matching byte **lengths** alone are insufficient.
The report does not independently record the guest hash, compiler flags or CPU
model, so those must accompany saved reports. Even matched metadata cannot
establish steady CPU frequency, equal caches or JIT state. Collect several
whole runs; do not apply the numeric-GCD report comparator to these differently
shaped Markdown reports, rank hosts or infer isolated host overhead.

## Interpretation boundaries

This cannot isolate *pure* host-call overhead or pure Markdown parser cost:
the full component path includes transport, guest parsing, result projection
and cleanup, while the prior WIT read fixture measures a different operation.
There is no native timing series to subtract, no aggregate memory accounting,
no controlled cold caches, no parallel host concurrency and no production vault.
Fuel is intentionally disabled for this trusted measurement; normal tests cover
the separate execution-budget failure behavior. Retain raw reports with commit
SHA, Rust/Node versions, CPU model, build profile/flags and machine conditions
before comparing hosts. The report itself records only static environment and
sampling metadata.

This is **one bounded P1 observation**, not completion of P1, A1, filesystem
confinement, a public document API or a measured general framework application.

For independently saved reports with the actual guest/runner binary hashes,
source-revision and host metadata, use the [P1 receipt capture](markdown-capture.md).
This adds provenance checks, not performance rankings or certified comparability.
The [bounded same-process comparison](markdown-comparison.md) admits only
binary-identical paired captures, not arbitrary saved reports or host rankings.
