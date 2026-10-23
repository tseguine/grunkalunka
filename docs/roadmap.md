# Research roadmap

The project's direction is clear; implementation choices should earn their place
through small experiments. This is a sequence of questions and exit criteria, not
a promise that any prototype will become the architecture.

## Direction

- Wasm-first, Rust-based, capability-oriented components and applications.
- Wasmtime initially; WIT/Component Model interfaces at component boundaries.
- TypeScript-first authoring with vanilla JavaScript compatibility as a goal.
- Browser, Node-compatible, and desktop environments are applications on top.
- Familiar Node/Bun build workflows for application authors.

Type annotations cannot by themselves justify changing observable JavaScript
behavior. A narrow lowering experiment may reject features it cannot implement;
that experimental restriction is not a decision to make the platform a TS subset.

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
the failure. `npm run build:spike` is wired to build the current experiment, not a
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

No tsrs or Wasmtime integration has been executed yet. WIT interfaces and the
Component Model are direction, not a claim that arbitrary JS objects, exceptions,
promises or module graphs already map to component types.

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

No production IR, object/GC strategy, scheduler, package
distribution scheme, or permanent capability API is settled by this roadmap.

## Node-compatible environment: conformance by observation

Use automated black-box comparison against a pinned Node runtime as the primary
compatibility method, as requested by the user. This concerns the environment
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
