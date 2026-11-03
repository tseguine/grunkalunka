# Source-lowering refactor: implementation and validation checkpoint

Date: 2026-11-03

## Result and scope

The in-review continuation series completes the planned effectful source-lowering
migration. Statements and declaration initializers, evaluator resumptions, local
and runtime factories, disposable definition checks, and real lifted closure
bodies now use the shared source cursor. The final cleanup deletes 22 synchronous
compatibility driver methods and their Region lease. There is one production
source entry chain and one evaluator host chain. Module lowering enters
`sequence`, then `control_statement_iterative` and `run_source_cursor`.
Expression tasks enter `advance_owned_expression`, which calls `run_evaluation`.

Typed requests and completions return to the source loop. Definition checking
publishes checked metadata and original source costs while discarding scratch IR;
real materialization publishes nested functions followed by its enclosing
function. Both restore the parent Lowering before applying parent continuations
or unwinding older parent frames.

This closes the implementation portion of L3/L4 and the corresponding L5 driver
retirement in [engine steering](javascript-engine-steering.md). L6 remains optional
until a source semantic extension needs it. The separate signature/Promise query
component, parser and semantic implementation, automatic destruction, async IR
transforms, emission, and runtime scheduling retain their own contracts. This
checkpoint does not close the broader cross-phase invariant in
[#281](https://github.com/tseguine/grunkalunka/issues/281).

## Exact revisions

| Role | Revision / source identity |
| --- | --- |
| Starting master | `926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c`; tree `30bafc7a28c98aea808c7d9a6475f432def28084` |
| Final production source | [#466](https://github.com/tseguine/grunkalunka/pull/466), commit `67400f91882124e021f4bf30157930bb91acba31`; tree `9ea19ddd1ff1dbd0d39c46216d13a22747130735` |
| Independently tested retirement checkout | `4bef43c35814d7cb747254ad4d4fbac0dc9715dd`, the same complete source tree |
| Frozen optional helper-state oracle | `511db64584a50b7e1c074014cf6f337aa5972dc3`; the original compatibility driver remained unchanged when the 21 complete states were captured |

Published API commits have different commit metadata from local validation
commits. Each publication checks the complete Git tree against the validated
tree before creating the branch and PR. Later evidence/documentation commits do
not change the compiler or runtime production source measured here.

## Preserved contracts

| Boundary | Preserved behavior |
| --- | --- |
| Declaration sequence | Charge and select one declarator at a time; publish it before selecting the next. Zero-argument metadata completion queues `AdvanceDeclaration`. |
| Const metadata | The metadata consumer adds no ordinary function-value or function-result admission; argument-owned guards remain unchanged. Arguments complete in caller scope; caller bindings restore before metadata publication or runtime materialization. |
| Local function results | Acquire the original result admission before definition checking; retain the independent original callee snapshot across metadata fallthrough. |
| Runtime function results | Admit the runtime result before validation; construct and snapshot the callee before arguments; release the callee's ordinary function-value charge before argument execution. |
| Definition checking | Validate unused definitions; retain source costs, capture pruning and intentional shared signature interning; discard scratch slots, values, Regions and lifted bodies. |
| Real materialization | Initial lifting admission precedes checking; expansion precedes cloning; preserve parameter/capture initialization and nested-function publication before the final capacity check. |
| Errors | Preserve the first diagnostic and source span; reverse unwind restores child owners before parent resources and lexical checkpoints. |
| Evaluation | Preserve original Oxc identities, typed Regions, modes, source charging, snapshots, effects, and normal/abrupt completion behavior. |

The existing source, expansion, block/function-result, function-value, lifting
and module-capacity limits are unchanged. No larger native thread stack, broader
source rejection, relaxed output comparison, or reduced CI selection was used.

## Verification on the final production source

| Check | Result |
| --- | --- |
| Compiler package | 589 unit tests and 10 integration tests passed |
| Compiler all-target Clippy / formatting | Strict `-D warnings` and `cargo fmt --all --check` passed |
| Frozen JS/TS differential compilation | All 7,487 cases across 34 matrices match: 6,033 admitted and 1,454 rejected. The 13,554 compared files comprise 13,520 artifact/diagnostic files and 34 per-matrix result summaries; no changed, missing or extra files and no expectation failures |
| CONTROL characterization | All 106 private IR/WAT/Wasm/diagnostic records match in original order |
| Full closure runtime suite | All 136 tests passed |
| Full async-suspension suite | All 426 tests passed; none ignored or filtered |
| Module callback and managed closure input suites | All 9 tests in each suite passed |
| Managed function value/storage suites | All 9 value tests and the storage test passed |
| CI impact regression suite and plan | All 106 regression tests passed; the generated dependency plan was inspected |

The differential harness runs each compilation on an explicit 2 MiB worker.
Accepted cases compare WAT and Wasm bytes; rejected cases compare the complete
first diagnostic. CONTROL records additionally compare private IR. Normalizing
the harness's `expected=observe` metadata requires an exact match to the already
frozen outcome and does not change artifacts or diagnostics.

The 34 matrices have their own historical oracle provenance. They were frozen
at different points in the continuation series; the combined count is not a
claim that every supplemental matrix was recorded directly on starting master.
The portable parity supplement preserves each source, original expectation,
frozen actual result, artifact hashes and available producer provenance. Its
strict replay checks frozen actual outcomes even where discovery originally used
`expected=observe`. It does not generate new expectations from the final compiler.

The retained runtime fixtures use their existing native JavaScript, Node-Wasm,
and Wasmtime oracles, including identity, captures, left-to-right effects, signed
zero, NaN, Promise settlements and job ordering where the suite provides them.
The compiler-only cost harness below does not supply semantic runtime evidence.

The maintained suite commands, run from a checkout of the final production
source with the repository's normal toolchain/dependencies available, are:

```sh
cargo test -p grunkalunka-compiler --locked --offline
cargo clippy -p grunkalunka-compiler --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
cargo test -p grunkalunka-runtime --locked --offline \
  --test runtime-closures --test async-suspension \
  --test module-callback-inputs --test managed-closure-inputs \
  --test managed-function-values --test managed-function-storage
node --test ci/impact/*.test.mjs
node ci/impact/plan.mjs origin/master
```

The final runtime groups were executed in two independent, nonoverlapping runs;
the combined invocation above selects the same complete suites. The CONTROL
records come from `evaluator_tests::evaluator_control_characterization` with
`--nocapture`, compared byte-for-byte in their original order against the frozen
baseline output. `--offline` assumes dependencies are already cached.

### Sensitive tests and owned state

New route tests fail on their unchanged predecessors, then pass with the
continuations. The exercised checking and materialization forms reduce nested
source-cursor entry to one. The evaluator-host resumption probe covers 30 JS/TS
zero-argument factory sources: concise cases change from two native host entries
at once to one, while block/runtime cases remain at one. The predecessor's
nesting was constant over the tested sibling counts; those observations do not
show proportional growth in that family.

All 21 optional helper-state records preserve their complete original sources,
diagnostics, resource counters, source costs, expansion/mode state, slots, SSA
types, Region identity/spans/operations and signature identities. They are
checked through the real definition-checking root without a synthetic statement
or function-value charge. Actual optional chaining remains rejected. The retired
lease test is replaced by a test of the real owned evaluator on completion and a
diagnostic. Test builds also reject nested native evaluator runners.

## Call-graph audit

The audit parses production Rust modules with `syn`, records function and
closure bodies, follows resolved receivers and callback arguments, and computes
strongly connected components (SCCs). Nodes include closure bodies, so a component
size is not a count of named Rust methods. The same frozen parser and analyzer
produce these inventories:

| Source | Production files | Functions and closures | Cyclic lowering components |
| --- | ---: | ---: | --- |
| Starting master `926a4aa` | 56 | 1,120 | 48-node effectful compatibility component; 25-node fact-query component |
| Composed source at #464, tree `e8ef039` | 58 | 1,213 | 35-node compatibility component; 7-node factory/declaration component; 25-node fact-query component |
| Final production tree `9ea19ddd` | 58 | 1,176 | 7-node factory/declaration component; 25-node fact-query component |

The original effectful compatibility component is gone. The two remaining
syntactic components require different source arguments:

- **Factory/declaration scheduling:** the 7-node cycle includes a possible
  `Factory` match after `publish_declaration_closure`. Its successful result is
  exhaustively one of `Complete`, `Materialize` or `CheckClosure`, and
  `resume_returned_factory_check` forwards that exact result. None can enter the
  distinct `Factory` arm. `Complete` queues `AdvanceDeclaration`, rather than
  directly restarting declaration selection. This excludes that specific native
  backedge. The explicit variant model becomes unknown when the exhaustive
  producer premise is removed; it does not infer impossibility from missing
  observations.
- **Signature/Promise fact queries:** the 25-node component contains 11 named
  functions and 14 closures. All 11 function bodies have identical normalized
  tokens to starting master. A reviewed certificate contracts the forwarding
  wrappers into nine semantic helpers and records 17 dependencies. Every native
  cycle increases the guarded `calls` argument; the dependencies that preserve it
  form an acyclic graph. A finite overapproximation over 81 helper/count states
  is also acyclic. The concise-arrow loop's saved frames do not lower `calls`
  below its entry value. This establishes bounded helper nesting under the
  reviewed premises, not a byte-level native-stack bound.

The final inventory records 10,048 syntactic calls and 3,900 graph edges. It keeps
7,132 unresolved or conservatively resolved call-site records, including external
library calls; those records are not certified absent. There are no unresolved
source modules or unparsed expression macros in this inventory. Independent
review of unresolved records for parsed free-function names and the relevant
Lowering/evaluator receiver types found no omitted internal source entry in those
subsets. This is a scoped review, not complete Rust name/type resolution.

Sensitivity fixtures cover direct recursion, typed callback flow, calls inside
macros, parent imports, inherited imported aliases, function-local enum fields
containing boxed receivers, and unknown boxed-receiver fallback. They also check
that comments, strings and exact `cfg(test)` items do not create production edges.
The inherited-alias and local-enum cases distinguish the corrected resolver from
its predecessors.

The graph is a source-level investigation tool. It does not establish whole-
compiler native-byte usage or resolve every external trait, generated call,
automatic drop or dependency implementation. Its conclusions are combined with
source-specific value-flow review and the exercised 2 MiB boundaries.

## Compiler time and process memory

### Fixed primary experiment

All 840 fresh-process observations completed, and every result matched the frozen
output. Across the 60 workload-specific median paired candidate/baseline ratios,
the median is **1.011** and the unweighted geometric mean is **0.990**. These are
descriptive aggregates over the selected cases, not a production-workload mix or
a performance-equivalence test. Individual case medians range from **0.611 to
1.579**.

The largest apparent increases motivated the focused confirmation because the
corresponding language pair and individual processes disagree:

| Primary observation | Baseline process median | Candidate process median | Median paired ratio | Individual paired ratio range |
| --- | ---: | ---: | ---: | ---: |
| Charged-result limit rejection, JS | 196.4 us | 284.9 us | 1.579 | 0.693–2.067 |
| Same source under TS parsing | 199.0 us | 192.7 us | 1.017 | 0.398–1.237 |
| Lifted-capacity rejection, TS | 7.481 ms | 9.453 ms | 1.251 | 0.925–2.004 |
| Same source under JS parsing | 7.208 ms | 7.474 ms | 1.048 | 0.988–1.089 |

The process medians and paired-ratio medians are different statistics; their
quotients need not agree. Differences between each revision's median whole-
process peak RSS range from **-32 to +252 KiB** across the 60 cases, with a median
difference of **+118 KiB**.
Every primary per-workload ratio and its underlying process observations is
retained, including apparent improvements and increases.

### Focused confirmation

A separately declared follow-up selected 16 cases: both languages for the charged
result boundary, admitted/over-capacity lifting, zero-argument declaration
boundary, affine controls and nested mutable closures. It used the same immutable
Rust executables, ten pairs per case, twelve batches of 32 calls, eight warmups
and CPU0 affinity, with five baseline-first and five candidate-first pairs per
case. All **320 processes** completed and every output matched. The first attempted
invocation requested 64 iterations, exceeding the immutable probe's limit; it
stopped before compilation or a timing sample. Both that failure and the corrected
pre-measurement plan are retained.

| Focused workload | JS median paired ratio | TS median paired ratio |
| --- | ---: | ---: |
| Affine control | 1.068 | 1.142 |
| Charged result, admitted depth 7 | 1.161 | 0.948 |
| Charged result, rejected depth 8 | 0.937 | 1.074 |
| Lifted capacity, admitted 256 | 1.008 | 0.981 |
| Lifted capacity, rejected overflow | 0.977 | 1.072 |
| Nested mutable closures | 0.907 | 0.985 |
| Zero-argument declarations, admitted 127 | 1.042 | 0.975 |
| Zero-argument declarations, rejected 128 | 1.029 | 1.029 |

The largest primary percentages did not retain their magnitudes. The charged JS
rejection changes from 1.579 to 0.937; the TS lifted-capacity rejection changes
from 1.251 to 1.072. The latter remains a possible modest increase: its baseline
and candidate revision medians are 7.626 and 8.561 ms, with paired ratios ranging
from 0.882 to 1.671. Its matched JS case does not show the same signal. Even the
affine controls retain individual ratios from 0.226 to 3.275. These observations
leave substantial shared-host uncertainty; they do not establish performance
equivalence, a general speedup or the absence of smaller regressions.

Source review found no added repeated semantic checking pass or new asymptotic
traversal in the inspected outlier paths. Factory continuations and child owners
do add allocations and parent/child switching. Cloning the accumulated lifted
function pool already occurred in the baseline. These identify plausible costs
of explicit ownership without attributing the measured percentages to a compiler
stage. Further optimization would need stronger workload/host evidence; this
checkpoint retains the observed tradeoff without installing a timing threshold.

The two experiments retain separate results. Their different batch sizes change
the number of simultaneously retained outputs, so their process-memory levels
must not be pooled.

### Measurement boundaries

The fixed workload contains 60 JS/TS cases: 38 accepted and 22 rejected. Both
executables use the same compiler-only release harness, toolchain, locked
dependencies and explicit release profile, on an explicit 2 MiB worker. Each
timed result must match the independently frozen artifact bytes or diagnostic.

Measurements ran serially after competing builds/tests finished. In the primary
experiment, seven fresh-process pairs per case alternate baseline/candidate order; each process performs
three warmups and five batches of five complete compiler calls. Artifact release
is timed separately. The report retains first-call, batch and release timing as
distinct quantities.

Linux process VmRSS/VmHWM includes the harness, worker, compiler, allocator,
retained outputs and frozen-output comparison buffers. It is not isolated
compiler heap usage. Fresh processes do not reset OS caches or fix CPU frequency.
The results describe this selected workload on one shared Linux host; they do not
establish universal or cross-platform performance equivalence.

## CI and reproduction

GitHub snapshot at **2026-11-03 11:43:38 UTC**, covering production PRs through
#466: master remains `926a4aa`, all 31 ordinary PR heads are unchanged and open,
and their discussion/review timelines contain no comments. This is a dated
snapshot, not a prediction of later checks or merges.

| PRs | Native stack membership | Hosted checks at the snapshot |
| --- | --- | --- |
| #435–439 and #441–459, 24 PRs | Stack #440, ID 2963533, size 24 | Five completed successful checks each, including Linux, Windows and actual native Apple Silicon fallback |
| #460–466, 7 PRs | Not enrolled; raw stack metadata is null | No head check runs yet |

The remaining branches need native stack registration before their expected
hosted feedback. Ordinary dependent PR creation does not establish registration
or a hosted pass. The follow-on evidence and documentation PRs are outside this
production snapshot; their actual head status must be checked independently.

The existing platform rehearsal has a separately reproduced failure on unchanged
starting master: its expected-command assertion omits existing dependency
selections and fails before selected builds/tests execute. This checkpoint does
not claim that rehearsal passed or weaken its check. The impact regression
suite, actual generated dependency plan and hosted workflow results are reported
separately.

The frozen manual-audit bundle is at
[`scripts/audits/lowering-2026-11-03`](../scripts/audits/lowering-2026-11-03/README.md).
It preserves the parser, graph analysis, sensitivity fixture, cost harness,
portable fixed workloads, differential replay, commands, hashes and compact
results. It introduces no workspace dependency, production feature, automatic
benchmark or CI job. Its standalone Cargo manifests and locks trigger the existing
global-build-input fallback: the actual staged-path plan selects full CI, with 43
logical test nodes and all 34 canonical commands. Canonical coverage and source-
dependency validation pass; no owner exclusion or reduced selection was added.
