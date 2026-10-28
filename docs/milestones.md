# Spike/PoC milestones

This is the active execution plan. Each milestone ends in a runnable demonstration,
measured compatibility evidence, and an updated status. A rejected input is not a
supported feature; an unattempted test is not a passing test.

## Execution scope and quality

Use small coherent commits, meaningful tests, proportional review and clear
limitations. Each experiment should resolve a concrete engineering uncertainty
before broader implementation depends on its result.

The initial L1/L2/C1/R1 horizon has been demonstrated; its measurements below are
historical evidence, not the current work queue. The compiler/runtime stream now
extends L3/L4/L5 in small verified units, prioritizing async and closures while
extending the [existing structured IR](ir-course-correction.md). Initialized local
function pairs, represented primitive arguments/results, captures and limited
[async catch/source throw](async-catch.md) are implemented; these do not establish
general functions, synchronous exceptions, `finally` or Promise-object support.

Primitive input inference through validated awaited `Promise.resolve`/`Promise.reject`
callback arguments now has native JS, Node Wasm and Wasmtime result/effect/job-trace
coverage, without a prior successful call establishing the kind. Private Number
and General scalar input interfaces now join across source-known constructors,
with result/capture/downstream-call facts kept consistent. JS/TS differential
tests cover named and immediately invoked selections, aliases, transitive groups,
forwarded/factory results and async snapshots while preserving existing
Number-only selections. Selected closures now also join Number and General scalar
result interfaces, preserving primitive tags, Number bits and source evaluation
order in both engines. Source-local Number proofs remain independent of the
private tagged result interface. General function-valued result joins and
unrepresented String/object results remain separate work.
Nested factory constructor groups now follow complete returns even for unused
definitions, retaining tagged leaf results, returned aliases, captured cells and
async caller snapshots within the existing signature-query bounds.
Lone unused factories now infer their returned constructor groups too, without
requiring a call or another factory peer. Differential tests preserve dormant
effects, generic identities, primitive leaf kinds and async reaction order.
[Fulfilled Promise locals](fulfilled-promise-locals.md) snapshot primitive payloads
at construction and preserve aliases, mutable captured cells and repeated await
jobs. A typed iterative cursor supports assignment, same-kind selection, object
truthiness and nested fulfilled adoption. The internal type remains distinct
from its primitive completion;
general Promise identity, rejected/unsettled values and escaping objects remain
separate requirements.
Represented `Promise.resolve`/`Promise.reject` intrinsics now default missing
payloads to Undefined and preserve ordered surplus-argument effects, snapshots
and suspension before the outer reaction. Stored fulfilled constructors share
these rules; pending async-call payloads with surplus arguments remain separate.
Known synchronous local arrows now return fulfilled Promise snapshots through
concise or complete block bodies. Their calls add no job; the caller's await keeps
its reaction. Runtime-selected/reassigned closures, returned functions and supported
callbacks now return the same fulfilled snapshots through distinct logical
signatures and the existing physical pair. Mixed parameter/result
interfaces and async-return adoption remain open.
The private IR now supports fulfilled Promise parameters with sixteen-byte frame
entries and distinct logical callback/return signatures. Mixed-frame, captured-cell
and 1,000-parameter executable IR checks cover that representation. Source inference
and binding now admit homogeneous fulfilled Promise parameters through known/runtime
closures, supported callbacks/factories and known awaited async local calls. Source
tests preserve tags, snapshots, captures, effects and reaction jobs. Selected peers
now share demanded callback roles by formal position, including unused callback
formals; complete signatures and ordinary identity specialization remain checked.
Arbitrary Promise objects and mixed parameter domains remain unfinished.
Async arrows with admitted non-suspending primitive bodies now use these private
closure pairs and fulfilled results, including callbacks/factories and mutable
selection. Their prefix effects and caller reactions have differential coverage;
Promise-return adoption remains open; pending identity source values are described below.
Non-suspending source async closures can also throw primitive reasons through
the distinct settled-completion IR and one-reaction await. Runtime selection,
reassignment, factories and supported callback forwarding have source oracles;
rejected results are restricted to immediate await/synchronous forwarding.
Immediate rejected-snapshot storage/inputs, unhandled-rejection tracking and adoption
remain open.
Runtime-selected suspending async source bodies now use async-effect private signatures,
an awaited closure-call operation and backend invocation frames. IR tests execute
child suspension, caller/body catches, typed inputs, nested and recursive frame
restoration, and existing resource boundaries against native JS in both engines.
Maintained source oracles cover selection/reassignment, captures, callbacks,
returned factories, primitive/Promise inputs and lexical catches. Stored pending
calls now overlap and preserve their identity until later await. Adoption,
unhandled events and general synchronous exceptions still keep L5 incomplete.
Conditional/comma await operands now preserve that effect when selecting between
a suspending call, primitive value or fulfilled/settled snapshot. Source-site
metadata keeps one distributed source await within one logical budget site;
child sites and the separate 32-job runtime limit are unchanged. Homogeneous
pending logical values/storage now execute; mixed storage and adoption remain gaps.
An executable backend Promise record/reaction kernel now supplies pending state,
single primitive settlement and FIFO waiter delivery in the existing guest arena.
Native Promise microtask traces and Wasmtime/Node kernel tests cover it. Logical
Promise identities now connect eager async starts, independently suspended frames,
typed private storage and queued awaits to both adapters. Root settlement and
remaining jobs are independent. Source bindings, aliases, captured getters,
reassignment, homogeneous selection and later awaits now consume these records.
Detached primitive child rejection and subsequent handling now reach host
notifications. Homogeneous pending parameter inference and transport now cover
known calls, runtime-selected functions, higher-order callbacks, factories and
captured formals, preserving passed handles and reaction order. Synchronous
forwarders return the same identity. Async roots/private functions now adopt
returned pending identities through distinct registration and reaction jobs;
no-await constructors retain an async effect. Adopted rejection bypasses local
catches, returned-expression failures remain lexical, and sibling frame/handle
snapshots survive delayed registration. Self-resolution now creates a catchable
guest TypeError with stable identity through catches, rethrow and host notifications;
the saved native4/guestpending counterexample now returns4 in both Wasm hosts.
General relational IR now closes the saved Error `e<=e` counterexample, preserves
operand snapshots through calls/suspension, and distinguishes equal intrinsic
Error strings from object IDs and explicit numeric NaN coercion. General strings,
Promise/scalar unions and general Promise payloads remain gaps.
Represented Promise values now support strict/loose equality and inequality,
preserving alias identity, distinct calls, captured snapshots and operand jobs.
Comparison does not handle rejection or equate a Promise with its completion or
numeric ID. Immediate settled async results now also preserve Error payloads and
their fulfillment/rejection status through a disjoint backend status bit; direct
inline and previous primitive cases remain controls. A tested IR materialization
operation now creates fresh identities from fulfilled/settled snapshots without
adding jobs or handling rejection. Async source constructors now materialize once,
preserve aliases, support homogeneous private forwarding and adopt on async return.
Completed and pending results share identities. Default fuel/job limits stay fixed;
former source guards execute against native JS, Node-Wasm and Wasmtime, including
detached rejection checkpoints. General String coercion and general payloads remain open.
This does not close general Promise support or L5.
[Runtime closure contracts](runtime-closures.md) and the maintained
`async-suspension` tests supply existing behavior and guards to preserve.

General JSON-compatible source values and parse/stringify remain a broader
unfinished goal; current IR/closure work supports their eventual typed integration,
and the separate [String/JSON bridge](json.md) is not general object/array lowering.
The bounded [capability-backed Markdown demonstration](https://github.com/tseguine/grunkalunka/issues/27)
is complete: the same compiled guest inspects authorized in-memory documents
through WIT, returns native-parity outlines, and rejects ungranted document
substitution. [Integration evidence](document-outline-authority.md) and the
three-platform [final CI run](https://github.com/tseguine/grunkalunka/actions/runs/40505705440)
cover this experimental boundary, not a writable vault or an A1 application.
The document/runtime stream has since demonstrated bounded P1 cost, provenance
and comparison experiments (summarized below); browser work remains separate.
This status does not assign new application-integration work. The summary
also addresses the coordination finding
in [steering #37](https://github.com/tseguine/grunkalunka/issues/37) without
changing those streams' scope or the maintainer's priorities.

## Lowering track

| Milestone | Status | Demonstration / completion evidence |
| --- | --- | --- |
| L0 — numeric expressions | Done | JS/TS -> portable Wasm; Wasmtime + Node-Wasm differential tests including signed zero, NaN, infinities and rounding. [Compiler contract](compiler.md). |
| L1 — local values and decisions | Done | Numeric `const` locals and comparison-controlled ternaries; the L1 run had 31 JS/TS pairs, 76,508 engine comparisons and 64 rejected inputs. Bindings become available after their initializer. |
| L2 — mutable control flow | Demonstrated | Numeric mutation, lexical blocks, `if`/`else`, early returns, numeric truthiness and `while`/`for`/`do...while`; the initial L2 run had 72 JS/TS pairs, 177,696 comparisons and 100 rejections. NaN/±0 conditions, nested loops, per-entry initialization and selected branches agree with JS. A separate runtime test observes OutOfFuel and successful instance reuse. Numeric compound assignments, prefix/postfix updates and parenthesized write statements are now admitted. Unlabelled break/continue now target the nearest while/for/do loop; switch, labels and other control forms remain explicit compatibility gaps. |
| L3 — functions and modules | In progress | Numeric helpers preserve ordered argument snapshots and self/mutual recursion. Initialized local/runtime function pairs, returned closures and captured cells work in the [admitted scalar/callback domains](runtime-closures.md). General function forms, nested declarations and module graphs/initialization remain. |
| L4 — JS value semantics | In progress | Undefined, Null, Boolean and Number values and their admitted coercions have executable coverage. The separate [experimental String/JSON source profile](heap-literals.md) and [JSON guest operations](json.md) admit bounded literal-origin parse/stringify chains outside the shared General-value lowerer. General source strings, arrays, objects/prototypes and a general heap/GC strategy remain open. |
| L5 — exceptions, closures and async | In progress | Local/runtime closures preserve captured cells and scoped block returns. The async profile preserves represented primitives and internal function pairs across suspension. [Lexical async catch and primitive source throw](async-catch.md) have differential result/job-order tests. Immediate primitive async closures materialize stable Promise identities from compact completions; stored rejection, homogeneous forwarding and async-return adoption are supported. Conditional/comma/logical async returns now select primitive or represented Promise leaves before adoption, preserving lexical evaluation errors and selected reaction jobs. Equality can compare selected Promise/primitive operands before adoption without adding jobs or handling rejections. Const and let mixed locals, aliases and captured cells retain the raw kind until equality, logical tests, explicit await or async return consumes it. Simple and logical assignments preserve raw snapshots and selected RHS effects. General synchronous exceptions, recursive initialization, broader mixed-value storage/private interfaces, exported mixed pending/scalar input domains and general Promise/thenable values remain. Homogeneous pending inputs retain handles through known/runtime calls, callbacks, captures and synchronous forwarding. Represented Promise/General mixtures now pass through private parameters and typed argument frames, including callbacks, factories, captures and async consumption. Ordinary synchronous private returns now preserve the same raw carrier through concise/block bodies, implicit undefined, callbacks and forwarding; returning alone adds no adoption or handling. Detached primitive child rejections and later handling now use guest FIFO notifications, Node native checkpoint reporting and a consuming Wasmtime API; this does not complete arbitrary Promise objects or HTML event-loop policy. Async try/catch/finally now preserves pending completions through suspended cleanup, supports abrupt finalizer overrides, and defers return adoption until cleanup ends. No blanket compatibility claim from these slices. |
| T1 — tsrs integration | Research gate | Pin source; perform one real type query associated with an expression; compare tsrs-only against Oxc+tsrs representations. Type information must not erase observable JS behavior. |

L1/L2 retain an explicitly numeric ABI during the experiment. They expand a
demonstration, not the platform's final language contract. Boolean conditions may
be represented internally without changing the public two-number/result-number ABI.

## Compatibility, runtime and application tracks

| Milestone | Status | Demonstration / completion evidence |
| --- | --- | --- |
| W0 — maintained implementation workspace | Done | Main Cargo workspace at development version `0.0.1`: compiler, runtime, CLI, Markdown, JSON data/guest, bounded Wasm composition and experimental layout crates, with the regression corpus in `tests/compatibility`. Generated Wasm and external dependency resolution were unchanged by the original workspace promotion. Continue PoC iteration in maintained crates; reserve disposable spikes for unresolved questions. |
| C1 — executable compatibility matrix | Done | The [executable corpus](../tests/compatibility/corpus.mjs) and [rejection inventory](../tests/compatibility/rejection-inventory.json) define current fixtures; executed reports record actual counts and provenance. **Corpus 84 (2026-10-27 snapshot)** contained 1,949 numeric JS/TS pairs, 175 primitive-profile pairs, 849 rejected inputs across 54 inventory groups and five explicitly unexecuted feature groups. Selected [lowering-gap references](lowering-gaps.md) separately run expected JS results while verifying that lowering still rejects them. Successful rejection is not supported language behavior. Every admitted fixture compares JS to Node-Wasm and Wasmtime; each expected rejection verifies absence of an artifact. |
| C2 — external conformance selection | In progress | A pinned TypeScript arithmetic source supplies 32 adapted runtime cases and zero selected admission rejections, with explicit provenance and limitations. This does not run the TypeScript conformance suite. Four pinned Test262 numeric programs adapt 18 assertions with an independent success oracle and per-check failure probes. Full upstream execution still needs its original harness and broader semantics. Record exclusions rather than silently skipping them. Use WPT for later browser behavior; Acid as historical regression; Can I Use as feature-roadmap input. |
| R1 — WIT capability profiles | Demonstrated | [One WIT component](capabilities.md) under absent/denied/granted profiles; typed policy errors and backing-read counters distinguish authorization from guest handle validation. Revocation, aliases, store ownership and three specific guest handle misuse cases are observed. No WASI links. Independent core-module tests observe fuel exhaustion, per-memory growth rejection, expired-epoch trapping/renewal, epoch expiry triggered by a host callback during an active invocation, and a host algorithm enforcing an item-processing quota. These are separate controls, not a complete component budget. |
| D1 — capability-backed Markdown inspection | Demonstrated | Closed [issue #27](https://github.com/tseguine/grunkalunka/issues/27): the real [Markdown guest and WIT component](document-outline-component.md) consume only host-authorized in-memory documents, return typed owned outline lists and original ranges matching the native library, and distinguish policy errors from malformed data and traps. [Multi-document grant isolation](document-outline-authority.md) checks denial without backing reads, alias revocation, owner collisions and forged guest borrow indices. Final [PR #40](https://github.com/tseguine/grunkalunka/pull/40) passed complete [Linux/Windows/macOS CI](https://github.com/tseguine/grunkalunka/actions/runs/40505705440). No filesystem/vault authority, persistence, source-language imports or finished application is claimed. |
| P1 — portability and costs | In progress | [Evidence ledger and remaining gaps](p1-evidence.md): correctness-checked build/compile, WIT/component, Node/V8 and Wasmtime replay, Markdown workloads, source/binary provenance and same-process comparisons. [#76](https://github.com/tseguine/grunkalunka/pull/76) provides same-byte GCD replay; [#83](https://github.com/tseguine/grunkalunka/pull/83) executes the identical x64-produced bytes on native Linux ARM64 in Node/V8 and Wasmtime. [#84](https://github.com/tseguine/grunkalunka/pull/84) builds and tests the Markdown/WIT pipeline on native Linux ARM64, without claiming cross-host guest byte identity. [#77](https://github.com/tseguine/grunkalunka/pull/77) measures bounded repeated real compiler API calls, not isolated lower/IR/emit phases. Whole-process memory observations, scalar call controls and standalone Oxc phase timings remain separately characterized. **Broader ARM64 application portability, production-stage attribution, comparable multi-host measurements, guest-attributed memory and full application costs remain open.** |
| A1 — first useful framework app | Planned | A small application with straightforward build/run/debug/package flow, using prior component/capability evidence. Show the same application under two capability profiles. |
| B1 — reusable browser pieces | In progress (headless experiment) | A standalone Rust [block-box layout core](block-layout.md) from [#59](https://github.com/tseguine/grunkalunka/pull/59) and a tested [typed semantic-tree adapter](block-layout.md#implemented-minimal-semantic-tree-projection) from [#62](https://github.com/tseguine/grunkalunka/pull/62) produce bounded rectangles and inert text anchors without a browser shell. These are **not** HTML parsing, CSS cascade, text shaping, painting, scripting, or a fully integrated second environment. Compare libraries before adopting larger components. Embedded and standalone browser use primarily differ by capability profiles. |
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
  At that step, mixed-type and Boolean equality remained separate work.
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
public ABI, representation or binary-operator policy changes. That unary step recorded:
1158 pairs, 861 rejections and 38 curated references, with 56 inventory groups
(26 invalid, 114 resource and 721 open requirements).

## Arithmetic Predicate binding reads

Corpus 45 shares the unchanged unary binding eligibility with only five arithmetic
operand callers. Eighty-eight pairs include four exact ordinary promotions and
27 classified negatives. No curated reference, representation, runtime, ABI or
native-liveness policy changes. That arithmetic step recorded 1246 pairs, 884 rejections,
38 curated references and 56 groups (26 invalid, 121 resource, 737 open).

## Discarded immutable Predicate reads

Corpus 46 adds a leaf-only semantic eligibility alternative to the unchanged free
discard classifier. Fifty-three pairs include five exact ordinary promotions;
26 classified negatives retain root-join, scope and value-position boundaries.
One Rust guard transfers positive; all 38 curated references remain. Selected
counts: 1299 pairs, 905 rejections, 56 groups (26 invalid, 127 resource, 752 open).
No representation, runtime, ABI, budget or native-liveness policy changes.

## Strict same-kind Boolean equality

Corpus 47 adds 47 pairs and 21 classified negatives for predicate `===`/`!==`.
Two exact ordinary sources move; no curated reference or Rust-only guard transfers.
Both proven Boolean operands convert through existing NumberFromPredicate in source
order, then existing Compare; the Number fallback and classifiers remain unchanged.
That step recorded 1346 pairs, 924 rejections and 38 curated references in 56 groups
(26 invalid, 129 resource restrictions, 769 open requirements). Mixed kinds, loose
Boolean equality and general Boolean values remain separate work.

## Number logical assignment

Corpus 48 adds 64 pairs including two exact ordinary `&&=`/`||=` promotions, plus
28 classified negatives. Initialized mutable Number identifiers use existing
captured Load/Truthy/If/Store/Yield; selected RHS effects/store and both result
snapshots are covered. No curated reference, Rust-only guard, IR/runtime/ABI or
budget changes. That step recorded 1410 pairs, 950 rejections, 38 curated references
and 56 groups (26 invalid, 133 resource restrictions, 791 open requirements).
Nullish/general/const assignment admission remains separate; two retained unsupported
operator diagnostics advance only their whitelist and the group assessment records
the supported partial Number domain.

Declaration-level TypeScript module erasure admits `import type` and `export type`
for the four original Oxc module-item families. Four ordinary TS exclusions
promote with exact sources and explicit JS oracles; no curated reference transfers.
Each item retains one shared statement charge, original source bytes and semantic
binding identity. Runtime module edges and ambient declarations remain open.
Corpus 49 records 1442 pairs, 979 rejections and 38 curated references in 56 groups
(29 invalid, 141 resource restrictions, 809 open requirements).

Generic declaration metadata erasure promotes three ordinary TS sources with unchanged
numeric runtime behavior. Corpus 50 records 1466 pairs, 995 rejections and 38 curated
references (31 invalid sources, 144 resource restrictions, 820 open requirements).
Explicit type-argument calls and runtime TypeScript constructs remain open.


A maintained tagged primitive runtime boundary now distinguishes undefined, null,
booleans and numbers. Its typed result, import denial and fuel behavior are tested
independently. `compile_value` now lowers explicit four-kind primitive returns through
verified General snapshots and pair emission, with Number/Predicate specialization.
A separate finite suite compares 27 JS/TS pairs on 14 inputs with original JS in both
Wasm engines (1512 comparisons). The numeric corpus and its rejection counts remain
unchanged. The CLI now selects this profile through `compile-value`/`wat-value`/
`eval-value`. Its separate report executes 19 pairs on 14 inputs (1064 comparisons),
checks eight rejected pairs and five CLI control groups, and verifies four exact
original numeric rejection sources as value-profile admissions while retaining their
numeric rejection. General source cells/coercions/helper parameters, primitive
equality, implicit returns and heap-backed values remain open.

Primitive strict equality now compares the four represented primitive kinds in both
profiles, preserving scalar numeric paths and left-to-right operand snapshots.
Corpus 51 transfers eight original rejection observations, including the two
null/undefined curated references with unchanged fixed results, and adds ten paired
programs. It records 1484 pairs, 987 rejections and 36 curated references; 56 groups
retain 31 invalid, 144 resource and 812 open observations. Loose equality and General
storage/coercion remain open; this is a bounded L4 increment, not L4 completion.

Immutable General const locals now retain primitive kinds across initialization,
aliasing, mixed scalar/tagged layout, lexical blocks and repeated loop entries.
Corpus 52 records 1494 pairs, 986 rejections and 36 curated references (31 invalid,
144 resource, 811 open across 56 groups). One exact numeric source transfers to
admitted; two retained diagnostics advance to unsupported numeric reads. The separate
Value profile promotes `general-local`, reaching 20 pairs and seven selected
rejections. Mutable General cells and coercion are still separate L4 work.

General ToBoolean now admits the represented primitive kinds in conditions,
negation and predicate short-circuit composition, with captured operands and unchanged
scalar paths. Corpus 53 has 1510 pairs, 986 rejections and 36 curated references;
this increment adds sixteen pairs and promotes no numeric negative. The separate
Value profile promotes its original `general-condition` sources, reaching 21 pairs
and six selected rejected pairs. General-valued logical selection remains separate.

General-valued &&/|| now use captured General operands and selected If arms; new
discarded logical shapes use the same evaluation after existing scalar classifiers.
Corpus 54 has 1530 pairs, 982 rejections and 36 curated references (807 open,
144 resource, 31 invalid in 56 groups). Four original rejection observations of
three exact sources transfer; one retained diagnostic advances to a numeric read.
The Value profile records four additional exact numeric origins: 25 pairs, six
selected rejections and eight origins. Number-only logical Value artifacts retain
their scalar specialization. General logical assignment and ?? remain separate.

Write-aware mutable General locals now support later primitive kind changes and
plain assignment results, while preserving proven Number layouts and normal lexical
initialization. ADR 0003 records the conservative planning and node-index tradeoff.
Corpus 55 has 1554 pairs, 974 rejections and 36 curated references (799 open,
144 resource, 31 invalid across 56 groups): eight original observations transfer,
sixteen pairs are added, and two retained diagnostics advance to numeric reads.
The Value profile has 29 pairs, six selected rejections and eight numeric origins.
General parameter writes, compound/update/logical assignment and implicit lexical
initialization remain separate units; mutable language support is not complete.

Explicit four-kind ToNumber now feeds unary +/minus and binary +, -, *, /, %,
retaining specialized scalar paths and original operand snapshots. Corpus 56 has
1589 pairs, 955 rejections and 36 curated references (780 open, 144 resource,
31 invalid across 56 groups); nineteen original observations transfer and sixteen
pairs are added. Value reporting has 34 result pairs/four selected rejections,
plus separate admitted recursion/fuel observations that count no result comparisons.
Other conversion positions and mutable operations remain distinct follow-up units.

General source parameter plain writes now preserve the Number ABI through selected
General cells (ADR 0004). Number-only parameter artifacts remain specialized.
Corpus 57 has 1605 pairs and unchanged 955 rejections/36 curated references;
seven diagnostics advance without counting support in the numeric profile.
The Value profile has 42 pairs/four selected rejections/twelve exact numeric origins,
with the two termination-only observations still excluded from result comparisons.
Other write operators, General arguments and discarded General sequences remain
separate units; parameter/source mutation support is not complete.

Discarded represented primitives now include nullish literals and General-valued
sequence, conditional and call results. Corpus 58 has 1625 pairs, 951 rejections
and 36 curated references (776 open/144 resource/31 invalid); four exact original
observations transfer and sixteen pairs are added. Existing admitted artifacts
retain their scalar routes. Value reporting reaches 46 pairs/four selected
rejections/twelve numeric origins, with termination observations still separate.
The discarded-General sequence gap recorded during ADR 0004 is resolved; other
operators and write/argument contracts remain separate work.

## Loose equality of represented primitives

Loose `==` and `!=` support the four represented primitive kinds: undefined,
null, Boolean and Number. Both nullish kinds compare equal to each other; neither
compares equal to a Boolean or Number. Boolean/Number comparisons use the Boolean
numeric value. NaN remains unequal, and signed zeros compare equal. This follows
[ECMAScript IsLooselyEqual](https://tc39.es/ecma262/#sec-islooselyequal).
String, BigInt, Symbol, objects and the optional IsHTMLDDA behavior remain outside
this domain; the curated string-coercion equality gap remains rejected.

The typed `PrimitiveLooseEqual` operation consumes two General snapshots and
produces a Predicate. With canonical nullish payloads of +0 and Boolean payloads
of 0/1, emission compares the nullish/non-nullish partition and the numeric
payload. This formula depends on the four-kind invariant; adding another kind
requires revisiting it. Both operands evaluate once, left to right. Existing
positive Number comparisons and strict equality retain their previous routes.

Corpus 59 promotes 19 exact rejection observations (17 distinct JS sources),
paired with TypeScript companions, and adds effect and type-wrapper cases:
1660 pairs, 932 numeric rejections and 36 curated references. The 56 inventory
groups now contain 757 open requirements, 144 resource restrictions and 31 invalid
sources. No retained rejection diagnostic changes. Value reporting adds four
cases, reaching 50 pairs, four selected rejected pairs and 2800 result comparisons;
twelve numeric origins, six control groups and two termination-only observations
remain unchanged. These finite cases do not establish complete language support.

## Primitive numeric updates

Initialized mutable General locals and selected source parameters now support
prefix/postfix `++`/`--` over the four represented primitive kinds. Postfix returns
the converted old Number; both forms store the new Number. See
[the compiler contract](compiler.md#primitive-numeric-updates). Corpus 60 adds
20 pairs (1680 total); the existing 932 rejection records and all 36 curated
references remain unchanged. Value reporting reaches 54 pairs/3024 comparisons.
This expands behavior without claiming an existing inventory gap was closed.

## Implicit let initialization

Bare `let` initializes Undefined at declaration execution, preserving source order,
TDZ, loop initialization and existing explicit-initializer output. Corpus 61 adds
four exact-source promotions and sixteen other pairs: 1700 pairs/928 rejections.
The curated bare-let source remains numerically rejected at its General return,
and executes unchanged in the Value profile (59 pairs/3304 comparisons/thirteen
numeric origins). See [the contract and limits](compiler.md#implicit-let-initialization).

## Primitive arithmetic compound assignments

`+=`, `-=`, `*=`, `/=` and `%=` support four-kind primitive RHS values and mutable
Number/General identifier targets, with captured-left effect order and Number
store/results. Corpus 62 has 1721 pairs/923 rejections (five exact promotions);
Value reporting reaches 63 pairs/3528 comparisons. All curated references and
remaining diagnostics retain their previous outcomes. See
[the contract and exclusions](compiler.md#primitive-arithmetic-compound-assignments).

## Primitive relational comparisons

`<`, `<=`, `>` and `>=` use represented primitive numeric conversion, preserving
left-to-right snapshots and NaN-false behavior. Corpus 63 has 1739 pairs and 921
rejections, with two exact promotions and no retained diagnostic changes. Value
reporting reaches 67 pairs/3752 comparisons. All curated references remain. See
[the contract and exclusions](compiler.md#primitive-relational-comparisons).

## General logical assignment

Represented primitive `&&=`/`||=` supports mutable identifier cells, exact selected
results and skipped RHS effects, including kind-changing local/parameter writes.
Corpus 64 has 1759 pairs/921 numeric rejections; eight diagnostics advance without
numeric promotions. Value reporting reaches 73 pairs/4088 comparisons with two
additional unchanged numeric origins. `??` and `??=` remain separate units. See
[the contract and exclusions](compiler.md#general-logical-assignment).


Local arrow calls now specialize into typed regions with shared captured
cells, ordered argument snapshots and fresh parameter slots. Corpus 65 admits the
two original curated arrow/capture sources and twenty additional paired cases;
1781 pairs, 919 rejections and 34 curated gaps remain. L5 has this demonstrated
capture slice; escaping/recursive closures and async still require further work.
Numeric block-arrow bodies now support statements, loops and early returns through
a scoped call region, including in primitive and async callers. This extends L5's
local slice; function-value lifetime and general Promise semantics remain open.
Captured bindings now use a bounded guest-memory arena with stable addresses
across helper return and async suspension. This is an L5 storage prerequisite;
returnable function records and their invocation remain open. Corpus counts are
unchanged. See [captured cells](captured-cells.md).
L5 now includes known concise local factories returning closures into immutable
bindings, with fresh creator captures for new arrows and shared alias writes.
Calls specialize known targets; general function records and indirect invocation
remain open. See [returned closures](returned-closures.md).
L5 now includes an immutable synchronous runtime-function slice: typed
code/environment values, lifted bodies, selected targets, shared captures and
indirect numeric calls. Packed frames support the existing 1,000-argument bound.
Primitive async frames now retain selected function pairs and captures across
suspension, with lifted bodies separate from the dispatcher. Async closure bodies
and remaining first-class function semantics stay open;
see [runtime closures](runtime-closures.md).
L5 now includes initialized mutable function bindings with same-arity plain writes,
shared captured cells and alias/callee snapshots, including across async jobs.
Rebinding can form self/mutual recursive calls through existing initialized cells;
recursive initialization and general function arguments/results remain open.
L5 now includes numeric-leaf callback arguments to known local arrows/factories,
with source-symbol arity demands, forwarding, typed parameter cells, argument
snapshots and retained factory captures. General function results remain open;
see [local callback arguments](local-callback-arguments.md).

L5 now includes runtime-selected synchronous functions receiving numeric-leaf
callbacks and Numbers. Typed logical signatures, captured callback parameter cells,
ordered snapshots, async callers and wide logical argument frames are demonstrated;
see [higher-order callbacks](higher-order-callbacks.md). General recursive callback
signatures, factories returning factories and async callback bodies remain open.

L5 also includes existing function pairs returned by concise known local calls,
with identity, conditional selection, assignment and immediate invocation across
synchronous and async callers. General function signatures remain incomplete; see
[local function results](local-function-results.md).

L5 now handles primitive rejected awaits through lexical catch handlers,
including nested handlers, later awaits and retained catch bindings, with native
JS reaction-order comparisons. Finally and general Promise/exception
semantics remain open; this closes no numeric-profile rejection. See
[async catch](async-catch.md).

L5 also includes compatible-signature function results from known block-bodied
local calls,
with typed early returns through branches/loops, callback identity/selection,
retained captures and async caller suspension. General recursive function-result signatures
and exported function values remain separate requirements. See
[block function results](block-function-results.md).

Primitive source throw now transfers immediately to lexical async catch handlers
or rejects the activation directly, preserving represented reason kinds and adding
no reaction job. Prefix/after-await effects, nested rethrows and 32-job settlement
are checked against native JS. Synchronous/lifted throw, finally and general
exception values remain open; numeric rejection sources remain unchanged. See
[async throw](async-throw.md).

Runtime-selected concise synchronous factories now return numeric-leaf function
pairs, including fresh creator cells, captured aliases, ordered argument snapshots,
recursive rebinding and async callers. Result-aware logical signatures and a
private pair-return table call preserve the existing host ABI and wide argument
bounds. See [runtime function results](runtime-function-results.md). General recursive
callback/result signatures, nonnumeric leaf results and async bodies remain open.

L5 now includes truthiness of represented synchronous functions in Boolean contexts,
with expression effects preserved and tested function bodies left dormant. Numeric
and primitive result boundaries, async jobs and existing resource limits remain
intact; see [function truthiness](function-truthiness.md).

## Primitive binary coalescing

Binary `??` now preserves represented primitive kinds and payloads, with RHS effects
and async jobs only for Null/Undefined left operands. Number-only results and Boolean
contexts retain their profile rules. Corpus 66 promotes three original numeric
rejections; the Value profile has 82 pairs/4592 comparisons and also executes the
original curated `gap-nullish` source. Its selected result is always a Number, but
the compiler retains a General join; narrowing it for the numeric profile remains
separate work. Constructor-free closure selectors using `??` now specialize successfully.
`??=` and `void` remain separate units. See [the contract](compiler.md#binary-nullish-coalescing).

## Represented numeric coercion (2026-10-28)

The async identity profile now supports numeric-only coercion of represented
Promise/General operands in unary operations, non-additive arithmetic/bitwise/
shifts, existing unary Math first arguments, and non-additive compound RHS values
on primitive cells. Six maintained differential groups execute native JS,
Node-Wasm and Wasmtime with ordering, mixed private calls/captures, erased types,
source limits and notification checks. Thirteen exact old guard assertions (twelve
unique sources) are promoted to execution; detached rejections retain an owned
native notification checkpoint. See [the contract](async-number-coercion.md).
Mixed addition, mixed-target compound/update writes, broader objects,
Strings/BigInts, thenables and general synchronous exceptions remain gaps.

## Represented Promise relational operands (2026-10-28)

The async identity profile now supports all four relational operators on raw
Promise/General operands, including selected locals, mixed private calls and
captures. Six maintained differential groups compare native JS, Node-Wasm and
Wasmtime with builtin string ordering, numeric NaN cases, ordered snapshots,
suspension/failure, erased types and source budgets. Three exact old guards now
execute unchanged, including the guard added with numeric conversion. Dedicated notification fixtures check rejection identity,
late handling and unchanged job counts. This extends the existing relational
operation and verifier; no new representation or host protocol. See
[the contract and exclusions](async-promise-relational.md). Broader object/String
conversion, other coercion contexts and general JS/TS remain separate gaps.
