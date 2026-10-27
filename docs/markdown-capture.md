# Saving Markdown/WIT measurement receipts (P1)

The [real Markdown component-cost probe](markdown-component-cost.md) prints raw
timings, input fixture bytes, and authorization/cleanup evidence, but it does not
identify the precise guest or native runner binaries. This wrapper adds those
identities and the observed measurement environment. It **does not** make the
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

- SHA-256 hashes of the actual Wasm guest, native benchmark executable,
  capture wrapper, Markdown benchmark source, fixed WAT adapter/embedding
  sources and Cargo.lock. The guest and executable are checked again after
  the run to reject observed replacement during measurement.
- Repository HEAD, a boolean indicating **tracked** working-tree changes,
  a caller-declared build label, and a fresh observation timestamp.
- OS and architecture, OS release, CPU model/core count, Node and V8 versions,
  Node options, and the locally *observed* rustc version when available.
- The explicit statement that OS/engine caches are uncontrolled.

The wrapper rejects unknown/duplicate/out-of-bound arguments, wrong profile
counts, mismatched host metadata, invalid timing arrays or statistical summaries,
wrong guest byte size, non-UTF-8 source fixture bytes, unauthorized backing reads
and changed observed binaries/repository state. These are input-integrity and
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
