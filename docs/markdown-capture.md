# Saving Markdown/WIT measurement receipts (P1)

The [real Markdown component-cost probe](markdown-component-cost.md) prints raw
timings, input fixture bytes, authorization/cleanup evidence and now SHA-256
identities of its actual guest and generated component bytes. It does **not**
identify the precise native runner executable. This wrapper independently
hashes the supplied guest file, verifies that it matches the native report's
guest hash, and records the runner identity and observed environment. It **does not** make the
measurements intrinsically comparable or establish a speed ranking.

After building the native and Wasm examples offline:

```sh
cargo build -p grunkalunka-markdown --example outline_guest --target wasm32-unknown-unknown --locked --offline
cargo build -p grunkalunka-runtime --example markdown_cost --locked --offline
node tools/bench/capture-markdown.mjs target/debug/examples/markdown_cost \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm \
  --workload unicode --samples 5 --iterations 20 --warmup 5 \
  --build-label debug > unicode-report.json
```

On Windows use the .exe suffix for the native runner. Paths resolve from the
invoking working directory. The program writes **only stdout**; the redirect
above is an explicit user-controlled saved report. No build, network operation,
artifact upload, release or automatic cache reset is performed. Native benchmark
compilation, instantiation, authorized/denied calls and cleanup behavior remain
as defined by its independent executable. The wrapper invokes one previously
built executable and validates its output; it does not compile source.

The JSON envelope retains its entire original nested report plus:

- `nativeProcessElapsedMs`: a monotonic **parent-Node process** wall-clock
  interval for exactly one invocation of the already-built Markdown cost
  executable, from the subprocess spawn request through the runner's exit
  and captured stdout. This is deliberately **outside** all Rust/Wasmtime
  embedding/compile/instance/call timing loops and excludes wrapper-side
  binary hashing, Git metadata, JSON parsing and report validation. It
  includes OS process creation and teardown, WASM guest embedding,
  Wasmtime compilation and all samples, unlike any single internal timing
  series. It does not establish cold disk/OS caches, startup of a general
  application, or a performance gate.

- SHA-256 hashes of the actual Wasm guest, native benchmark executable,
  capture wrapper, Markdown benchmark source, fixed WAT adapter/embedding
  sources, Cargo.lock, **and the transitive Rust process-memory collector**
  included by the Markdown benchmark via `#[path]`. The guest and executable are checked again after
  the run to reject observed replacement during measurement.
- Repository HEAD, a boolean indicating **tracked** working-tree changes,
  a caller-declared build label, and a fresh observation timestamp.
- OS and architecture, OS release, CPU model/core count, Node and V8 versions,
  Node options, and the locally *observed* rustc version when available.
- The explicit statement that OS/engine caches are uncontrolled.

The wrapper rejects unknown/duplicate/out-of-bound arguments, wrong profile
counts, mismatched host metadata, invalid timing arrays or statistical summaries,
wrong guest byte size **or SHA-256**, incomplete/malformed guest/component
hash pairs, non-UTF-8 source fixture bytes, unauthorized backing reads
and changed observed binaries/repository state. The generated component hash
comes from the native runner and is a self-reported content identity, not
an independent component reconstruction. These are input-integrity and
correctness checks; timing values are accepted as observations with no thresholds.

**Important limits:** a checked hash does not prove how a binary was built.
The supplied build label is an assertion by the person running the benchmark;
the installed rustc version need not be the one that built the executable.
Untracked files, environment variables other than NODE_OPTIONS, compiler flags,
virtualization and CPU power state are not fully captured. A clean tracked
worktree does not exclude untracked edits or a modified binary outside Git.

Compare only matched workloads/inputs, binaries, harness hashes and documented
machine/build settings. Capture does **not** calculate a ratio, enforce
same-host identity or compare different runs. Report comparison needs its own
explicit compatibility gate, rather than silently treating equal OS/arch as
comparable host performance. To compare saved repeat observations without trusting CPU-model matches across
machines, use the [strict same-process receipt pair](markdown-comparison.md).
It refuses unrelated captures and different binaries; it does not establish
verified compiler flags or cross-build/cross-host comparisons.

CI runs a tiny sample to validate the capture path,
but deliberately does not persist timing reports or compare latency across
Linux, Windows and macOS.

## Versioned harness identity for memory observations

New captures retain the original capture-v1 JSON envelope but declare
`provenance.harnessVersion: 2` and include the SHA-256 of
`crates/grunkalunka-runtime/examples/core_engine_cost/process_memory.rs`
in the complete six-file harness identity. This shared Rust module now
affects reported Markdown/WIT process-memory observations and therefore
belongs to the measurement implementation, even though the guest and
native runner executable hashes are already recorded independently.
The new capture runner calculates its source hash outside benchmark timing.

Historical captures that omit `harnessVersion` are read with the original
five-file set; they are not retroactively assigned an unknown sixth hash.
A version-2 capture must include all six complete hashes **and** the
`processMemory` report field. Missing/invalid extra source evidence fails
instead of being silently treated as a newer compatible receipt.
Same-process pairs may use either complete generation but cannot mix them.
Neither generation proves a signed compiler invocation or actual hardware
memory attribution.

## Native Windows working-set provenance

Current Windows Markdown/WIT captures now contain real five-phase
`processMemory` readings with `source: "windows-psapi-working-set"`.
The original `rssBytes` and `peakRssBytes` keys represent current and
process-peak Windows working-set bytes, **not** Linux `VmRSS` and
`VmHWM`. Validation accepts this source only for saved Windows hosts,
checks coherent integral phase values and monotonic peak, and does not
interpret absolute byte counts as a host-comparable performance score.

Archived Windows v1/v2 receipts that explicitly recorded
`unsupported-on-this-host` with `snapshots: null` remain readable and are
never backfilled with fictional measurements. Historical macOS captures also
retain their original explicit unavailable/null evidence. The reader now
accepts a separately labeled complete `macos-mach-task-basic-info` phase
envelope only when the saved native OS is macOS, with the same phase/peak
integrity checks. The shared native collector now uses this Mach API; the
[standalone witness](p1-macos-process-memory-control.md) and the
integrated three native cost probes require separate real Apple Silicon
execution evidence. Mach task residency is neither Linux RSS nor Windows
working set. The versioned harness hashes include the real native sampler
source, so captures from changed implementations remain distinguishable.
