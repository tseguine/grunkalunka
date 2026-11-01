# JavaScript engine steering: semantic convergence gates

Date: 2026-11-01. Evidence baseline: master commit [5f3c43c](https://github.com/tseguine/grunkalunka/commit/5f3c43cd802bb0da9f09b1941ef90ffc7e57ae6c).

**Status: advisory execution steering.** The maintainer asked for this cross-stream review. It records observed implementation, proposed next proof points and existing ownership boundaries. It is not a new language profile, accepted ABI, automatic agent assignment or permission to interrupt unrelated work. [AGENTS.md](../AGENTS.md) remains authoritative for permissions, PR review and merging.

## What is currently established

- The maintained Oxc frontend lowers selected JS/TS through verified structured IR to real portable Wasm. Numeric and primitive-return profiles retain their contracts and specialized f64 paths.
- The **synchronous** WasmGC path executes source ordinary-object and sparse-array literals, static own-property reads/writes, Strings with lossless UTF-16 equality/truthiness, aliases, typed helpers, captured managed references, returned closures and canonical function identities. Calls through own object/array fields require source-proven compatible synchronous signatures. See [managed values](managed-values.md) and [accepted WasmGC direction](adr/0019-wasmgc-runtime.md).
- The **async** profile supports bounded Promise identity, settlement/adoption, captured scalar storage, suspended continuations and selected try/catch/finally/job ordering. It does **not** yet carry traced WasmGC values through suspended frames or provide general Promise/thenable or synchronous exception semantics.
- A1 has authorized Markdown/WIT inspection, native layout/paint/hit/repaint and a **separate** source-compiled scalar call to a real Rust WIT service. No connected source-service-input-repaint application or rendered readable glyph pipeline is accepted.
- Stack-hardening has converted many source, IR, emission and JSON walks to heap worklists, but the remaining native-stack audit is tracked in [#281](https://github.com/tseguine/grunkalunka/issues/281).

These are **different bounded execution profiles**, not a claim of general ECMAScript support or an implemented browser/Node environment. Historical observations lower in the roadmap and milestone ledger describe their named revisions, not this state.

## Steering by concern

| Concern | Evidence and current owner boundary | Next bounded proof / exit criterion |
| --- | --- | --- |
| **Accepted-source correctness** | [#334](https://github.com/tseguine/grunkalunka/issues/334) reproduces a captured for-let header returning 33 instead of native JS's 2; IR/control-flow owns the existing corrective work | Per-iteration header binding renewal, including continue and mutable escaped closures; native JS/Node-Wasm/Wasmtime agree and old var/body-local behavior remains |
| **Semantic runtime path** | [#355](https://github.com/tseguine/grunkalunka/issues/355); general-value source lowering currently requires static keys/receivers and exhaustive typed field-call proofs | A genuinely runtime-dependent key or call operation has correct supported JS semantics, with an admitted catchable source TypeError and distinct internal/host failures |
| **GC-aware suspension and identity** | [#356](https://github.com/tseguine/grunkalunka/issues/356) and root [#300](https://github.com/tseguine/grunkalunka/issues/300); synchronous traced references and numeric async frames are separate | Source-compiled managed object/closure survives a real suspension, forced collection, alias mutation, and ordered resume; two related activations share the intended identity domain and teardown is checked |
| **First integrated app** | [#52](https://github.com/tseguine/grunkalunka/issues/52), [A1 preview](a1-note-preview.md) and [source WIT import](a1-ui-source-import.md); owner for the full next slice must be selected by maintainer | Input hit invokes source-compiled JS/TS, calls the actual authorized WIT service, changes state, and drives native reflow/repaint with readable headings; native Rust uses the same service contract |
| **Robustness and evidence** | [#281](https://github.com/tseguine/grunkalunka/issues/281), [tests](testing.md), [CI](ci.md) | Retain small-stack admitted/rejection probes; distinguish executed admitted behavior, rejected sources, unexecuted scenarios, observed mismatches and independent runtime-only fixtures |

The labels above order concerns within their relevant streams; they do **not** stop the parallel compiler, IR/stack, A1, browser or document streams or silently hand work to an unassigned agent. The A1 proposal [#54](https://github.com/tseguine/grunkalunka/issues/54) remains unassigned until the maintainer chooses its owner.

## Runtime semantics, not ever-more mandatory proofs

Preserve the existing specialized numeric, typed closure and known own-property paths. But a source-flow proof is optimization/admission evidence for an implemented subset, not an alternative definition of JavaScript. The next architecture should expose semantic Get/Set/Call/ToPropertyKey and normal/throw behavior independently of known-operand fast paths, incrementally and only as needed by executed programs.

**Do not silently** translate arbitrary property access into own-only lookup, remove receiver/coercion effects, assume a positive constructor fact excludes unknown alternatives, or report a source TypeError as a Wasm trap. Incorrect private projections may still trap on *invalid IR*; ordinary admitted source operations need catchable abrupt completions. Preserve native source order, saved snapshots, actual call targets and handler/finalizer precedence. The first proof need not implement full prototypes, proxies or all built-ins; unsupported valid forms must remain explicit rejections until semantics are correct.

The current structured IR and WasmGC choice do not need reversal. Keep proven f64 arithmetic unboxed. Introduce a broader CFG, universal dynamic-call ABI or object shapes/caches only on measured or demonstrated needs; avoid building a second language interpreter by default. Relevant proposed boundary documents ([ADRs 0011](adr/0011-semantic-values-and-lifetime-boundaries.md), [0012](adr/0012-guest-runtime-and-host-capabilities.md), [0013](adr/0013-async-job-and-activation-ownership.md)) stay **proposed**.

## Managed suspension and runtime lifetime

Never store raw WasmGC references in a linear-memory numeric spill or conflate the managed-value carrier with the async Promise tag space. Decide the JavaScript agent's heap/instance identity domain, where suspended references are rooted, which jobs own resumptions, and how teardown invalidates late work before widening async admission. Compare a shared engine-traced activation/frame design against an explicit checked rooting bridge before committing to a general layout. Keep JS job semantics distinct from optional future parallelism.

## Application and verification gates

**A1 integration is independent of general-JS completion.** Reuse the existing limited scalar source-to-WIT call and headless hit/repaint path; the next integrated example does not require a DOM, full CSS, ambient Node APIs, general Promise objects, or a full window system. Preserve WIT/capability permission checks and avoid unnecessary per-pixel component crossings. Require the same intended view/action semantics from native Rust and generated JS/TS.

**Report verification categories separately:** admitted source with executed native-JS/Node-Wasm/Wasmtime comparisons; admitted source with known mismatches; rejected source (including invalid source and resource limits separately); source not attempted; and hand-built IR/runtime evidence that does not establish source admission. Compare nonnumeric effects and job order, not only final scalar bits. Start an original-harness Test262 pilot for a few *selected* supported language semantics; do not describe existing adapted fixtures as full Test262 coverage or make an unbounded suite a blanket PR gate. Use the current [executable corpus](../tests/compatibility/corpus.mjs) and [check catalog](../scripts/checks/README.md) for current counts; historical receipts remain dated.

**Success of this review:** ongoing work can find its actual next independently verifiable integration frontier, without assigning a new architecture by implication or mistaking regression volume for language completeness.
