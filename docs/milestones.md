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
| L2 — mutable control flow | Demonstrated | Numeric mutation, lexical blocks, `if`/`else`, early returns, numeric truthiness and `while`/`for`/`do...while`; the initial L2 run had 72 JS/TS pairs, 177,696 comparisons and 100 rejections. NaN/±0 conditions, nested loops, per-entry initialization and selected branches agree with JS. A separate runtime test observes OutOfFuel and successful instance reuse. Numeric compound assignments, prefix/postfix updates and parenthesized write statements are now admitted. Unlabelled break/continue now target the nearest while/for/do loop; switch, labels and other control forms remain explicit compatibility gaps. |
| L3 — functions and modules | In progress | Top-level direct numeric helpers, exact arity, ordered argument snapshots and self/mutual recursion are demonstrated. Function values, nested declarations, captures and module graphs/initialization remain. |
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
| C1 — executable compatibility matrix | Done | [Corpus version 44](../tests/compatibility/corpus.mjs): 1158 paired programs, 861 rejection fixtures, five explicit unexecuted feature groups. The [38 lowering-gap references](lowering-gaps.md) separately execute their specified JS result and verify the current rejection. Reports distinguish expected admission from test outcome, preserve runtime/build provenance, seed, source/artifact hashes and rejection diagnostics. Every admitted fixture compares JS to both Wasm engines; every rejection verifies no artifact. |
| C2 — external conformance selection | In progress | A pinned TypeScript arithmetic source supplies 32 adapted runtime cases and zero selected admission rejections, with explicit provenance and limitations. This does not run the TypeScript conformance suite. Four pinned Test262 numeric programs adapt 18 assertions with an independent success oracle and per-check failure probes. Full upstream execution still needs its original harness and broader semantics. Record exclusions rather than silently skipping them. Use WPT for later browser behavior; Acid as historical regression; Can I Use as feature-roadmap input. |
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
  programs cover their runtime erasure and boundaries. That unit left type checking,
  type-only declarations and wrapped write targets as separate work.
- Numeric `for`: a lexical initializer block and an optional loop step preserve
  initializer/test/body/update order and early return. Twenty-four paired cases
  cover omissions, scope, effects and source budgets; 30 boundary cases reject
  unsupported headers, control flow and scope leaks. At that stage, `for-in`/`for-of`,
  continue and labels remained separate work.
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
- Unlabelled continue reaches the nearest loop's next-iteration machinery.
  Twenty-seven paired programs cover for updates, do tests, nested targets,
  lexical initialization, mixed break/continue/return paths and source budgets;
  26 boundary rejections preserve whole-source admission and return coverage.
  Finite mutations skipping the for update or do test are detected. Generated
  body-end blocks leave affine at 93 bytes and grow Mandelbrot from 490 to 504
  bytes; the fuel allowance remains 10,000 and no speed claim follows.

- Numeric unary `~` truncates and wraps Number operands before complementing,
  preserving captured operand effects and returning a Number. Twenty-one paired
  programs and 20 boundary rejections cover conversion, snapshots, wrappers and
  source budgets. Actual emitted Wasm is also checked over 4,265 focused values
  in both engines; flooring and saturation mutations are detected. That unit left
  binary bitwise operators and shifts as separate work.
- Number-only binary `&`, `|` and `^` reuse the captured conversion helper and
  preserve eager left-to-right evaluation. Thirty paired programs and 30
  boundary rejections cover snapshots, conversion edges, wrappers and budgets;
  both engines also check 5,110 focused input pairs. Operator-mapping and operand-order
  mutations are detected. That unit left shifts and bitwise compound assignments as separate work.

- Initial Oxc-recognized hashbang comments erase without IR or source statement
  charges. Eighteen full-source pairs and 22 boundary rejections cover line
  endings, Unicode, directives, unchanged numeric output and original-source
  budgets. Three former rejections are promoted. BOM-prefixed/misplaced comments,
  invalid entries and unsupported bodies remain rejected; no source normalization
  is added. Restored refusal, BOM stripping and incorrect statement charging
  mutations are detected.

- Number-only shifts reuse captured bit conversion and Wasm count masking.
  Thirty-four paired programs and 31 boundary rejections cover signed/unsigned
  results, five-bit counts, operand effects, wrappers, loops and budgets. Both
  engines check 9,504 additional edge/raw pairs; opcode and signedness mutations
  are detected. That unit left bitwise/shift compound assignments and mixed-type coercion as separate work.

- Parameter, return and local annotation syntax is admitted as unvalidated
  metadata, retained on the original Oxc AST through compilation. Twenty-three
  paired programs and 24 new rejections cover contradictory/compound/query types,
  effects and unchanged runtime/binding/source restrictions. Four ordinary
  rejections are promoted; the 42 reference gaps remain. Restoring a number-only
  local annotation guard is detected. This adds no checker, persistent type
  metadata API, IR fields or optimization claim. One full-source pair composes
  unknown/compound annotations with an unsigned shift whose RHS writes its left binding.

- Number-only bitwise/shift compound operators reuse the existing assignment
  path and numeric Binary operations. Thirty-nine paired programs and 29 boundary
  rejections check target capture, stores, yielded snapshots, RHS effects, unsigned
  results, wrappers, loops and budgets. Ten ordinary rejections are promoted without
  reducing the curated frontier. A finite operator-mapping mutation is detected.

- Explicit aliases/interfaces and direct named/default-interface exports emit no
  runtime code while retaining the original AST and one statement charge each.
  Thirty-six paired programs and 40 new rejections cover positions, type/value
  bindings, completion preservation and budgets. Three rejections are promoted,
  leaving 41 reference gaps. Missing-charge and broad abrupt-tail mutations are
  detected. Other module forms and a type checker remain separate work.
  One full-source case composes module/post-return aliases with an unsigned shift
  assignment whose RHS writes its target, preserving the target snapshot.

- Complete rejection classification: all 631 source-bound records in 55 groups
  distinguish 26 invalid sources, 79 resource restrictions and 526 open requirements.
  Input and final-report checks enforce coverage and preserve unexecuted lowering;
  a missing resolved-report classification mutation is detected. Existing fixture
  sources, diagnostics and 41 curated oracle opt-ins remain unchanged. This inventory
  does not implement its open requirements or establish whole-language conformance.

- Direct top-level numeric helpers retain the public entry ABI and use verified
  function IDs/signatures and captured arguments. Thirty-nine paired modules and
  51 new rejections cover arities, shadows, independent parameters/locals, finite
  recursion and global budgets. No old rejection is promoted. A reversed-argument
  mutation is detected; the Wasmtime-only deep-call test observes fuel exhaustion
  and instance reuse without claiming stack-overflow observation. The expanded
  source-bound inventory has 682 records in 57 groups: 26 invalid sources, 85
  resource restrictions and 571 open requirements.

- Number remainder and remainder assignment use an original exact private helper,
  preserving ordered snapshots and the numeric ABI. Sixteen exact rejections are
  promoted and 24 pairs add effects/selection/loop/wrapper/call composition evidence.
  A production kernel checks 23,136 raw binary64 pairs across Node, BigInt and both
  Wasm engines. Three arithmetic/order mutants are detected; a finite Wasmtime-only
  workload demonstrates fuel exhaustion and instance reuse. Remaining inventory:
  666 records, 56 groups, 26 invalid sources, 85 resource restrictions, 555 open
  cases and 40 curated references. The two operator-list diagnostic additions are
  explicit; no whole-language, full-upstream or performance claim follows.

- TypeScript-wrapped numeric write targets reuse original semantic references and
  the existing assignment/update paths. Fourteen exact source promotions and 37
  extra pairs cover snapshots, all supported compounds, scopes and source depth.
  Three finite depth/binding/order mutants are detected. No checker, coercion, IR
  or ABI expansion follows. At that step, the inventory contained 685 records in 56 groups, with 26
  invalid sources, 87 resource restrictions, 572 open requirements and 39 references.

- Numeric Boolean complement admits four exact old sources and 36 extra pairs.
  An explicit Predicate-to-Number bridge preserves the numeric unary contract;
  original operands run once and retained wrapper depth counts normally. Four
  finite conversion/classification/evaluation/depth mutants are detected. General
  Boolean values remain outside admission: 750 classified rejections in 56 groups
  include 26 invalid sources, 88 resource limits and 636 open requirements; the
  Boolean-values group retains 82 records and all 39 curated references remain.

Use [the research roadmap](roadmap.md) for rationale, source references, licensing,
quirk flags, browser scope and portable/module boundaries. Update this plan after
each demonstrated unit; do not replace evidence with a checked milestone box.

The [external testing plan](testing.md) registers Test262, TypeScript tests, the
Computer Language Benchmarks Game, JetStream, Node.js tests and Speedometer.
Pinned targets, concrete prerequisites and selected execution results are included
in compatibility reports. Registration does not complete C2, P1, A1 or B1.

The Number-only pristine Math.abs intrinsic adds 29 paired programs and 35 boundary
rejections, promoting exactly two existing sources. Its production kernel agrees
with pristine Node across 4,109 binary64 inputs in both engines. Mutable realm
objects, coercion and other builtins remain open; this does not settle L4.

Pristine Number sqrt/floor/ceil/trunc extend the fixed-opcode Math family. Forty-nine
pairs and 70 new boundary rejections preserve effects and binding identity; one
ordinary floor rejection is promoted. Per-method raw kernels and six sensitivity
probes cover direct Wasm emission without adding helpers, coercion or realm objects.

Discarded Number/Boolean evaluation adds 45 paired programs and 30 boundary
rejections, promoting exactly 13 ordinary sources. It reuses existing typed
predicate lowering for unobserved results; Boolean locals, arguments and returns
remain outside the Number ABI. All 39 curated gap references remain.

R1 additionally demonstrates private owned-resource roundtrip and explicit guest
destruction, repeated borrow control, stale owned-index replay and duplicate guest
drop. Five finite tests distinguish host backing lifetime from guest handle-table
entries and destructor calls. The original six borrow/policy tests stay unchanged;
no public grant API, resource-table policy or post-trap cleanup guarantee follows.

## Immutable Boolean predicate locals

Corpus 42 adds 44 pairs (four original promotions) and 34 boundary rejections.
Immutable Boolean source cells use the existing Predicate slot foundation, with
SymbolId/TDZ, initializer effects, aliasing and cumulative source budgets preserved.
This completes the single `gap-boolean-value` reference, not general Boolean values
or all L4 work. That run recorded 1008 pairs, 797 rejections and 38 curated references.
See [the exact source boundary](compiler.md#immutable-boolean-predicate-locals).

Primitive-Boolean unary plus/minus reuse the existing predicate-to-Number bridge
for the same bounded operand shapes as `~`. Forty-one pairs and 18 new negatives
cover exact signed zero, once-only effects and existing admission/resource limits.
No old rejection or curated reference is promoted; numeric source storage, ABI,
whitelist, classifier and runtime remain unchanged. That unary run recorded 1049 pairs,
815 rejections and 38 curated references.

## Inline Boolean arithmetic

Corpus 43 adds 66 pairs and 30 boundary negatives for the five primitive arithmetic
operators. Exactly six ordinary rejections are promoted and three Rust-only bodies
move to positive checks. No curated reference graduates. This remains a bounded
Number-result profile, not general value coercion: that step left 839 classified rejections and
38 curated requirements remain. See [the contract](compiler.md#inline-boolean-arithmetic-operands).

## Unary Predicate binding reads

Corpus 44 admits initialized immutable Predicate bindings only through unary
`+`, `-` and `~`, with semantic SymbolId/slot evidence and original wrapper depth.
Forty-three pairs include three exact ordinary promotions; 25 classified negatives
retain scope, binding and other value-position boundaries. No curated reference,
public ABI, representation or binary-operator policy changes. Selected totals:
1158 pairs, 861 rejections and 38 curated references, with 56 inventory groups
(26 invalid, 114 resource and 721 open requirements).
