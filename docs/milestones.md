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
| L0 — numeric expressions | Done | JS/TS -> portable Wasm; Wasmtime + Node-Wasm differential tests including signed zero, NaN, infinities and rounding. [Compiler contract](compiler.md). |
| L1 — local values and decisions | Done | Numeric `const` locals and comparison-controlled ternaries; the L1 run had 31 JS/TS pairs, 76,508 engine comparisons and 64 rejected inputs. Bindings become available after their initializer. |
| L2 — mutable control flow | Demonstrated | Numeric mutation, lexical blocks, `if`/`else`, early returns, numeric truthiness and `while`/`for`/`do...while`; the initial L2 run had 72 JS/TS pairs, 177,696 comparisons and 100 rejections. NaN/±0 conditions, nested loops, per-entry initialization and selected branches agree with JS. A separate runtime test observes OutOfFuel and successful instance reuse. Numeric compound assignments, prefix/postfix updates and parenthesized write statements are now admitted. Unlabelled break now exits the nearest while/for/do loop; continue, labels and other control forms remain explicit compatibility gaps. |
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
| W0 — maintained implementation workspace | Done | Main Cargo workspace at development version `0.0.1`: separate compiler, numeric runtime and CLI crates, with the regression corpus in `tests/compatibility`. Generated Wasm and external dependency resolution are unchanged by promotion. Continue PoC iteration in maintained crates; reserve disposable spikes for unresolved questions. |
| C1 — executable compatibility matrix | Done | [Corpus version 25](../tests/compatibility/corpus.mjs): 424 paired programs, 425 rejection fixtures, five explicit unexecuted feature groups. The [53 lowering-gap references](lowering-gaps.md) separately execute their specified JS result and verify the current rejection. Reports distinguish expected admission from test outcome, preserve runtime/build provenance, seed, source/artifact hashes and rejection diagnostics. Every admitted fixture compares JS to both Wasm engines; every rejection verifies no artifact. |
| C2 — external conformance selection | In progress | A pinned TypeScript arithmetic source supplies 24 adapted runtime cases and eight admission rejections, with explicit provenance and limitations. This does not run the TypeScript conformance suite. Four pinned Test262 numeric programs adapt 18 assertions with an independent success oracle and per-check failure probes. Full upstream execution still needs its original harness and broader semantics. Record exclusions rather than silently skipping them. Use WPT for later browser behavior; Acid as historical regression; Can I Use as feature-roadmap input. |
| R1 — WIT capability profiles | Demonstrated | [One WIT component](capabilities.md) under absent/denied/granted profiles; typed policy errors and backing-read counters distinguish authorization from guest handle validation. Revocation, aliases, store ownership and three specific guest handle misuse cases are observed. No WASI links. Independent core-module tests observe fuel exhaustion, per-memory growth rejection, expired-epoch trapping/renewal, epoch expiry triggered by a host callback during an active invocation, and a host algorithm enforcing an item-processing quota. These are separate controls, not a complete component budget. |
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
  runtime dependency required.
- C1: the same corpus now produces a versioned per-fixture evidence inventory.
- L2a: initialized mutation and assignment values pass 106,124 comparisons;
  the corpus detects a deliberately omitted write.
- L2b: scoped statements and early returns pass 140,676 comparisons; a lost
  early return is detected even when the resulting Wasm validates.
- W0: compiler, numeric runtime and CLI are maintained workspace crates at 0.0.1.
  L2c demonstrates numeric truthiness and while loops in the maintained code.
- L2c: loop differential fixtures pass 177,696 comparisons. Wasmtime fuel exhaustion
  is typed and recoverable for the next call; other engines receive only finite
  fixtures.
- R1a: generated WIT bindings and a component resource fixture pass six profile,
  lifecycle and handle-misuse tests. A policy-bypass mutation fails observably.
- R1b: two finite core-module tests observe per-memory growth limits and expired
  epoch deadlines at a subsequent call. Omission mutations fail both checks.
  Wall-clock latency, aggregate budgets and portability remain P1 follow-ups.
  Further language/module work stays planned above.
- R1c: a finite guest performs observable work, then a host callback expires its
  epoch deadline during the invocation. The call traps before finishing all three
  iterations. Disabled ticking returns 42; omission mutation fails the trap check.
- Corpus 5: the selected TypeScript arithmetic adaptations expand replay to 236,928
  comparisons and 108 expected rejections; upstream type checking remains untested.
- R1d: five host-work tests observe valid and invalid requests, zero and partial
  allowance, cumulative consumption and typed exhaustion before an unfunded item.
  Removing the quota guard produces an incorrect successful result. The initial
  L1/L2/C1/R1 demonstration horizon is implemented; the later tracks remain planned.
- TypeScript expression erasure: `as`, angle assertions, non-null and `satisfies`
  delegate to admitted numeric expressions or predicates. Twenty-eight paired
  programs cover their runtime erasure and boundaries; type checking, type-only
  declarations and wrapped write targets remain separate work.
- Numeric `for`: a lexical initializer block and an optional loop step preserve
  initializer/test/body/update order and early return. Twenty-four paired cases
  cover omissions, scope, effects and source budgets; 30 boundary cases reject
  unsupported headers, control flow and scope leaks. `for-in`/`for-of`,
  `continue` and labels remain separate work.
- Numeric `do...while`: the body executes before the condition and carries its
  guaranteed-return coverage. Twenty-one paired programs cover iteration effects,
  early return, scopes and source budgets; 18 boundary rejections preserve whole
  AST admission, structural completion and the remaining control-flow exclusions.
- Number-only `==`/`!=` predicates reuse numeric comparisons and ordered snapshots.
  Nineteen paired programs cover boundaries, operand effects and composed
  conditions; 20 boundary rejections preserve numeric result and operand admission.
  Mixed-type and Boolean equality remain separate work.
- Discarded numeric expression statements preserve source-order effects and
  evaluate through the same numeric lowerer as value positions. Twenty paired
  programs include arithmetic, comma, selected branches, whole TypeScript wrappers
  and finite loop bodies; 19 retained boundary cases check Number-only admission,
  wrapped-target restrictions and source limits.
- Empty statements fall through without IR allocation. Twelve paired programs
  cover standalone/nested empties, branches, finite loop bodies and exact source
  count/nesting bounds. Eight boundary rejections preserve source limits, return
  coverage, unreachable-statement policy and the one-export module shape.
- Unresolved `Infinity` and `NaN` reads become numeric constants. Nineteen paired
  programs cover values, effects, lexical shadows, scope restoration and source
  depth; 32 boundary rejections preserve TDZ, immutable global writes, exact names
  and property-access restrictions. No host access or general global support is added.

Use [the research roadmap](roadmap.md) for rationale, source references, licensing,
quirk flags, browser scope and portable/module boundaries. Update this plan after
each demonstrated unit; do not replace evidence with a checked milestone box.

The [external testing plan](testing.md) registers Test262, TypeScript tests, the
Computer Language Benchmarks Game, JetStream, Node.js tests and Speedometer.
Pinned targets, concrete prerequisites and selected execution results are included
in compatibility reports. Registration does not complete C2, P1, A1 or B1.
