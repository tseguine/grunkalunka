# Spike/PoC milestones

This is the active execution plan. Each milestone ends in a runnable demonstration,
measured compatibility evidence, and an updated status. A rejected input is not a
supported feature; an unattempted test is not a passing test.

## Working authority and quality

On 2026-10-23 the user authorized autonomous iteration and merging to `master`
through the spike/PoC stage, while retaining similar quality and reducing approval
ceremonies. Use small coherent commits, meaningful tests, proportional review and
clear limitations. Routine implementation choices and verified local integration
do not require renewed approval. Revisit major direction changes, irreversible
external actions, and a transition out of this stage. This is project-specific;
it does not change workspace policy for other projects.

The current goal's first practical horizon is L1, L2, C1 and R1: useful scalar
control flow, explicit compatibility reporting, and one component whose authority
changes with its capability profile. The later milestones remain visible without
pretending a whole browser/framework is a short implementation task.

## Lowering track

| Milestone | Status | Demonstration / completion evidence |
| --- | --- | --- |
| L0 — numeric expressions | Done | JS/TS -> portable Wasm; Wasmtime + Node-Wasm differential tests including signed zero, NaN, infinities and rounding. [Spike 002](../spikes/002-numeric-wasm/README.md). |
| L1 — local values and decisions | Next | Numeric `const` locals, comparisons and conditional expressions. Resolve bindings through semantic symbols; preserve initialization order; evaluate only the selected branch. Differential tests exercise every admitted operator and branch. |
| L2 — mutable control flow | Planned | Numeric `let`, assignment and structured `if`/loops, delivered in separate small units. Define numeric truthiness explicitly; test NaN/±0, lexical lifetime and early returns. Bound guest execution and observe fuel exhaustion. |
| L3 — functions and modules | Planned | Multiple functions, calls and one small module graph with explicit imports/exports. Test evaluation order, argument handling and module initialization. Reject unsupported cycles/features explicitly. |
| L4 — JS value semantics | Research gate | Compare small value-representation experiments for strings, undefined/null, booleans, numbers, objects/prototypes and coercion. Demonstrate equivalent observable behavior before selecting a production representation or GC strategy. |
| L5 — exceptions, closures and async | Planned | Separate demonstrated slices for capture/lifetime, throw/catch/finally and promise/job ordering. No blanket compatibility claim from one slice. |
| T1 — tsrs integration | Research gate | Pin source; perform one real type query associated with an expression; compare tsrs-only against Oxc+tsrs representations. Type information must not erase observable JS behavior. |

L1/L2 retain an explicitly numeric ABI during the experiment. They expand a
demonstration, not the platform's final language contract. Boolean conditions may
be represented internally without changing the public two-number/result-number ABI.

## Compatibility, runtime and application tracks

| Milestone | Status | Demonstration / completion evidence |
| --- | --- | --- |
| C1 — executable compatibility matrix | Next | A versioned fixture manifest records admitted, rejected and unimplemented surfaces with reference runtime versions. Every admitted fixture compares original JS to emitted Wasm in both engines; every rejection verifies a diagnostic and no artifact. Preserve deterministic seed, counts and concrete gaps. |
| C2 — external conformance selection | Planned | Pin and license-check an appropriate Test262 subset; connect tests to supported semantics. Record exclusions rather than silently skipping them. Use WPT for later browser behavior; Acid as historical regression; Can I Use as feature-roadmap input. |
| R1 — WIT capability profiles | Next after scalar control flow | One component and an explicit host resource/interface. Same artifact under absent/denied/granted profiles; observe authority changes, isolation and revocation behavior. Add resource/fuel/deadline observations as independent checks. No ambient WASI links by default. |
| P1 — portability and costs | Planned | Reuse emitted Wasm across engines/hosts; test Linux/macOS/Windows and relevant architectures before claiming support. Separate frontend/lowering, compilation, instantiation, execution, host-call, artifact/executable and memory costs. Report reproducible workloads and variance. |
| A1 — first useful framework app | Planned | A small application with straightforward build/run/debug/package flow, using prior component/capability evidence. Show the same application under two capability profiles. |
| B1 — reusable browser pieces | Later | Compare libraries before custom implementation. Own HTML/CSS modules should work through explicit inputs outside the browser shell; prove a second consumer. Embedded and standalone browser use primarily differ through capability profiles. |
| M1 — alternative markup + converters | Research gate | Evaluate KDL/SDLang as application authoring syntaxes for XHTML-equivalent documents. Support includes round-trip converters: preserve namespaces, attributes, ordered mixed text/elements, significant whitespace and comments. Reject unsupported constructs without data loss. Exact source formatting/trivia preservation is a separate fidelity contract. |

### Candidate reference client: Obshittian

The user proposed an Obsidian-like client, self-deprecatingly named **Obshittian**.
Treat it as a candidate reference application, not an instruction to implement a
full clone now. A useful first slice would edit/save Markdown notes inside a
granted vault and follow note links. It could exercise rendering, editing,
persistence, search and later extension capabilities through the framework.
Verify that its vault capability cannot access paths outside the grant, and that
the same client behaves predictably under different capability profiles.

## Iteration record

- Baseline: `63c993a` combines frontend observations, numeric lowering and the
  browser/framework direction. Its full checks passed before integration into
  `master`. L0 has 39,488 numeric comparisons and 48 expected rejections.
- Next unit: L1's local numeric values and conditional expressions, with the
  compatibility harness expanded alongside it. No new runtime dependency required.

Use [the research roadmap](roadmap.md) for rationale, source references, licensing,
quirk flags, browser scope and portable/module boundaries. Update this plan after
each demonstrated unit; do not replace evidence with a checked milestone box.
