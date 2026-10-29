# Strict same-process Markdown/WIT receipt comparison (P1)

The [Markdown receipt capture](markdown-capture.md) identifies actual guest and
native executable bytes, but **two independent saved receipts do not establish
same-machine execution**. A matching CPU model, OS and architecture is not a
machine identifier. The build label is caller-declared; observed rustc is not a
verified compiler invocation. Consequently, this tool does **not** accept arbitrary
pairs of existing capture JSON files or compare different builds.

Instead, produce a saved **paired** receipt by running two independent native
cost-probe processes sequentially under one Node comparison process. After the
native/Wasm examples have been built, from the repository root:

```sh
node tools/bench/compare-markdown.mjs --capture \
  target/debug/examples/markdown_cost \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm \
  --workload unicode --samples 5 --iterations 20 --warmup 5 \
  --build-label debug > paired-markdown.json

node tools/bench/compare-markdown.mjs paired-markdown.json > comparison.json
```

On Windows use the native runner's .exe suffix. The second command can also read
from stdin with `-`; both outputs are JSON on stdout only. Saving files is an
explicit caller-controlled redirect. The reader accepts one paired file of at
most 1 MiB and runs no benchmark, source, executable, build or network operation.

## Compatibility gate

The `--capture` path calls the **unchanged** existing capture wrapper twice, with
the same guest, executable and bounded options. It preserves both nested complete
capture receipts, supplies a random session identifier, and binds their serialized
contents by SHA-256 to catch accidental edits. Before a pair is emitted, the
reader checks both receipts and refuses comparison unless all conditions hold:

- Both are valid capture-v1/native-v1 Markdown reports with bounded workload,
  sample configuration, nonnegative timings, coherent min/median/max, valid UTF-8
  source, and the existing granted/denied backing-read and cleanup counts.
- Each has clean **tracked** Git state, explicit non-default build label,
  nonempty CPU model and observed rustc, Git revision, actual guest/native
  executable SHA-256 and byte size, and the complete **versioned** harness
  hash set (five historical files, or six including the Rust memory
  collector).
- Both have the same Git revision, binary/harness identities, build label,
  workload and fixture source/outline bytes, component length, native Wasmtime
  declaration, fuel policy, CPU model/count, OS/release/architecture, Node/V8,
  NODE_OPTIONS and observed rustc. Different machines are not admitted based
  solely on a common architecture or CPU model.
- The pair declares the tool's sequential same-Node-process collection method;
  both original receipt digests match their binding and timestamps are ordered.
  No force, ignore-host or cross-build override exists.

The comparison retains input-order samples for instantiation, granted and denied
WIT calls; it presents median differences and the separate single-observation
embedding/compilation differences as **candidate minus baseline milliseconds**.
There are no ratios, winner selections, noise significance tests, performance
thresholds or benchmark rankings. Zero timings are valid.

## Evidence limits

The same-process collection mechanism establishes a much tighter operational
relationship than comparing two unrelated saved receipts, but the JSON session
identifier and digest are **not signatures or hardware attestations**. Someone
who deliberately fabricates an entirely new self-consistent file can fabricate
the binding too. The reader validates recorded claims and accidental tampering,
not provenance authenticity. The actual host, binary bytes and Rust compiler are
not re-inspected when reading an old saved pair.

Requiring **the same guest and native executable hashes** avoids comparing
unverified different builds; it does not verify how either binary was compiled.
The build label and installed rustc observation remain unverified claims.
Untracked files, compiler arguments, optimization flags, environment variables
other than NODE_OPTIONS, virtual machine migration, process affinity, clocks,
power state, Wasmtime/OS caches and concurrent machine load are not controlled.
Do not interpret differences as a build regression, speedup, standalone host-call
cost, production behavior, or cross-host portability ranking.

The canonical scoped CI builds and runs the real Markdown guest/native runner,
then executes one two-capture pair and reads it back through this compatibility
gate on Linux, Windows and macOS. Its timing samples are *not* retained or
compared between CI hosts. The separate Node tests use synthetic receipts only
to check acceptance/rejection rules, not to claim execution evidence. No runner
matrix, workflow, dependency, artifact publishing, deployment or release changes
are introduced. Process memory, frontend-only cost, measured alternative builds
and broader application workloads remain unfinished P1 work.

## Recorded Rust-process memory integrity

New Markdown cost reports may include a `processMemory` envelope with five
validated Linux RSS/peak-RSS or Windows working-set/peak-working-set
checkpoints. Earlier Windows receipts explicitly recorded unsupported/null,
and that historical absence remains valid. macOS still records
unsupported/null observations. The capture and same-process comparator now
**validate a claimed memory envelope** against the saved native OS and the
complete ordered lifecycle, including units, snapshot keys, integer bounds,
RSS no greater than peak and nondecreasing process peak. The comparator uses
the recorded host instead of assuming the machine reading the JSON is the
machine that produced it.

Historical `schemaVersion: 1` captures lack memory observations. Those
remain readable and comparable; absence is **not** represented as a zero
observation or an implied memory pass. If a receipt *does* claim memory
values, corrupt or contradictory data fail even when someone recomputes the
pair's ordinary SHA-256 digest binding. This still does **not** establish a
signed provenance attestation, genuine physical measurements, guest-only
memory attribution, cross-host comparability or performance rankings.

These are ordinary pure Node regression assertions in the already canonical
build/capture and comparison suites. No new Actions workflow, runner, Cargo
dependency or independent status check is introduced.

## Cross-generation capture compatibility

Saved capture-v1 receipts without an explicit `provenance.harnessVersion`
retain their original five-file identity and remain readable. Newly
generated receipts use `harnessVersion: 2` and require six complete hashes,
including the transitive Rust module that collects Markdown host-process
memory; they also require a claimed, independently validated memory
envelope. The comparator rejects partial identity sets, unsupported
versions and mixed legacy/new pairs even when their digest bindings are
recomputed. Existing single-generation pairs retain the old envelope
and interpretation without imposing artificial memory-comparison metrics.

## Comparing recorded whole-process memory without attribution

The strict paired comparison now emits a separate `processMemory` summary.
For two validated Linux **or current Windows** receipts containing complete
five-phase native process-memory observations, it preserves each phase's
baseline and candidate byte count alongside their **signed
candidate-minus-baseline difference**, together with the actual
`linux-proc-status` or `windows-psapi-working-set` source label.
Windows `rssBytes` means current working-set bytes, not Linux RSS.
The reader refuses mixed source types even when receipts are rebound.
Both inputs were collected in **separate native processes** by one Node
capture session. Even equal binaries and OS metadata cannot isolate guest
allocation, memory fragmentation, cache lifetime, page residency or CPU load.
A process-lifetime high-water mark is not a per-component allocation peak.
Do **not** rank implementations or interpret positive/negative deltas as
regressions without additional controlled experiments.

For historical captures where either process-memory envelope was omitted,
the comparison explicitly reports `availability: not-comparable` and whether
each side recorded evidence. The absence is **not** zero memory. Old
Windows captures and all currently unsupported macOS captures remain
`unsupported-on-this-host`, without any fabricated phase values. Saved host identity, not the OS used
to read the JSON, determines whether a reported sample is valid. The
original timings, signed timing deltas, intact receipt bindings and
historical pair compatibility remain unchanged.
