# Documentation guide

Start at the [project README](../README.md) for a supported build/check command,
workspace map and high-level scope. This page routes readers to the maintained
contracts; it is not a second feature inventory. For the current execution
priorities see [milestones](milestones.md), the [JavaScript engine steering
review](javascript-engine-steering.md) for current capabilities and remaining
boundaries, and the [research roadmap](roadmap.md) for longer-term questions.

## Compiler and runtime

- [Accepted ADR 0022: general JS/TS lowering first](adr/0022-general-js-ts-lowering-first.md):
  coverage of valid JS/TS source and workable Wasm execution comes before
  exhaustive compliance or further optimization. The historical feature train
  remains useful implementation evidence, not the governing compiler priority.
- [Compiler/source admission](compiler.md) and [typed region IR](ir.md): admitted
  JS/TS profiles, diagnostics, verifier and backend.
- [Managed values and source data objects](managed-values.md) and
  [WasmGC decision](adr/0019-wasmgc-runtime.md): objects, sparse arrays, UTF-16
  Strings, typed captures and traced references across supported suspension.
  Public reference exports and managed Promise settlements remain separate.
- [JavaScript feature train](javascript-feature-train.md),
  [String methods](managed-string-methods.md),
  [object bindings](object-binding-patterns.md),
  [array bindings](array-binding-patterns.md), and
  [iteration](array-iteration.md): practical source combinations and their
  receiver, mutation and control-flow boundaries.
- [Async and promises](async.md), [runtime closures](runtime-closures.md), and
  [bounded String/JSON source experiments](heap-literals.md): distinct
  implementations with distinct admission boundaries.
- [Synchronous errors](synchronous-errors.md): local source throws, lexical
  catches and finally completion; cross-call throw propagation remains outside
  that contract.
- [Source-lowering validation](lowering-refactor-validation-2026-11-03.md) and
  [cross-phase inventory](lowering-cross-phase-reentry.md): the completed
  source-cursor migration, its frozen evidence and residual native-stack scope.
- [CLI](cli.md), [Node build API](build-api.md) and
  [opt-in diagnostics](observability.md): execution and debugging entry points.
- [Known lowering gaps](lowering-gaps.md): **numeric-profile** rejection
  inventory, not a list of everything unsupported by every experimental profile.

## Components, applications and browser

- [Capabilities](capabilities.md), [authorized Markdown/WIT outline](document-outline-authority.md)
  and [strict core-Wasm composition](adr/0008-strict-core-wasm-composition.md):
  explicit authority and transport experiments.
- [Live A1 application](a1-live-application.md) and
  [detached live package](a1-live-package.md): the connected bounded note app,
  native pixels, browser input forwarding, note selection, reload and session
  recovery. Rust owns the application state; the browser presents its output.
- [A1 note preview](a1-note-preview.md),
  [owned-session replay](a1-session-replay.md), and
  [source-compiled A1 UI import](a1-ui-source-import.md): headless reproduction,
  action/frame evidence and the scalar JS/TS-to-Rust/WIT service boundary.
  These do not establish a public DOM or general typed reference API.
- [Headless block layout](block-layout.md), [RGB painting](block-paint.md),
  [hit testing](headless-hit-test.md), [native flex rows](native-flex-rows.md),
  and [browser oracles](browser-oracles.md): resolved native layout contracts
  and independent browser observations, with a full HTML/CSS engine still outside
  the implemented surface.
- [Component-composed environments](adr/0018-component-composed-environments.md):
  **proposed** architecture, not an accepted API or assigned workstream.

## Verification and project evidence

- [Milestone status](milestones.md) and [research roadmap](roadmap.md) distinguish
  executed increments from intended end states. Much of their lower content is a
  dated experiment log; read current headings before interpreting old counts.
- [External tests and benchmarks](testing.md),
  [P1 portability/cost evidence](p1-evidence.md), and
  [CI](ci.md) with [impact selection](../ci/impact/README.md): how evidence is
  generated and which platforms/revisions it covers.
- [Agent coordination](../AGENTS.md) and the [ADR directory](adr/) hold working
  conventions and recorded decisions. Read each ADR's **status**; proposals do
  not become accepted policy merely by appearing in the repository.

The [executable compatibility corpus](../tests/compatibility/corpus.mjs) and
ordinary test suites are more authoritative than dated result counts. Rejecting a
source is not equivalent to supporting it; a standalone component/runtime
experiment is not a source-language compatibility claim.
