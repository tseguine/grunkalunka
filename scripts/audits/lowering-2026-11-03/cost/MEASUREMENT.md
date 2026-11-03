# L7 compiler cost observation protocol

This is an intermediate verification harness, not a new repository benchmark or CI gate.
The baseline is commit `926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c`, tree
`30bafc7a28c98aea808c7d9a6475f432def28084`. Its source worktree is clean.
`baseline-build-receipt.json` records the exact immutable executable, harness,
manifest, locked dependencies, compiler version, and every frozen output hash.

## Relation to the maintained probe

The repository's `crates/grunkalunka-runtime/examples/source_compile_cost.rs` and
its `source_compile_cost_probe` test time complete public compiler calls, retain
artifacts during a batch, validate exact output bytes outside timing, separately
time artifact release, and read Linux process VmRSS/VmHWM. They additionally run
Wasmtime semantic goldens for affine JS/TS and GCD JS.

This standalone path-dependency harness preserves the full compiler-call and
separate release boundaries. It links only the production compiler dependency
graph; Wasmtime and runtime dev dependencies are absent. It adds exact rejected
diagnostic outcomes, one explicit 2 MiB compiler worker, and process-isolated
paired measurements. It does not execute semantic goldens or expose individual
compiler-stage cost. Existing independent runtime tests supply semantic evidence.

## Fixed workload

`manifest.tsv` selects 60 source inputs, paired JavaScript/TypeScript for 30
families: 38 admitted and 22 rejected. Sources and expected admission were fixed
before final measurements. `workload-provenance.json` records exact source hashes
and their independently frozen matrix origins. It includes maintained affine
fixtures and GCD source; declarations and metadata breadth; block factory `.call`;
charged results; unchecked IIFEs; lifted capacity; statement, initializer and local
boundaries; mutable and managed closures; async/mixed forwarders; and first errors.
The TypeScript GCD case uses unchanged JS source under the TypeScript parse mode.

The immutable baseline executable compiled each input on a 2 MiB worker and saved
complete WAT/Wasm bytes or the first diagnostic under `frozen/`. No performance
measurement was inferred from this freeze. Smoke runs exercise the accepted and
rejected report paths; their timings are explicitly excluded from evidence.

## Builds

Both binaries must use the identical `probe.rs`, release profile, Rust toolchain,
target, locked dependency versions/features, and build flags. The explicit profile
is opt-level 3, debug=false, strip=false, LTO=false, codegen-units=16,
panic=unwind, incremental=false, overflow-checks=false. The standalone baseline
lock began with the repository lock. Every selected dependency name, version,
source and checksum was compared with that lock; the new harness root is the only
new package. Future builds use `--locked --offline`.

For the final candidate, create a separate manifest with the same package/bin and
profile sections, changing only the compiler path to a clean worktree at the
exact final source tree. Copy the baseline harness lock. Clean the compiler and
harness packages in the separate `target-cost` directory before building the
candidate; keep the immutable baseline executable outside that target. Record the
candidate Git commit/tree, binary hash, lock and harness hashes, command, flags,
toolchain, and dependency graph before measuring. No production source is edited.

## Execution

Wait for other compiler builds, tests, and artifact parity probes to finish. Run
`run_pairs.py` once, serially, with baseline and final candidate executables,
`manifest.tsv`, `frozen/`, and a fresh output directory. Defaults are seven
independent process pairs per case, five timed batches per process, five compiler
calls per batch, and three warmup calls. Every case therefore has seven independent
process-level medians per revision. Case order is deterministically shuffled per
repetition using seed `20261103 + repetition`; revision order uses
`(repetition + original_manifest_index) % 2`, with baseline first when zero.
Each workload therefore has a four/three split over seven repetitions, independent
of its shuffled position. The runner records the actual order. No processes overlap.

The runner refuses to start or continue when it detects an active Rust build or
known compiler/runtime test. Coordinating with other agents remains necessary:
process-name checks cannot prove exclusive host use. Exact first-output validation
is required against frozen files, and every warmed/timed result must match that
first output. A failure stops the run and preserves raw partial observations;
partial results are not reported as a complete experiment.

First-call, batch-call, and release times remain distinct. Process-wall time is
recorded separately and includes startup/I/O. Memory checkpoints cover before the
first compile, after first compile, after frozen-file validation, after warmup,
and after all batches. The report retains actual Linux VmRSS/VmHWM in bytes.

## Reporting limits

Keep all raw samples and paired ratios, per-process medians, per-workload ranges,
binary and workload hashes, host/CPU/cgroup settings, process affinity, and load
observations. Whole-process VmHWM includes the harness, thread, compiler, allocator,
retained outputs, and frozen-output comparison buffers; it does not isolate the
compiler's heap or a single call. First-call observations are not cold-cache
measurements. Fresh processes do not reset OS page caches or stabilize CPU speed.

This is one shared Linux host and a selected bounded workload. No cross-platform
result, universal throughput estimate, timing threshold, or statistical performance
equivalence is established. Small timings or RSS changes may reflect scheduling,
allocator variation, page granularity, or background activity. Any observed
regression should be investigated as a concrete workload result before attributing
it to a compiler stage or making a general claim.
