# Comparing numeric cost reports

Save two reports from the [numeric probe](numeric-benchmark.md), then run:

```sh
node tools/bench/compare.mjs baseline.json candidate.json
```

The command reads existing JSON files and prints one comparison to stdout. It
never runs the compiler, evaluates source, modifies the reports or creates build
artifacts. Paths resolve from the caller's directory. Invalid arguments, malformed
JSON, incomplete reports or incompatible conditions exit nonzero without a report.
The importable `compareReports(baseline, candidate)` uses the same validation.

Only schema-v1 `numeric-gcd` reports are accepted. Workload cases/checksum,
iterations, warmup, sample count, source/harness hashes, cache policy and recorded
Node/V8/OS/architecture/CPU/runtime flags must agree. Missing metadata is not treated
as matching metadata. There is deliberately no force/bypass option. Compiler path,
version, hash and build label may differ, as may emitted Wasm hash and size; both
identities are retained. This is for before/after compiler comparisons with the same
workload and measurement environment, not cross-machine or cross-harness rankings.

Raw JS and Wasm samples remain in input order. Min/median/max are recomputed and
must match the saved summaries; sample counts and finite nonnegative timings are
validated. Timing differences use candidate minus baseline. Ratios use candidate
divided by baseline: below one means a smaller observed duration, not a certified
speedup. A zero denominator or unrepresentable ratio yields explicit `null`, not an
infinite or silently dropped JSON number. Artifact byte differences are separate.

Build API time and first-startup observations remain individual measurements;
only warmed batches have repeated samples. The result does not combine unrelated
phases, calculate significance, select a winner or impose a performance gate.
Compare the JS control as well as the Wasm measurements, and retain multiple whole
runs to assess noise. Matching metadata does not prove identical machine load,
thermal/cache state or JIT tiering, and equal `cpuModel: null` means both models
were unavailable. The reader validates recorded claims, not their authenticity.
It reads local reports in full and has no file-size or filesystem deadline policy.

Run `node --test tests/build/benchmark-comparison.test.mjs` for validation and CLI
tests. The existing native benchmark integration also passes an actual emitted
report through comparison, without asserting a timing threshold. Both are part of
the canonical checks. No benchmark measurements are uploaded or published.
