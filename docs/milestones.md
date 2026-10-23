# Spike/PoC milestones

This is the active execution plan. Each milestone ends in a runnable demonstration,
measured compatibility evidence, and an updated status. A rejected input is not a
supported feature; an unattempted test is not a passing test.

## Execution scope and quality

Use small coherent commits, meaningful tests, proportional review and clear
limitations. Each experiment should resolve a concrete engineering uncertainty
before broader implementation depends on its result.

The current goal's first practical horizon is L1, L2, C1 and R1: useful scalar
control flow, explicit compatibility reporting, and one component whose authority
changes with its capability profile. The later milestones remain visible without
pretending a whole browser/framework is a short implementation task.

## Lowering track

| Milestone | Status | Demonstration / completion evidence |
| --- | --- | --- |
| L0 — numeric expressions | Done | JS/TS -> portable Wasm; Wasmtime + Node-Wasm differential tests including signed zero, NaN, infinities and rounding. [Spike 002](../spikes/002-numeric-wasm/README.md). |
| L1 — local values and decisions | Done | Numeric `const` locals and comparison-controlled ternaries; 31 JS/TS pairs, 76,508 engine comparisons, 64 rejected inputs. Bindings become available after their initializer. Numeric truthiness, mutation and statement control flow remain outside admission. |
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

### Later authoring option: XAML

Evaluate XAML as a UI authoring language after the application and rendering
contracts have executable evidence. Select a dialect and an explicit supported
subset before claiming compatibility. Explore object/property construction,
namespaces, bindings, resources, styles/templates and event wiring separately;
parsing XML alone does not demonstrate UI compatibility. Keep host access behind
the same capability interfaces used by other authoring formats.

XAML's object model is a separate research question from M1's XHTML-equivalent
KDL/SDLang syntax. Identify which constructs can share a representation and which
need distinct semantics before promising lossless cross-format conversion.
Any supported converter needs an explicit fidelity contract and round-trip tests.
See Microsoft's [XAML language overview](https://learn.microsoft.com/en-us/dotnet/desktop/wpf/xaml/)
and [WinUI XAML overview](https://learn.microsoft.com/en-us/windows/apps/develop/platform/xaml/xaml-overview)
for the language/framework distinction and representative UI features.

### Candidate reference client: Obshittian

**Obshittian** is a candidate Obsidian-like reference application. A useful first
slice would edit/save Markdown notes inside a
granted vault and follow note links. It could exercise rendering, editing,
persistence, search and later extension capabilities through the framework.
Verify that its vault capability cannot access paths outside the grant, and that
the same client behaves predictably under different capability profiles.

## Iteration record

- Baseline: `63c993a` combines frontend observations, numeric lowering and the
  browser/framework direction. Its full checks passed before integration into
  `master`. L0 has 39,488 numeric comparisons and 48 expected rejections.
- L1: const locals and conditional expressions pass 76,508 comparisons; no new
  runtime dependency required. Next: C1's explicit manifest, then L2 control flow.

Use [the research roadmap](roadmap.md) for rationale, source references, licensing,
quirk flags, browser scope and portable/module boundaries. Update this plan after
each demonstrated unit; do not replace evidence with a checked milestone box.
