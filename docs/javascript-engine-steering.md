# JavaScript engine steering: semantic convergence gates

Current-state review: 2026-11-06, master
[624de02](https://github.com/tseguine/grunkalunka/commit/624de02bfdbed407a1d4ff1526f00a26f71f92f9).
The original review and source-lowering checkpoints below retain their dated
evidence; their former pending/open statuses are historical.

**Status: advisory execution steering.** The maintainer asked for this cross-stream review. It records observed implementation, proposed next proof points and existing ownership boundaries. It is not a new language profile, accepted ABI, automatic agent assignment or permission to interrupt unrelated work. [AGENTS.md](../AGENTS.md) remains authoritative for permissions, PR review and merging.

**Current compiler direction (2026-11-12):** [accepted ADR 0022](adr/0022-general-js-ts-lowering-first.md)
prioritizes general, workable lowering of valid JS/TS over new specialized
source-admission proofs. Lowering coverage, standards compliance and optimization
are separate measures. The November 3
[impact-ordered feature train](javascript-feature-train.md) is now historical
execution guidance, not the default ordering for new compiler units. Preserve
coherent in-flight work, the existing owned source cursor/IR/runtime, and the
independent workstream/PR coordination contracts. Dated evidence below remains
valid for its inspected revisions.

## What is currently established

- The maintained Oxc frontend lowers selected JS/TS through verified structured IR to real portable Wasm. Numeric and primitive-return profiles retain their contracts and specialized f64 paths.
- The WasmGC path executes objects, sparse arrays, lossless UTF-16 Strings, aliases, checked helpers, managed captures, returned closures and canonical function identities. Nested bindings/rest, scoped dictionary keys, property presence/enumeration, collection copies and String operations now compose with the application service. Calls through fields still need compatible source-proven signatures. See [managed values](managed-values.md), the [feature train](javascript-feature-train.md) and [accepted GC direction](adr/0019-wasmgc-runtime.md).
- The **async** profile supports represented Promise identity, settlement/adoption, rejection notifications, suspended continuations and selected try/catch/finally/job ordering. Actual managed references remain traced across suspension and related child activations in one generated instance. Managed Promise settlements/reasons, managed await operands, general thenables and public reference results remain outside that contract.
- [Synchronous source errors](synchronous-errors.md) support primitive throws caught in the same function, including cleanup through finally. Cross-call and uncaught synchronous throws and broader reason values remain separate boundaries.
- A1 connects authorized Markdown/WIT, native input, an actual compiled JS/TS scalar service call, Rust-owned action state and readable bitmap repaint. [Live sessions](a1-live-application.md) add scrolling, note selection, per-note reading state, reload and reconnect; [detached packaging](a1-live-package.md) retains portable components. The typed public view proposal in [#483](https://github.com/tseguine/grunkalunka/issues/483) was retired without selecting or implementing that interface.
- The planned source-lowering migration and obsolete-driver retirement are merged. Native-stack limits and small-stack tests remain engineering requirements. [#281](https://github.com/tseguine/grunkalunka/issues/281) was retired as an open-ended audit tracker; its closure does not prove every compiler/runtime path stack-independent.

These are **different bounded execution profiles**, not a claim of general ECMAScript support or an implemented browser/Node environment. Historical observations lower in the roadmap and milestone ledger describe their named revisions, not this state.

## Resolved steering and remaining boundaries

| Concern | Completed or retired work | Remaining contract boundary |
| --- | --- | --- |
| **For-let closure admission** | [#573](https://github.com/tseguine/grunkalunka/pull/573) completes [#334](https://github.com/tseguine/grunkalunka/issues/334)'s two unchanged managed-result/stored-function fixtures after #374's numeric repair | Preserve per-iteration cells and the existing numeric, managed-result and stored-function regressions as new forms are admitted |
| **Semantic runtime path** | [#380](https://github.com/tseguine/grunkalunka/pull/380) implements runtime own-key enumeration and catchable nullish errors in the async profile; [#355](https://github.com/tseguine/grunkalunka/issues/355)'s indefinite remainder was retired | General Get/Set/Call, coercion, arbitrary prototypes/accessors and synchronous error propagation remain feature boundaries; retirement claims no complete-language or performance equivalence |
| **GC-aware suspension and identity** | [#356](https://github.com/tseguine/grunkalunka/issues/356)'s bounded proof is complete through #370/#375/#377, with source oracles and forced collection in suspended related activations | Managed Promise payloads/reasons, public references and identity across independently instantiated host entries remain unsupported; umbrella [#300](https://github.com/tseguine/grunkalunka/issues/300) is retired |
| **First integrated app** | [#52](https://github.com/tseguine/grunkalunka/issues/52) and [#54](https://github.com/tseguine/grunkalunka/issues/54) close their bounded integration/pilot objectives; [#549](https://github.com/tseguine/grunkalunka/issues/549) completes reading state and recovery | The connected application has a scalar service interface and fresh guest Store per action. A typed public view contract remains an unselected design choice, not an active #483 assignment |
| **Robustness and evidence** | Source-lowering retirement is merged; [#281](https://github.com/tseguine/grunkalunka/issues/281)'s open-ended audit is retired | Retain small-stack probes, [tests](testing.md) and [CI](ci.md); track a new concrete defect with a bounded observable completion criterion |

These remaining boundaries explain current capability, not a new assignment or
an indefinite issue backlog. Closed implementation proofs and retired proposals
have different meanings. Continue the maintainer's assigned feature train and
independent workstreams; define new work in concrete reviewable units.

## Source-lowering architecture: completed base and remaining work

The planned effectful-source migration through L3/L4 and L5 compatibility-driver
retirement is merged through [#466](https://github.com/tseguine/grunkalunka/pull/466),
with the evidence/design closure in [#467](https://github.com/tseguine/grunkalunka/pull/467)
and [#468](https://github.com/tseguine/grunkalunka/pull/468). The shared source
cursor owns statements, initializers, evaluator resumptions and lifted bodies;
22 synchronous compatibility drivers were removed. See the
[validation checkpoint](lowering-refactor-validation-2026-11-03.md).
L6 remains conditional on a concrete semantic extension. These refactoring
labels do not declare the similarly numbered language milestones complete.

<details>
<summary>Historical architecture assignment and pre-merge checkpoints (November 1–3)</summary>

**Maintainer-assigned direction, 2026-11-01.** Continue the lowering-machine
migration in bounded, independently reviewable units until source-dependent
cross-phase native-stack recursion has been eliminated or an explicit,
measured exception has been justified. The proof is semantic and stack
behavior, not merely that individual functions now use a `Vec<Work>`.

### Integrated baseline (updated 2026-11-03)

The lowering checkpoint below is current through master
[`926a4aa`](https://github.com/tseguine/grunkalunka/commit/926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c);
the document's earlier cross-stream review retains its own dated evidence baseline.

- [#381](https://github.com/tseguine/grunkalunka/pull/381) centralized
  source admission, cursor tasks and Region handoff.
- The complete [#382–#391](https://github.com/tseguine/grunkalunka/pull/391)
  source-control stack was tested and rebase-merged through #391: one
  heap-owned cursor for if/while/do/for, blocks, sequences and try/catch/finally,
  with explicit source-context checkpoints and composed 2 MiB probes.
  Predecessor PRs were closed as *included in the batch*, not individually merged.
- The [#392–#394](https://github.com/tseguine/grunkalunka/pull/394)
  expression stack was merged through #394: evaluator task/value vocabulary
  and six function-result context crossings were consolidated. A forced-inline
  regression in the 2 MiB Windows truthiness test was bisected and fixed before
  merge. This step alone retained a separate native expression-call boundary.
- The [#395–#396](https://github.com/tseguine/grunkalunka/pull/396)
  source stack was merged through #396: a cursor now owns pending work,
  completed results and source-control continuations; for-loop transitions
  have a dedicated typed module. The combined exact head passed its applicable
  hosted checks. This is a **source-control** milestone, not a complete
  expression/function-body stack-safety proof.
- [#398–#421](https://github.com/tseguine/grunkalunka/pull/421) integrated
  typed source/expression requests for predicates, discard, for headers,
  direct returns and async throw reasons; owned evaluator state and block-arrow
  activations then made block-bodied `LocalCall` yield to the same source cursor.
- [#427](https://github.com/tseguine/grunkalunka/pull/427) integrated the
  #422–#425 scalar-`let`/closure-return/async-return stack and shared local
  publication.
  [#428](https://github.com/tseguine/grunkalunka/pull/428) releases the large
  evaluator runner before discarded/truthy function effects, while
  [#430](https://github.com/tseguine/grunkalunka/pull/430) owns function-result
  charges and factory binding restoration across phases.
- [#431](https://github.com/tseguine/grunkalunka/pull/431) routes direct
  zero-argument local returned block factories through the same source cursor,
  including the returned callee in `get()()`;
  [#433](https://github.com/tseguine/grunkalunka/pull/433) adds runtime-oracle
  and 2 MiB source coverage. [#432](https://github.com/tseguine/grunkalunka/pull/432)
  shares factory argument classification/evaluation but retains synchronous
  argument execution at this frozen master checkpoint.
- [ADR 0020](adr/0020-source-lowering-control-kernel.md) records the
  constraints. [#281](https://github.com/tseguine/grunkalunka/issues/281)
  remains the native-stack-hygiene evidence tracker.

### In-review series (2026-11-03)

At this checkpoint the pushed, unmerged sequence was
[#435](https://github.com/tseguine/grunkalunka/pull/435) →
[#436](https://github.com/tseguine/grunkalunka/pull/436) →
[#437](https://github.com/tseguine/grunkalunka/pull/437) →
[#438](https://github.com/tseguine/grunkalunka/pull/438). It added shared returned-factory
completion, ordered scalar/callback/surplus arguments for direct local block
factories, source-owned Discard/Truthy function construction and ordinary-call
callback continuations. These changes cover both function results and immediately
invoked returned callees, preserving caller-scope argument snapshots, Region
identity, source limits and error order.

The callback-argument unit adds `LowerCallbackArgument` for
`Work::LocalCall` and `Work::ClosureCall`, keeping their pending argument work
on the heap while the source cursor drives the Function-mode child. The shared
`EvaluateFunctionValue` frame uses `FunctionValueResume::{Effect, Callback}`
for effect completion or callback-value delivery, preserving callback admission,
signature provenance and context restoration.

At the callback-argument checkpoint, the published tip was
[`425f9b6`](https://github.com/tseguine/grunkalunka/commit/425f9b6c7dda02224d64901f67d980cc84a07f84).
Its matching source tree passed 500 compiler tests, 118 runtime-closure tests,
targeted async/module callback tests, formatting and strict compiler Clippy.
The [cross-phase inventory](lowering-cross-phase-reentry.md#in-review-argumenteffectcallback-continuation-series)
records exact artifact/diagnostic parity against the frozen baseline and routing
sensitivity checks. These local receipts do not imply a hosted pass or integration
into the master checkpoint.

### Follow-on callee and function-result units

The newer dependent sequence is
[#441](https://github.com/tseguine/grunkalunka/pull/441) →
[#442](https://github.com/tseguine/grunkalunka/pull/442) →
[#443](https://github.com/tseguine/grunkalunka/pull/443) →
[#444](https://github.com/tseguine/grunkalunka/pull/444) →
[#445](https://github.com/tseguine/grunkalunka/pull/445) →
[#446](https://github.com/tseguine/grunkalunka/pull/446) →
[#447](https://github.com/tseguine/grunkalunka/pull/447). The last three units schedule
charged concise results, stage runtime-factory ownership and schedule runtime
factory callees/arguments. These follow-ons to
documentation checkpoint
[#439](https://github.com/tseguine/grunkalunka/pull/439) add no integrated master
claims. They extend native stack #440; the maintainer handles registration where
connector support is unavailable. Native membership and hosted check results
are checked separately from the local implementation receipts.

The callee pair separates validation/direct-slot snapshots from effectful
construction, then schedules non-slot synchronous, awaited and Pending-mode
callees through `LowerClosureCallee` / `EvaluateFunctionValue`. The next pair
stages and schedules concise closure-metadata factory arguments, restoring caller
bindings before materialization and retaining their zero result-admission charge.
The charged concise unit adds `EvaluateFactoryValue` for general expression
bodies: it retains the eight-result token and body function-value admission,
starts expression depth at zero, preserves caller statement depth, and delivers
the completed `ValueId` without a second materialization.

Runtime factories use `EvaluateRuntimeFactoryCallee` followed by the shared
argument continuation. This preserves ordinary admission even for slot callees,
inherited result depth, callee snapshots, complete formal signatures, discarded
inputs, and expansion after argument effects. The compatibility wrapper remains
synchronous and shares the same argument classifier.

The [follow-on receipt and residual inventory](lowering-cross-phase-reentry.md#in-review-callee-and-function-result-continuations)
separate local compiler, runtime, artifact and routing evidence. At that checkpoint
the published implementation tip was #447, tree
`3a12c294a2d55d26ae12f3672d6d00192511cd43`. Those receipts did not establish a
hosted pass, merge, or complete source-lowering SCC migration.

### Final in-review source ownership and retirement

The maintainer subsequently directed the stream to continue through the remaining
refactor. The follow-ons after [#448](https://github.com/tseguine/grunkalunka/pull/448)
complete L3/L4's maintained effectful-source ownership and L5's obsolete-driver
retirement. They remain an in-review implementation, separate from the frozen
integrated master checkpoint above.

The [final source inventory](lowering-cross-phase-reentry.md#final-in-review-effectful-source-ownership)
maps [#449](https://github.com/tseguine/grunkalunka/pull/449) through
[#466](https://github.com/tseguine/grunkalunka/pull/466) to their exact ownership
boundaries: all declaration consumers and for headers, block-factory `.call`,
known-local selection, disposable definition checks, const metadata arguments,
real lifted bodies, identity materialization, queued evaluator resumptions and
factory-definition consumers. The final retirement deletes the synchronous
evaluator, function/factory/callback and body/checking drivers; it retains the
legacy optional-call state oracle as frozen observations, not another interpreter.

Top-level source lowering has one native scheduler entry. Nested effectful source
work uses typed requests, owned child `Lowering` states and queued resumptions.
Checking publishes only its success metadata/source costs; real materialization
retains its distinct lifted-IR publication policy. Caller bindings and resource
tokens remain owned through suspension and restore in their original order.

The [final validation report](lowering-refactor-validation-2026-11-03.md) records
the exact combined revision, source-routing and error-state tests, artifact/runtime
comparisons, composed native-call/value-flow audit and bounded cost observations.
The local correctness receipts, scoped audit and primary/focused cost studies are
complete. The cost record retains shared-host noise and a possible modest increase
in one rejection workload; it does not claim performance equivalence or a general
speedup.
The completion scope is this source-lowering architecture. It does not mean every
fact-query helper, IR pass or runtime phase is acyclic, close the broader
[#281](https://github.com/tseguine/grunkalunka/issues/281) invariant, or establish
hosted exact-head CI before those results are available.

### Ordered implementation units

| Unit | Scope and architectural result | Required evidence / completion criterion |
| --- | --- | --- |
| **L1 — Cross-phase call graph and baselines** | Retain the historical reentry inventory and final composed native-call/value-flow audit. Distinguish queued transitions from native calls, and source effects from fact queries, IR passes and runtime scheduling. | Reproducible edges and any excluded syntactic cycles have explicit source/value-flow justification; admitted and just-over-budget probes retain their host and revision scope. No assumption that source depth equals native stack depth. |
| **L2 — Owned expression request/result seam** | The bounded seam is integrated: typed *compiler* requests/results and owned Regions connect the source cursor to ADR 0001/0016's evaluator. The final in-review implementation uses it for the remaining effectful-source consumers while preserving `Mode`, AST identity and request-site context. | Preserve the established schedule/suspend/resume path, emitted IR/Wasm, side-effect ordering, first diagnostics and 2 MiB behavior. No recursive legacy fallback on a converted path. |
| **L3 — Source expression consumers** | Complete in the final in-review implementation: declarations and for-header initializers join predicates, discard, returns and throw reasons on the source cursor. Typed consumers own dynamic context, per-local publication and lexical Regions; `Completion` algebra is unchanged. | Composed branch/loop/try/lexical cases retain admission, exact diagnostics and generated bytes; initializer effects execute once and in order. Zero-argument declaration completion queues advancement instead of restarting a native dispatcher. |
| **L4 — Function/closure-body reentry** | Complete for maintained effectful-source routes in the final in-review implementation: all callee/argument/factory results, disposable definition checks and real lifted bodies use owned continuations. Captures, async modes, expanded/returned factories and signature provenance retain their separate contracts. | Mixed expression/body/checking/materialization nesting stays on one source scheduler; 2 MiB boundary cases and runtime oracles retain source/argument order, resource limits and publication. Owned suspended activations must not be confused with native runner frames. |
| **L5 — Retire duplicate paths and consolidate ownership** | Complete in the final in-review implementation: synchronous statement/evaluator/function/factory/checking/materialization drivers and their compatibility lease are deleted. Normal and error continuations own restoration; optional helper assertions use the real checker and frozen legacy states. | No obsolete fallback remains on the converted effectful-source circuit. The composed audit distinguishes real native cycles from impossible typed return edges, while exact state/diagnostic checks preserve charges, shared signature identities and lexical scope. |
| **L6 — Facts, extensibility and future semantics** | Review effect-free function signatures, callback origins, stored kinds and Promise/input analysis for genuine duplicate traversal/fixed-point rules. Keep these *queries* distinct from effectful lowering; introduce an extensible normal/throw source-operation interface only when a concrete dynamic Get/Set/Call implementation needs it. | A new supported source feature crosses one well-defined semantic boundary without duplicating fact propagation, using observable JS errors rather than invalid-IR traps. No blanket new CFG, universal dynamic ABI, or ambient host capability as a prerequisite. |
| **L7 — Hardening and design closure** | Keep accepted/rejected JS/TS differential programs, Node-Wasm/Wasmtime comparisons, 2 MiB native-stack fixtures, generated artifact identity, compile-time/peak-memory checks, and CI impact selection aligned with changes. Revise ADR 0020 based on proven implementation boundaries rather than speculative completion. | Exact-head Linux/Windows/macOS policy gates, focused source/runtime oracles, final SCC/stack report and explicit remaining limitations. Close #281 only once its full cross-phase acceptance invariant is supported, not merely because PRs merged. |

The [cross-phase inventory](lowering-cross-phase-reentry.md) preserves the early
native-stack observation and the historical #447 residual circuits alongside
their final replacements. The complete ownership path, its reverse error unwind
and the retirement of all callers establish the source architecture claim;
individual handoff counters alone would not. L7 retains exact-head verification,
cost interpretation and broader native-stack limits through the final validation
report and continuing supported-host policy gates.

**Continuing workflow:** retain the L1 inventory and integrated L2 seam, and keep
future source consumers within the completed L3/L4 ownership contracts. L5 removes
an obsolete path only after its replacement is verified. L6 is optional until a
specific source semantic extension requires it; L7 evidence accompanies each
change. The maintainer authorized dependent
PR series using native PR stacking on 2026-11-03; keep each unit reviewable and
record its parent branch/PR. Where connector support is unavailable, push the
dependent branches and leave native PR stack registration to the maintainer.
Current exact-head checks, review and ownership rules still apply; a caretaker
may repair CI without silently changing semantics. Historical refs from integrated
batches do not constitute unmerged code.

**Preserve:** Oxc semantic identities, typed Region IR and verifier, WasmGC,
specialized numeric and known-value paths, guest and host scheduling
boundaries, source-limit/first-diagnostic order, lexical `for (let ...)`
environments, completion precedence, and JS source side-effect order.
The runtime job queue is not the compiler continuation scheduler. The
full-language runtime, general synchronous exceptions, and GC-aware async
identity have distinct implementation and evidence gates; do not make this
lowering refactor implicitly claim or block them.

</details>

## Runtime semantics and source proofs

Preserve the existing specialized numeric, typed closure and known own-property paths. But a source-flow proof is optimization/admission evidence for an implemented subset, not an alternative definition of JavaScript. The next architecture should expose semantic Get/Set/Call/ToPropertyKey and normal/throw behavior independently of known-operand fast paths, incrementally and only as needed by executed programs.

**Do not silently** translate arbitrary property access into own-only lookup, remove receiver/coercion effects, assume a positive constructor fact excludes unknown alternatives, or report a source TypeError as a Wasm trap. Incorrect private projections may still trap on *invalid IR*; ordinary admitted source operations need catchable abrupt completions. Preserve native source order, saved snapshots, actual call targets and handler/finalizer precedence. The first proof need not implement full prototypes, proxies or all built-ins; unsupported valid forms must remain explicit rejections until semantics are correct.

The current structured IR and WasmGC choice do not need reversal. Keep proven f64 arithmetic unboxed. Introduce a broader CFG, universal dynamic-call ABI or object shapes/caches only on measured or demonstrated needs; avoid building a second language interpreter by default. Relevant proposed boundary documents ([ADRs 0011](adr/0011-semantic-values-and-lifetime-boundaries.md), [0012](adr/0012-guest-runtime-and-host-capabilities.md), [0013](adr/0013-async-job-and-activation-ownership.md)) stay **proposed**.

## Managed suspension and runtime lifetime

Never store raw WasmGC references in a linear-memory numeric spill or conflate
the managed carrier with the async Promise tag space. The accepted initial
[WasmGC design](adr/0019-wasmgc-runtime.md) keeps actual references in traced
globals and per-activation records beside numeric control addresses. Related
activations share one generated instance; completed roots are released and
late/stale identities are checked. Public sharing across separate host entries
and managed Promise payloads need their own contracts. Keep JS job semantics
distinct from optional future parallelism.

## Application and verification gates

**A1 integration is independent of general-JS completion.** The live application
already connects the scalar source-to-WIT call and native hit/repaint path.
Further bounded application work can preserve that path without requiring a
DOM, full CSS, ambient Node APIs, general Promise objects or a full window
system. Preserve WIT/capability checks and avoid unnecessary per-pixel component
crossings. A future public view/action interface needs concrete selected
semantics and equivalent native Rust/generated JS behaviour before adoption.

**Report verification categories separately:** admitted source with executed native-JS/Node-Wasm/Wasmtime comparisons; admitted source with known mismatches; rejected source (including invalid source and resource limits separately); source not attempted; and hand-built IR/runtime evidence that does not establish source admission. Compare nonnumeric effects and job order, not only final scalar bits. Start an original-harness Test262 pilot for a few *selected* supported language semantics; do not describe existing adapted fixtures as full Test262 coverage or make an unbounded suite a blanket PR gate. Use the current [executable corpus](../tests/compatibility/corpus.mjs) and [check catalog](../scripts/checks/README.md) for current counts; historical receipts remain dated.

**Success of this review:** ongoing work can find its actual next independently verifiable integration frontier, without assigning a new architecture by implication or mistaking regression volume for language completeness.
