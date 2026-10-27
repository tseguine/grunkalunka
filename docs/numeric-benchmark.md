# Numeric build and execution timing probe

Run `node tools/bench/numeric.mjs` after building the native compiler. The command
compiles the checked-in numeric GCD example once and prints a JSON report to stdout;
it creates no persistent output or uploaded artifact. For a different compiler:

```sh
node tools/bench/numeric.mjs --compiler ./target/release/grunkalunka --build-label release --samples 9 --iterations 100000 --warmup 5000
```

Add `.exe` to an explicit Windows executable path. Defaults resolve relative to
the script; an explicit compiler path resolves against the caller's directory.
No compiler is built or downloaded. The default executable is the **debug** build;
`--build-label` is caller-supplied provenance, not an inferred build configuration.
Record actual Rust flags/features separately when sharing results. This is a
single-workload measurement tool, not completion of P1 or a platform speed claim.

## What is measured

- `buildApiMs`: one source-text build API invocation, including the native process,
  source/output I/O and temporary cleanup. It is not isolated frontend/lowering time.
- `startup`: first observed Wasm module construction, instantiation, JS function
  call and Wasm function call. Each is a single observation, not a distribution.
  JS module loading/parsing is not timed. These are not equivalent end-to-end
  startup measurements and must not be compared as such.
- `batches`: repeated JS and Wasm calls after explicit warmup. Every batch includes
  the same JavaScript input-selection loop and checksum accumulation; the Wasm
  path also includes a JS-to-Wasm call per iteration. This is not pure guest time.
  Order alternates per sample. Raw millisecond timings, minimum, median and maximum
  are retained; no automatically selected winner or performance threshold is used.

Eight fixed small-integer pairs exercise negative/zero/coprime/non-coprime inputs.
Both engines must agree with fixed goldens before batches begin. Every timed batch
must produce the expected checksum; checks run outside the measured region. A bad
result aborts the report. This does not replace the full differential corpus.

Options use unique `--name value` pairs. Samples range from 2 to 50; iterations
from 1 to 1,000,000; warmup calls from 0 to 1,000,000 per engine. Defaults are 9,
100,000 and 5,000. The entire worker phase has a 120-second watchdog; compilation
retains the build API's separate limit. The worker is terminated before settling,
including on errors. No imports are granted. This is for trusted local artifacts,
not a memory-bounded service; metadata reads and hashing have no deadline.

## Reproduction and interpretation

Reports include source, Wasm, harness and compiler SHA-256 hashes, emitted size,
compiler version/build label, Node/V8 versions, OS/architecture/CPU and worker
options. The compiler hash is checked again afterward to detect an intervening
rebuild. Run with an idle machine and unchanged binaries/source; retain several
whole reports to assess run-to-run variance. Existing local measurements need not
use this harness, but comparisons require equivalent work and provenance.

OS and engine caches are **not** flushed or controlled. Reading/hashing the
compiler and querying its version occur before timing its compilation invocation,
so do not describe that measurement as cold start. Warmup does not establish that
JIT tiering has finished; GC, scheduling and timer overhead remain. Short batches
and single-call times can be dominated by noise. CI uses tiny batches only to test
report correctness, never to enforce a timing ratio. No Wasmtime/Bun measurements,
process-memory accounting, worker-startup timing or multi-workload score is supplied.

`node --test tests/build/benchmark.test.mjs` checks configuration, summary statistics
and the measurement code using an independent small-input Wasm fixture. After a
native build, `node --test tests/build/benchmark-integration.test.mjs` exercises the
actual command and compiler from another working directory. Both are included in
`node scripts/check.mjs`. Timing uses Node's [performance clock](https://nodejs.org/api/perf_hooks.html#performancenow).

To compare saved reports with matching workload and environment metadata, use the
[comparison command](benchmark-comparison.md). It retains raw samples and rejects
incompatible reports rather than silently presenting a timing ratio.
