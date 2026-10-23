# Research roadmap

The project's direction is clear; implementation choices should earn their place
through small experiments. This is a sequence of questions and exit criteria, not
a promise that any prototype will become the architecture.

The [external testing plan](testing.md) tracks all six adopted conformance,
reference and performance suites, their pinned sources and execution prerequisites.

## Direction

Product framing: a web application framework that makes building, running,
debugging, packaging and shipping applications straightforward. The platform and compiler
serve that developer experience; they are not the product pitch by themselves.

- Wasm-first, Rust-based, capability-oriented components and applications.
- Wasmtime initially; WIT/Component Model interfaces at component boundaries.
- TypeScript-first authoring with vanilla JavaScript compatibility as a goal.
- Browser, Node-compatible, and desktop environments are applications on top.
- Familiar Node/Bun build workflows for application authors.
- Portability across hosts and environments wherever reasonably practical.

Type annotations cannot by themselves justify changing observable JavaScript
behavior. A narrow lowering experiment may reject features it cannot implement;
that experimental restriction is not a decision to make the platform a TS subset.

Use portable Wasm artifacts and WIT interfaces as the application boundary. Keep
OS-specific paths, process behavior, UI integration and services behind explicit
host capabilities, with platform differences and unsupported grants documented.
Capability profiles should describe authority without unnecessarily tying an
application to one host. Native implementations may vary behind those interfaces.
Do not make a runtime-specific precompiled cache the sole distributable artifact;
retain portable Wasm and treat host caches as derived outputs. Avoid a single-host
assumption unless evidence shows a worthwhile tradeoff. Verify other OS/architecture
combinations before claiming them; the current PoC has two-engine coverage on
Windows, not demonstrated cross-platform tooling support.

## Build support

Cargo builds the Rust implementation. A small package-facing wrapper around the
compiler executable is the initial approach to explore for application builds:
ordinary `package.json` scripts, usable from Node/npm or Bun. Source contributors
need Rust; the intended packaged compiler should not require users to install it.
Start with subprocess invocation rather than a native JS addon or long-lived daemon.
Revisit only if measurements or editor integration justify the extra coupling.

The repository already has a dependency-free check entry point:

```sh
# Once, to acquire the locked Rust dependencies:
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo fetch --locked
# Verified from the repository root:
node scripts/check.mjs
# Package-script wiring (local npm launch currently fails):
npm run check
# Intended Bun paths (not yet exercised on this machine):
bun run --bun check
# Or directly:
bun scripts/check.mjs
```

The shared runner uses Node-compatible standard APIs and propagates child failures.
Direct `node scripts/check.mjs` is verified here. The local npm launch cannot find
Node in its child shell (root cause unconfirmed), so `npm run check` failed before
entering the runner; the [playback](../spikes/001-frontend-semantics/README.md) records
the failure. `npm run build:spike` is wired to build the first frontend experiment, not a
Grunkalunka compiler; that npm entry has not been verified either.
No JS package installation, bundler, workspaces framework, or JS lockfile is needed
for these scripts. The Cargo lockfile is committed with the experiment.

Before shipping the package wrapper, verify Node and Bun on Windows/macOS/Linux,
including paths with spaces, arguments, diagnostics, exit status, cancellation,
platform binary selection, and offline operation after installation. Pick and test
explicit maintained runtime versions then; today's Node version is evidence for
this probe, not a release support policy. ESM/CJS package resolution, npm dependency
compatibility, and Node API runtime compatibility are separate questions.

Sources: [Node subprocess API](https://nodejs.org/api/child_process.html) and
[Bun's compatibility matrix](https://bun.sh/docs/runtime/nodejs-compat).

## Experiments

| Order | Question and bounded experiment | Evidence required to move on |
| --- | --- | --- |
| 1 | Can Oxc supply inspectable lexical binding and diagnostic information for paired JS/TS? | Exact shadowing/closure targets, unresolved globals, both diagnostic stages, type-checking counterexample. [Playback](../spikes/001-frontend-semantics/README.md). |
| 2 | How can tsrs type information be associated with the source consumed by lowering? | Pin a tsrs revision; inspect its Rust API; check the same valid/invalid TS and JS fixtures; relate spans, symbols, and value/type namespaces across both representations. Demonstrate one type query reaching one source expression. Compare dual parsing against using tsrs's frontend alone. Report unsupported APIs and coupling. |
| 3 | Can one precisely admitted numeric function lower to core Wasm while preserving its JS result? | One exported scalar function, parameters, literals, arithmetic and return; explicit rejection of everything outside admission. Run emitted bytes in Wasmtime and compare against JS including signed zero, NaN, infinity, rounding, and evaluation order. Include emitted size, validation and rejected input diagnostics. This temporary admission is not the language contract. |
| 4 | Can an explicit WIT resource give useful authority without ambient access? | A Rust Wasmtime component host with a narrow in-memory capability, initially no WASI links. Observe missing import, granted handle, cross-instance isolation, forged/stale handle and revocation behavior. Document ownership/drop and alias rules. Add fuel/epoch interruption, memory limits, and a bounded host-call test separately. |
| 5 | What costs do the viable frontend/runtime choices impose? | Repeatable measurements of empty host, trivial core module, component, and a meaningful small app. Separate frontend/check/lower time, Wasm validation/compile, instantiate, first call, steady state, host calls, executable/artifact size and process memory. |
| 6 | What is the smallest useful environment application? | Select one vertical slice from earlier evidence; exercise it through WIT capabilities. Defer broad browser/Node/desktop compatibility layers until this seam works. |

Do not batch all experiments into an infrastructure project. Each should end with
replayable evidence, discarded alternatives, limits, and a next recommendation.

The [numeric PoC](compiler.md) now exercises the pure
numeric part of question 3 before question 2: its explicit two-number ABI makes it
independent of the unresolved tsrs bridge. It emits real Wasm from JS/TS and passes
1,351,812 engine-to-JS comparisons across Wasmtime and Node-Wasm, plus 528 rejected
inputs, including numeric locals, comparison-controlled ternaries, numeric
assignment/update, lexical blocks, statement branches, early returns, numeric truthiness
and while/for/do loops with nearest-loop unlabelled break/continue (L1/L2). Number-only `==`/`!=` predicates share strict
numeric equality semantics; coercion remains outside admission. Numeric unary `~`
and binary `&`, `|` and `^` use signed 32-bit truncation/wrapping semantics and
return Numbers. Both binary operands run eagerly in source order; shifts and
bitwise compound assignments remain outside admission. Numeric expression
statements preserve effects while discarding their Number result. Empty statements
fall through without IR allocation while consuming the ordinary source budget. TypeScript expression and predicate wrappers erase while
their runtime operands remain subject to the numeric admission. Type checking and
type-only declarations remain separate work. Unresolved `Infinity` and `NaN` reads
emit numeric constants; lexical shadows and TDZ retain ordinary binding rules,
without adding host access or support for other globals. It preserves AST grouping and observes
local mutation, statement/operand evaluation order and selected-branch effects. External effects
and coercive values remain unsupported. Recognized module/function directive prologues erase with source statement charges;
initial Oxc-recognized hashbang comments erase without source statement charges,
while all original bytes count toward the size limit. General strings remain outside admission. This is an executed fragment, not adoption
of a restricted platform language or a completed compiler architecture.

## Frontend findings as of 2026-10-23

Oxc 0.155.0 provides a parser and lexical semantic information. The first spike
observes exact captured/shadowed bindings and rejection diagnostics. It does not
perform TS type checking: `const x: number = "not a number"` has no diagnostic.
The scope/symbol data is promising input to lowering, not a complete JS runtime model.

The inspected tsrs workspace has its own AST, scanner, parser, binder and checker
crates. It must not be assumed to consume Oxc's AST or share its symbol IDs. The
bridge experiment must also account for different parse options, source offsets,
generated nodes, module graphs, declaration merging, and upstream API churn.
Keep original source identity through any transforms; a span match alone is not
proof of equivalent semantics. Compare an Oxc+tsrs path against a tsrs-only path
before building a general bridge or stable intermediate representation.

Source snapshots inspected (moving upstreams are deliberately pinned here):

- [Oxc semantic example](https://github.com/oxc-project/oxc/blob/0e6498f7e218420153910074d5a55bbef7c1c1d4/crates/oxc_semantic/examples/semantic.rs)
  and [workspace](https://github.com/oxc-project/oxc/blob/0e6498f7e218420153910074d5a55bbef7c1c1d4/Cargo.toml): Rust 1.97 minimum, MIT.
- [tsrs workspace](https://github.com/oxc-project/tsrs/blob/736a0e189939278c9878e5a3454f8126f55081fc/Cargo.toml)
  and [README](https://github.com/oxc-project/tsrs/blob/736a0e189939278c9878e5a3454f8126f55081fc/README.md): Rust 1.99 minimum, Apache-2.0 derivative of TypeScript with attribution obligations.
- [Wasmtime security model](https://docs.wasmtime.dev/security.html): external
  effects pass through linked interfaces; the host's imports determine authority.

Wasmtime core-module execution is exercised by the numeric PoC. The separate
[WIT resource experiment](capabilities.md) now demonstrates component profiles,
grant lifecycle and specific handle misuse, with independent fuel, linear-memory
and expired-epoch observations. A finite guest also traps after a host callback
advances the epoch during its invocation. A separate finite host algorithm enforces
an item-processing quota and reports typed exhaustion before an unfunded item.
The tsrs type-query experiment has not been executed.
These results do not claim that arbitrary JS objects, exceptions, promises or
module graphs already map to component types, or that the JS compiler emits
components yet.

## Runtime and performance questions to preserve

- Decide JS value representation, strings, objects/prototypes, closures, exceptions,
  dynamic features and async/event-loop behavior from representative workloads.
  Static TS types do not remove runtime coercions or reflective behavior.
- Treat unresolved identifier scanning as discovery. Aliases, dynamic property
  access and `eval` mean it cannot enforce capabilities. Specify and test authority
  transfer, attenuation, revocation and cross-component ownership at the host boundary.
- Fuel and memory limits do not alone bound blocking host calls or all host memory.
  Cancellation, deadlines and per-resource quotas need their own observations.
- Performance reports must include compiler/runtime versions, target, build flags,
  feature sets, cold/warm cache state, workload sizes, repetitions and dispersion.
  Compare equivalent work against Node and Bun when installed. Keep correctness
  checks outside timed loops. A debug frontend probe or empty component cannot
  establish production speed, memory use, startup cost or platform footprint.

The numeric PoC uses an internal [verified region IR](ir.md). A full-language
production IR, object/GC strategy, scheduler, package distribution scheme, and
permanent capability API remain unsettled by this roadmap.

Dependency policy: keep our source MIT and avoid GPL/LGPL terms, checking
transitives as well as direct dependencies. Where an explicit permissive alternative
exists, record that choice. [Current metadata screening](dependencies.md)
is reproducible but does not replace notices and bundled-material review before
binary distribution.

## Browser environment scope

The browser environment should serve both an embeddable web application runtime
in the Ultralight/Electron category and a real, desirable standalone browser.
**The primary difference between embedded and standalone use is the capability
profile**, rather than separate environment engines
or hard-coded product modes. An embedded slice is a practical early demonstration,
not a separate architecture that must later be replaced to make a browser.

Keep embedding and browser-shell concerns separable from the platform core. Future
experiments should expose navigation/lifecycle, origins and isolation, permissions,
network policy, persistent storage, input and accessibility as deliberate boundaries.
Use capability profiles to express permitted navigation, creation of browsing
contexts/windows, network/storage authority and host integration. A profile is
selected by the trusted embedding host or user-controlled browser configuration;
guest content cannot promote itself to broader authority. UI/shell choices can
vary around that same environment. Single-application embedding assumptions should
not quietly prevent independent contexts, untrusted sites or browser user controls.

The renderer, rendering architecture, compatibility target, UI toolkit and full
browser feature set remain unselected. Profile composition, individual grants and
defaults still need concrete experiments. Start with a useful embedded application
slice once the platform seams can support it, and demonstrate that changing the
profile changes authority without replacing the environment implementation.

### Reusable implementation pieces

Maximize reasonable reuse and modularity across the project. If HTML/CSS functionality
is implemented here rather than supplied by an existing library, it should be usable
independently of the browser environment and its shell. Candidate boundaries include
HTML parsing/document representation, CSS parsing, style calculation and layout;
their actual APIs and division should follow demonstrated consumers and coupling.

Keep document access, resource resolution, viewport information, font metrics and
rendering services explicit where needed. Avoid quietly reaching through browser
globals or its event loop. A module may depend on other libraries and contracts;
standalone use should be possible by supplying those dependencies deliberately.
Prove useful reuse with a small second consumer (for example, a parser tool or
headless layout probe) as each piece emerges, without building speculative plugin
frameworks or stabilizing public APIs before there is evidence for their shape.

### Reusable file formats and implementation references

Format support should be usable as standalone libraries with explicit source
inputs and outputs. Resource references remain data until an authorized consumer
resolves them. Parsing, document semantics, rendering and source-preserving editing
are separate responsibilities. [Markdown inspection](markdown.md) is the first
maintained adapter: headings, links and images for application indexing, with no
renderer or resource resolver.

Evaluate existing parsers before implementing new ones: html5ever for HTML,
Lightning CSS/cssparser for CSS, quick-xml for XML-family syntax, and kdl for KDL.
Inspect each selected version, features, licenses and resolved dependencies before
adoption. Test native and Wasm portability rather than inferring it from a Rust API.
XML parsing alone does not provide SVG rendering or XAML object/binding semantics.
KDL/SDLang alternatives still require the explicit document mapping and round-trip
fidelity contract described in the milestones. Avoid a general format framework
until multiple concrete consumers demonstrate a shared boundary.

Selected WebKit implementations are also candidates for reuse or Rust ports.
WebKit's [licensing page](https://webkit.org/licensing-webkit/) identifies both LGPL
and BSD portions; inspect the exact files and helpers rather than assigning one
license to the whole project. For example, the inspected
[HTML tokenizer](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/html/parser/HTMLTokenizer.cpp)
has a BSD-style notice. Pin source revisions and preserve applicable notices for
incorporated or translated code. A language port does not remove those terms.
Keep the GPL/LGPL exclusion and compare coupling/cost against existing Rust
libraries before choosing a port. No WebKit source has been incorporated.

### Browser conformance evidence and feature roadmap

Use complementary sources:

| Source | Role |
| --- | --- |
| [Test262](https://github.com/tc39/test262) | ECMAScript language/runtime conformance. |
| [Web Platform Tests](https://web-platform-tests.org/) | Browser API, integration and rendering behavior, mapped to the standards being implemented. |
| [Electron specs](https://github.com/electron/electron/tree/main/spec) | Candidate desktop/embedding behavior tests and implementation references for the environment layer: application lifecycle, windows, navigation and IPC. Select relevant cases; this is not a commitment to clone every Electron API. |
| [Acid1/2/3](https://www.acidtests.org/) | Historical integration/visual regression checkpoints; pin test versions and record intentional differences. |
| [Can I Use](https://caniuse.com/ciu/about) | Feature inventory and roadmap input; connect chosen features to standards, tests, capability requirements and representative applications. |

The [Acid maintainers](https://www.acidtests.org/) state that the suite is no longer
maintained and parts of Acid3 no longer reflect current standards consensus. Use
it as historical evidence, not a current certification gate. Can I Use's own
[support definition](https://caniuse.com/ciu/about) describes basic usable support,
not exhaustive conformance. Treat its feature map as planning evidence and record
our own measured support; do not substitute another browser's support label for tests.

Pin imported test/data versions and inspect applicable licenses before copying or
redistributing them. Can I Use's [data repository](https://github.com/Fyrd/caniuse)
states CC BY 4.0 and requests attribution to caniuse.com. A reference link alone
does not require importing the corpus. No browser test corpus or support dataset
has been vendored or executed at this stage.

Electron's [MIT license](https://github.com/electron/electron/blob/main/LICENSE)
permits reuse with its copyright and permission notices retained. Before adapting
tests or code, pin a revision and inspect the selected files, helpers, fixtures,
assets and dependencies for applicable terms. Keep provenance and modifications
beside incorporated material. The [testing guide](https://www.electronjs.org/docs/latest/development/testing)
describes an Electron application test suite; inspected
[app](https://github.com/electron/electron/blob/main/spec/api-app.spec.ts) and
[window](https://github.com/electron/electron/blob/main/spec/api-browser-window.spec.ts)
specs depend on Electron APIs, Node helpers, fixture apps and platform conditions.
Adapt bounded behavior tests as the corresponding environment APIs emerge,
recording platform assumptions and capability grants/denials. Review code reuse
for dependencies and architectural fit separately. No Electron material has been
vendored or executed here.

## Node-compatible environment: conformance by observation

Use automated black-box comparison against a pinned Node runtime as the primary
compatibility method. This concerns the environment
application's behavior, independently of Node/Bun support for build tooling.
The target is drop-in compatibility for the Node environment, including deliberate
emulation of security-uncritical bugs and quirks where compatibility requires it.

For each supported surface, run the same fixture under Node and the environment,
capture structured results, and compare values, error types/codes, output, and
observable event order. Start with deterministic modules, buffers/strings, timers
and events, then filesystem/process behavior in isolated temporary fixtures with
explicit grants. Keep expected failures and edge cases beside happy paths. Record
the Node release, platform, relevant flags, and capability set for every result.
Repeat scheduling-sensitive cases; test only ordering guaranteed by the chosen
contract rather than asserting identical wall-clock timing or incidental races.

Consult official docs and inspect source to explain gaps where appropriate to
the applicable licenses. Record source/version/license provenance, and inspect
the license of any material before incorporating it; black-box observations do
not themselves authorize copying implementations or upstream test suites. The
project remains MIT; dependencies and incorporated material keep their applicable
terms and required notices. Do not infer all third-party material shares one license.

Classify differences as an implementation defect, unsupported surface, platform
variation, reference-version difference, or deliberate capability restriction.
Test Node-equivalent behavior when authority is granted, and separately test denial
without the grant. This makes intentional restrictions visible instead of either
quietly granting ambient access or claiming unrestricted Node compatibility.

Every deliberately emulated bug/quirk must be individually enableable and
disableable by a flag or configuration setting. Retain a minimal reference
reproducer, affected Node versions, reason for emulation, provenance, and tests
for both enabled and disabled behavior. A quirk disabled on purpose is a selected
deviation, not evidence of drop-in behavior for that configuration.

A proposed Node compatibility profile can enable the relevant quirks together;
per-quirk overrides allow applications to opt out. Ordinary Grunkalunka applications
should not be forced to inherit unreasonable legacy behavior. Exact setting names,
precedence, defaults and profile/version selection will be specified with the first
concrete case. Do not emulate security-critical bugs or allow a compatibility switch
to bypass capability enforcement. Reassess a quirk if evidence changes its security
classification. Keep fixed-upstream behavior and supported reference versions visible
rather than silently changing an application's selected behavior on upgrade.

Keep a small supported-surface matrix with fixture IDs and observed gaps as each
slice is implemented. No compatibility percentage or blanket claim follows from
the first frontend probe; it does not execute a Node-compatible environment yet.
