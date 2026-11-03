# Frozen lowering investigation, 2026-11-03

This directory preserves the manual investigation tools and compact evidence for
the source-lowering continuation refactor. It is **not** a general Rust correctness
checker, a maintained benchmark, or an automatic performance gate. Nothing here
adds a workspace dependency, canonical check, or GitHub Actions job.

The compiler baseline is `926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c`, tree
`30bafc7a28c98aea808c7d9a6475f432def28084`. The published final source is
`67400f91882124e021f4bf30157930bb91acba31` in PR #466, tree
`9ea19ddd1ff1dbd0d39c46216d13a22747130735`. Independent builds used local commit
`4bef43c35814d7cb747254ad4d4fbac0dc9715dd`; the published tree was separately read
from GitHub and is identical. Reproduction should use the published commit.

The original model ledgers cite local composed commit
`16411448823b77252875f051796f005b0331fab2`. Its tree
`e8ef039e517b157f820b4694c3d0a99b1e628891` is also published in PR #464 at
`a8db7fcf790d7a58a4f2a31667fad08c6b990688`. Use that reachable revision to inspect
the original source positions. The frozen model/certificate bytes are unchanged;
`results/source-revisions.json` records the mapping separately.

## What the inventory establishes

`parser/` is a standalone `syn` parser with its own manifest and lock. It discovers
module files, ordinary function bodies, lexical closures, source positions, local
enum/struct fields, imports and call-bearing expression macros. `analyze.py`
propagates known callback targets and selected receiver types, then calculates
strongly connected components with an iterative graph algorithm. Normalized body
hashes ignore formatting and comments while retaining Rust tokens.

On the inspected source, the baseline has a 48-node component containing effectful
compatibility reentry and a 25-node fact-query component. The final source has two
syntactic components intersecting lowering: a seven-node factory/declaration
component and the unchanged 25-node fact-query component. The intermediate
composition still had a 35-node compatibility component before its deletion.
These counts include lexical closure nodes; they are not counts of recursive
runtime calls or independent compiler operations.

Two manual refinements are essential to interpret the remaining components:

* The factory/declaration component's apparent back edge forwards the result of
  `publish_declaration_closure` unchanged into an initializer match. Its exhaustive
  successful results are `Complete`, `Materialize` and `CheckClosure`; none selects
  the direct `Factory` arm. `Complete` enqueues `AdvanceDeclaration`. The three
  producer/forwarder/consumer bodies match the independently reviewed intermediate
  snapshot exactly. See `factory-cycle-ledger.md`, the two gloss models and the
  portable finite consequence check in `check_factory_variants.py`. Removing the
  exhaustive-output premise gives **unknown**, not a proof by absence.
* The 25-node fact-query component consists of 11 named helpers and 14 lexical
  closures. All 11 named bodies have identical normalized tokens in baseline and
  final source. The reviewed `calls` rank never decreases below a native
  invocation's entry value, and every cycle increases it. The zero-increment graph
  is acyclic; guarded entries bound `calls` at eight. The concise-chain entry gate
  and saved-frame reconstruction require the explicit reasoning recorded in
  `query-ranking-certificate.json`. `check_query_rank.py` checks the certificate's
  finite consequences, including an 81-state overapproximation. It does not prove
  the source premises, native stack bytes, external compiler stages or total work.

The final parser report contains 1,176 functions/closures and 3,900 modeled edges.
It retains 7,132 unresolved or conservative call sites, including standard
library and external operations; 326 edges use method-name fallback. There are no
unresolved modules or unparsed expression macros in this snapshot. These numbers
do **not** establish a closed-world graph. The tool does not implement Rust type
checking, macro expansion, trait resolution or complete callable flow through
heap fields, returned closures and arbitrary patterns. Unknown external callbacks
are conservatively treated as invoked. Conditional compilation other than exact
`cfg(test)` is retained. Manual inspection of scoped unresolved entries and the
sensitivity fixture complement the graph; neither makes it a whole-program proof.

`results/final-scc-receipt.json` records the exact source mapping, tool/input/output
hashes, component membership, body preservation and checked model results. Raw
multi-megabyte inventories/graphs are intentionally regenerated rather than stored.

## Reproduce the inventory

The original toolchain was Rust 1.99.1 on Linux x86-64. Use Python 3.11 or newer.
The fixed Cargo locks require the corresponding cached crates for `--offline`;
omit that option to fetch those same locked versions on a new machine. Start at
the repository root, with both source commits available locally:

```sh
export RUSTUP_TOOLCHAIN=1.99.1
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER CARGO_BUILD_TARGET
audit_repo=$(pwd)
audit_bundle="$audit_repo/scripts/audits/lowering-2026-11-03"
audit_work=$(mktemp -d "${TMPDIR:-/tmp}/grunkalunka-lowering-audit.XXXXXX")
git worktree add --detach "$audit_work/baseline" 926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c
git worktree add --detach "$audit_work/final" 67400f91882124e021f4bf30157930bb91acba31
CARGO_TARGET_DIR="$audit_work/parser-target" cargo build --manifest-path "$audit_bundle/parser/Cargo.toml" --locked --offline
"$audit_work/parser-target/debug/lowering-call-inventory" "$audit_work/baseline/crates/grunkalunka-compiler/src" > "$audit_work/baseline-inventory.json"
"$audit_work/parser-target/debug/lowering-call-inventory" "$audit_work/final/crates/grunkalunka-compiler/src" > "$audit_work/final-inventory.json"
python3 "$audit_bundle/analyze.py" "$audit_work/baseline-inventory.json" "$audit_work/baseline-graph.json"
python3 "$audit_bundle/analyze.py" "$audit_work/final-inventory.json" "$audit_work/final-graph.json"
"$audit_work/parser-target/debug/lowering-call-inventory" "$audit_bundle/fixture" > "$audit_work/fixture-inventory.json"
python3 "$audit_bundle/analyze.py" "$audit_work/fixture-inventory.json" "$audit_work/fixture-graph.json"
python3 "$audit_bundle/check_fixture.py" "$audit_work/fixture-graph.json"
python3 "$audit_bundle/check_query_rank.py" "$audit_bundle/query-ranking-certificate.json"
python3 "$audit_bundle/check_factory_variants.py"
```

Absolute source locations occur in regenerated graphs, so their file hashes change
when worktree paths change. Compare component membership, normalized body hashes,
classification and resolution counts; the recorded hashes identify the original
investigation files. The fixture distinguishes direct recursion, callback flow,
macro calls, local-enum boxed receivers, unresolved boxed fallback and imported
aliases through a parent glob. Comments, strings and exact test-only bodies are
negative controls. The imported-alias fixture was checked to fail with the prior
incomplete glob resolver.

The gloss receipts also record the original evaluator revision. That evaluator
is not bundled or required: the portable finite checker independently reproduces
the relevant variant consequences from the documented manual premises. Neither
checker reads Rust to establish exhaustive returns automatically.

## Compiler cost observations

`cost/probe.rs` is the exact measured Rust source. It links only the compiler and
uses the same release profile and locked dependency versions for both revisions.
It measures complete public compile calls on an explicit **2 MiB** worker. Each
first output must equal frozen baseline WAT/Wasm bytes or the exact first error;
every warmup and timed result must equal that first output. Validation and artifact
release are outside the compile timer; release has its own timer. Linux VmRSS and
VmHWM describe the whole process, including the harness, allocator, retained
outcomes and validation buffers, rather than isolated compiler allocations.

The 60 fixed sources cover 30 JS/TS families: 38 admitted and 22 rejected. The
primary run used seven independent process pairs per case, five batches of five
calls and three warmups: 840 fresh processes. Workload order was deterministically
shuffled, and revision order was balanced four/three per workload. All outputs
matched. The median of the 60 per-workload paired ratios was 1.011; the unweighted
geometric mean was 0.990. Individual medians ranged from 0.611 to 1.579, with large
pair-to-pair variation. Differences between revision medians of whole-process peak RSS ranged from -32 to
+252 KiB. These descriptive aggregates establish neither equivalence nor a
general speedup.

The largest primary observations prompted a separately predeclared 16-case
confirmation using matched JS/TS, accepted/rejected boundaries and simple controls.
It retains all primary observations. See `cost/results/focused-plan.json` for the
fixed selection and protocol. The first attempted invocation exceeded the immutable
probe's 32-iteration limit and stopped before any compilation or timing sample;
its failure receipt is retained. The corrected protocol uses ten balanced pairs,
12 batches of 32 calls, eight warmups and explicit CPU0 affinity. This increases
retained outcomes and therefore must not be pooled with the primary RSS data.
All 320 confirmation processes matched their outputs. The primary charged-result
limit observation changed from 1.579 to 0.937 for JS; the corresponding TS result
was 1.074. Lifted-capacity rejection changed from 1.251 to 1.072 for TS, while JS
was 0.977. The zero-argument TS accepted case changed from 1.182 to 0.975, and its
JS rejection changed from 1.203 to 1.029. All 16 focused medians range from 0.907
to 1.161. The original large magnitudes did not persist, but even affine controls
retain large individual-pair variation. A modest cost increase for TS lifted-
capacity rejection remains possible; this host does not establish equivalence.
`cost/results/focused-comparison.json` reports every selected workload, including
apparent improvements and increases, without pooling the two protocols.

All 1,160 process observations are retained compactly under
`cost/results/observations/`: every first-call, compile-batch, release-batch and
memory checkpoint value, with revision/order metadata and a schema/hash index.
Shared environment and configuration fields live in the separate run metadata.
The three JSONL chunks total 343,219 bytes; no binary or emitted artifact is stored.

`cost/MEASUREMENT.md` gives the full scope and limitations. The source, artifact,
manifest, toolchain, binary, profile and lock hashes remain in the receipts. This
is a selected workload on one shared Linux host; it does not establish native-host
portability, compiler-stage attribution or stable CPU/cache behavior. No threshold
is installed as a CI performance gate. Existing runtime suites provide semantic
execution evidence; this standalone harness does not run Wasmtime goldens.

## Reproduce the cost protocol

Use the same `audit_bundle` and clean worktrees as above. The manifest expansion
wrapper resolves paths relative to the manifest and emits source hashes. It keeps
the measured Rust code byte-for-byte unchanged and works from any caller directory.
Build directories must be new; each has an independent Cargo target. Use the same
toolchain and build flags for both builds; receipts record the actual environment.

```sh
python3 "$audit_bundle/cost/resolve_manifest.py" "$audit_bundle/cost/manifest.tsv" "$audit_work/cost-manifest.tsv"
python3 "$audit_bundle/cost/build_probe.py" "$audit_work/baseline" "$audit_work/baseline-probe" --offline
python3 "$audit_bundle/cost/build_probe.py" "$audit_work/final" "$audit_work/final-probe" --offline
"$audit_work/baseline-probe/target/release/lowering-cost-probe" freeze "$audit_work/cost-manifest.tsv" "$audit_work/frozen"
python3 "$audit_bundle/cost/verify_frozen.py" "$audit_work/frozen" "$audit_bundle/cost/results/baseline-build-receipt.json"
python3 "$audit_bundle/cost/run_pairs-primary.py" "$audit_work/baseline-probe/target/release/lowering-cost-probe" "$audit_work/final-probe/target/release/lowering-cost-probe" "$audit_work/cost-manifest.tsv" "$audit_work/frozen" "$audit_work/primary"
python3 "$audit_bundle/cost/resolve_manifest.py" "$audit_bundle/cost/focused-manifest.tsv" "$audit_work/focused-manifest.tsv"
python3 "$audit_bundle/cost/run_pairs.py" "$audit_work/baseline-probe/target/release/lowering-cost-probe" "$audit_work/final-probe/target/release/lowering-cost-probe" "$audit_work/focused-manifest.tsv" "$audit_work/frozen" "$audit_work/focused" --repetitions 10 --samples 12 --iterations 32 --warmup 8 --cpu 0
```

Wait for builds, tests and parity probes to finish before either timed command.
The runner detects known competing Rust jobs, but that cannot prove exclusive
host access. Affinity requests fail explicitly when unavailable; document any
different CPU and treat it as a new run. Keep all generated raw samples. Rebuilt
binary hashes can vary with toolchain and checkout paths; semantic output hashes,
the frozen source/profile/lock and actual build receipts identify the comparison.

## CI ownership and evidence scope

These are executable investigation tools and source fixtures, not documentation
only. The existing `canonical-scripts` owner covers `scripts/`; no owner exception,
new workspace member or automatic audit job is added. The unchanged planner also
classifies every nested `Cargo.toml` or `Cargo.lock` as a global build input, so
this bundle's standalone manifests/locks produce **full verification**: all 43
logical test nodes and 34 canonical commands. Canonical coverage is 34/34, the
bounded source-dependency audit is valid, and no required independent browser
workflow is missed. The staged-path receipt records this result. No exclusion or
special ownership rule reduces that existing fallback. A planner result describes
expected selection, not a claim that the standalone investigation executes in CI.

The compact runtime/parity receipt in `results/` is separate from the cost protocol:
7,487 fixed JS/TS cases (6,033 accepted and 1,454 rejected) matched across
13,520 artifact/diagnostic files plus 34 summary metadata files, and the independent
full async suite passed 426 tests at the exact final compiler tree. The broader
project report records other suites, hosted CI state and retained platform limits.

The complete 7,487-case input catalog, frozen output hashes and portable replay
are supplied in [parity/README.md](parity/README.md). This complements the 60-case
cost workload; the cost sources alone do not reproduce the full differential suite.

For a read-only forecast using only this bundle's paths against the unchanged
final-source planner, run:

```sh
node "$audit_bundle/check_bundle_impact.mjs" "$audit_work/final" "$audit_bundle"
```

`results/bundle-impact-selection.json` is the pre-integration receipt; the evidence
PR additionally runs the normal planner against its actual complete changed paths.
`results/tool-smoke-receipt.json` records syntax/format/sensitivity and portable
source/output checks. The standalone build wrapper's preparation was checked
without claiming that this copied bundle produced the original measured binaries.
