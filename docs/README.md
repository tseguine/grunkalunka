# Documentation guide

Start at the [project README](../README.md) for a supported build/check command,
workspace map and high-level scope. This page routes readers to the maintained
contracts; it is not a second feature inventory. For the current execution
priorities see [milestones](milestones.md), the [JavaScript engine steering
review](javascript-engine-steering.md) for cross-stream convergence gates, and
the [research roadmap](roadmap.md) for longer-term questions.

## Compiler and runtime

- [Compiler/source admission](compiler.md) and [typed region IR](ir.md): admitted
  JS/TS profiles, diagnostics, verifier and backend.
- [Managed values and source data objects](managed-values.md) and
  [WasmGC decision](adr/0019-wasmgc-runtime.md): synchronous object subset,
  traced references, typed synchronous captures and remaining dynamic/prototype/async exclusions.
- [Async and promises](async.md), [runtime closures](runtime-closures.md), and
  [bounded String/JSON source experiments](heap-literals.md): distinct
  implementations with distinct admission boundaries.
- [CLI](cli.md), [Node build API](build-api.md) and
  [opt-in diagnostics](observability.md): execution and debugging entry points.
- [Known lowering gaps](lowering-gaps.md): **numeric-profile** rejection
  inventory, not a list of everything unsupported by every experimental profile.

## Components, applications and browser

- [Capabilities](capabilities.md), [authorized Markdown/WIT outline](document-outline-authority.md)
  and [strict core-Wasm composition](adr/0008-strict-core-wasm-composition.md):
  explicit authority and transport experiments.
- [A1 note preview](a1-note-preview.md) and
  [source-compiled A1 UI import](a1-ui-source-import.md): runnable host-managed
  preview and separate source-to-Rust service proof. Neither establishes an
  integrated interactive app or public DOM API.
- [Headless block layout](block-layout.md), [RGB painting](block-paint.md),
  [hit testing](headless-hit-test.md), and [browser oracles](browser-oracles.md):
  reusable experiments, not a full HTML/CSS/browser engine.
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
