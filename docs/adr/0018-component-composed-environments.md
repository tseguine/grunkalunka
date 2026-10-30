# ADR 0018: Component-composed application and browser environments

Date: 2026-10-30

Status: proposed

## Context

Grunkalunka aims to be a Wasm-first application framework whose browser, embedded-web, desktop and headless environments reuse a Rust-based implementation. The maintainer's design discussion identified a risk: a first application built with private Node/Rust shims could succeed without establishing reusable framework services or showing that JS/TS authors can access functionality offered to Rust authors.

Inspection baseline: `master` `39d37b811a` (2026-10-30). The actual A1 note preview already combines a WIT-authorized Markdown component, a separately compiled import-free JS numeric selector, a private Node host/scene protocol and native Rust layout/RGB painting; the bundle runs independently of the source checkout on the packaging host. [A1 evidence](../a1-note-preview.md) and [steering #52](https://github.com/tseguine/grunkalunka/issues/52) distinguish this from an interactive application or a public cross-language UI API. The [compiled-capability experiment](../compiled-capability.md) has a handwritten component adapter around generated numeric Wasm, not source-lowered WIT imports. [Compiler admission](../compiler.md) still rejects runtime source imports. The [headless renderer](../block-layout.md) owns box geometry and inert text anchors but does not rasterize actual text glyphs or deliver OS input.

[ADR 0012](0012-guest-runtime-and-host-capabilities.md) proposes guest-owned language semantics and WIT contracts at actual capability/service boundaries; [ADR 0013](0013-async-job-and-activation-ownership.md) separates guest async activations from host scheduling. This ADR records the broader *proposed composition strategy*, not adoption of new runtime, plugin, thread, browser-security or UI APIs.

## Decision

**Proposed architectural direction:** make a browser, embedded view, desktop application and headless application different compositions and authority profiles of one extensible component ecosystem. Application services should be implementable in Rust or Wasm and accessible to *both idiomatic native Rust and compiled JS/TS/Wasm consumers* through appropriate interfaces. WIT/Component Model describes published component service contracts; it does **not** require a separate instance, process, ABI crossing or allocation for every internal operation.

The technical policies below are design constraints and experiments to validate, **not yet accepted platform contracts**. No new priority, workstream ownership, fixed public ABI or security guarantee follows automatically from this proposed ADR.

### Shared services, distinct environment profiles

Possible logical services include JS execution, document/DOM representation, HTML/CSS, layout/paint, text, input/events, workers, networking, storage, window/view/surface and application lifecycle. They may be packaged together or separately. Keep language semantics inside the guest/runtime, not delegated invisibly to ambient host JavaScript.

A browser profile exposes web-compatible globals and web authorities. A general application profile may import richer portable system and desktop services under explicit grants. A pure web document must not acquire process, native window, file or plugin authority merely because its containing application uses those services. Selection of a component set is **not sufficient** for browser security: browsing contexts, origins, navigation, permission policies, scheduling and isolation remain observable requirements.

Prefer standard WASI component interfaces for suitable portable files, streams, sockets, clocks, randomness and related OS services; layer usable, .NET-like higher-level library ergonomics above them. Do not introduce a parallel POSIX-style syscall ABI by default. Platform-dependent process/window/device operations may need additional WIT services and explicit unsupported/denied results. Exact WASI release, async compatibility and provider support require verification at implementation time.

### Rust and JS/TS interface parity

Aim for similar **available capabilities**, not identical language syntax or automatic projection of arbitrary Rust types. A reusable Rust implementation can expose an idiomatic Rust facade and a versioned WIT interface with generated TS/JS bindings. WIT records, resources, lists, results and ownership rules do not directly encode Rust generics/lifetimes, arbitrary JS object graphs or whole DOM references. Generated language facades can preserve ergonomic object identity through checked resource handles.

The compiler should recognize a statically known imported WIT interface in admitted JS/TS source and lower calls to an actual imported operation, rather than require a manually written adapter to execute the requested operation. Generated glue may include type/wire conversion, resource retain/release, errors, scheduling and version checks; a WIT package alone does not implement all JS language interop. Introduce source support incrementally and preserve ordinary JavaScript evaluation order and observable results. TypeScript annotations and generated declarations are not substitutes for runtime checks.

### Components, shim providers and loading

Separate three mechanisms:
1. **Build-time composition:** link available Wasm components and library code against typed imports/exports, retaining portable Wasm.
2. **Runtime provider resolution:** instantiate a supplied implementation of a declared versioned interface, possibly an application-local third-party shim.
3. **Native host plugins:** expose OS-specific/Rust implementations under the same typed contract when the host explicitly permits them.

A future loader/resolver should specify interface versions, dependency resolution, provider choice, capability attenuation, ownership/revocation, failure/lifecycle, and artifact provenance. An unknown provider must not be able to substitute a privilege escalation. Native in-process code has the host process's memory authority; a WIT wrapper does not sandbox it. A native plugin needing containment may require a distinct process. Do not assume an installed provider is visible to untrusted browser content.

### Window, view and surface separation

A native application-window resource is not the JavaScript browser `Window` global. Borrow the useful separation seen in Electron between OS window management, contained views/web contents, and presentation/input. Define browser-compatible `window.open()` and other web objects as environment facades mediated by a browsing-context and policy service. General app window/view/surface operations should be accessible to native Rust and via WIT/JS bindings. A headless surface is a valid output backend, not necessarily a disposable test-only hack.

For A1, a headless frame sink with synthetic or forwarded real input is sufficient to prove the interaction and shared API. Exact Win32/Cocoa/Wayland/X11 integration, toolkit selection, monitor APIs, menus, dialogs, accessibility, compositor and full text engine are separate implementation decisions. Platform restrictions such as global positioning support must be reflected honestly.

### Workers, asynchronous tasks and optional parallel JS

Separate **Web Worker-compatible agents** (independent JS execution and message/transfer semantics), **asynchronous jobs** (cooperative scheduling, futures/streams, I/O), and **shared-memory parallel threads** (atomics and explicit synchronization). Exposing all three behind a single `async` keyword would be misleading. The web profile must preserve standard JS run-to-completion, microtask and Worker semantics within the implemented compatibility surface.

**Desired opt-in extension:** permit general-application JS/TS code to request true parallel computation without forcing web-compatible JS behavior to change. An initial **async-returning task API** is a plausible source-level seam: the caller supplies an explicitly parallelizable function/work item with owned or transferable inputs; an isolated worker/instance runs it, and completion is delivered asynchronously to the caller. The mere presence of `async` on a function is *not* permission to execute it concurrently: shared lexical captures, mutability, errors, Promise ordering and reentry may make that incorrect.

First explore isolate-style parallelism with explicit transfers or immutable shared data, no implicit sharing of JS object identity, bounded workers, cancellation and failure reporting. Shared-memory threads, shared closures/heap, workers-as-processes and concurrency syntax remain separate decisions. The provider may use OS threads, processes or another executor without changing the requested source semantics. Preserve ADR 0013's one-agent job order, no unsafe reentry and distinction between host task completion and guest Promise jobs. Measure actual concurrency and latency without claiming that async functions run in parallel today.

### Isolation and placement independent of composition

Model separately: per-instance linear-memory isolation; capability/grant isolation; worker scheduling/ownership; and OS process or OS sandbox containment. A component may expose a public WIT interface without being in another OS process; an isolation policy may require a process boundary even for components with identical APIs.

Rust and Wasm plausibly reduce the memory-corruption/crash rationale behind Chromium's original 2008 multiprocess architecture, but do not make every in-process configuration safe. Host/engine defects, unsafe Rust/FFI, native plugins, aborting panics, OOM, drivers, shared service logic flaws and side channels remain risks. Spectre-era cross-origin secrecy is an additional reason for stronger process/address-space isolation, not the original motivation for Chromium's process model. Do not claim Wasm linear-memory separation alone supplies the required cross-origin security. Prefer explicit trust domains and measured placement policies over a hardwired PID-per-view or PID-per-component rule.

Changing placement must **not** silently change authority or observable API semantics. Synchronous calls cannot always be moved behind IPC without a new async contract, deadlock/lifecycle analysis and error handling. Keep policy/placement as a candidate plan, not an implemented generic placement optimizer.

### Minimize crossing and marshalling overhead

WIT specifies **logical interoperability**, not a mandate to marshal every DOM/layout operation. Especially for a native Rust renderer with compiled Wasm clients, avoid unnecessary JS-host round-trips and repeated subtree serialization.

Candidate optimization levels:
- Direct Rust calls or private core-Wasm calls inside one trusted implementation/fused deployment.
- Stable, validated resource handles for trees/windows/surfaces, retaining provider-owned state rather than copying it.
- Generated canonical-ABI adapters across distinct component memories with deliberate string/list conversions, copies and ownership rules.
- Batched/coarse-grained scene operations, reusable buffers or validated transfers, and lazy conversion caches when semantics permit.
- Explicit IPC across OS isolation boundaries, with its own scheduling and failure contract.

Never share an unchecked host pointer across a sandbox or silently fuse components that require independent isolation. Identical WIT signatures do not guarantee interchangeable memory layouts or safe shared mutable state. Browser-compatible DOM mutations must remain synchronously observable where specified; deferred layout is allowed only when reads and event semantics stay correct. A higher-level general-app scene API may offer transaction-style updates with a different declared contract.

**Optimization target:** minimize crossings, converted/copied bytes, allocations and redundant state in equivalent workloads, while retaining authority, ordering, resource lifetime and fault containment. Measure before adopting component fusion or claiming near-zero interop cost.

## Scope and non-goals

This proposal frames composable platform services, JS/Rust parity, third-party shims, host profile differences, a possible parallel-JS extension and performance-aware isolation/ABI selection. It does **not** select a public WIT package, stable plugin manifest, global window API, specific OS backend, JS concurrency syntax, garbage collector, general DOM implementation, compiled component output format, automatic optimized fusion or security certification. It does not close the current runtime-import, object/heap or Unicode font-rendering gaps. A1 remains an application demonstration, not a mandate to finish the entire platform architecture.

## Alternatives considered

- **One monolithic browser engine and a separate desktop framework:** simpler initial integration but duplicates capabilities and reduces substitutability; prefer a shared component ecosystem.
- **Expose browser DOM/`Window` APIs as the universal OS API:** familiar to web authors but entangles browsing-context security with native application capabilities; retain as a compatibility facade only.
- **One independently instantiated WIT component per internal library or DOM operation:** maximizes uniformity but risks unnecessary copies and call overhead; use typed *published* boundaries without prescribing runtime separation.
- **Hardwire OS processes to tabs, workers or components:** offers strong containment but couples application topology to processes; preserve as a placement option for threat models that require it.
- **Link all providers statically with no loader:** simple and efficient for trusted apps; retain as a deployment option, but leave room for opted-in interchangeable providers and host shims.
- **Automatically parallelize every `async` function:** appealing syntax but breaks JS shared-state and ordering assumptions; reject implicit concurrency, explore opt-in async-returning tasks.
- **Replace WASI with a Grunkalunka POSIX clone:** creates duplicate portable OS contracts; prefer standards plus explicit additions where needed.

## Consequences

Potential gains: one portable application stack, third-party extension ecosystem, Rust/JS capability parity, substitute implementations, less unnecessary IPC/JS-host marshalling, and process placement driven by trust rather than incidental object topology. Costs: versioned interface and binding maintenance, resource lifecycle complexity, ABI lowering, runtime/provider resolution, opt-in concurrency semantics, performance regression risk and a larger host security review surface. The existence of a WIT interface does not guarantee portability, near-zero-cost calls, or isolation.

## Verification and evidence

**Verified in repository before this proposal:** a WIT-backed Markdown outline under granted/denied profiles; a compiled import-free JS numeric toggle; Rust native semantic geometry/RGB painting and hit testing; a packaged headless A1 runner; handcrafted compiled-capability adapter tests; bounded async ownership/job scheduling experiments; and a core-Wasm runtime composer. These are individually documented, not one finished cross-language public service. The previous macOS detached-entry bug was fixed in [PR #270](https://github.com/tseguine/grunkalunka/pull/270).

**Not verified:** a source-compiled JS WIT import, general generated bindings or WIT provider loader, WIT-backed UI mutation, real text rasterization, live windowing, portable thread model, arbitrary third-party components, in-process equivalence to OS-process isolation, cross-boundary zero-copy or security against side channels. No new runtime/compiler experiments were executed while writing this ADR.

**Bounded next experiments, in order of independent risk:**

1. **Source-to-WIT call.** Compile paired JS/TS source that statically imports a narrowly specified WIT service (initially scalar inputs/results); make the generated guest actually invoke the import. Compare execution with a Rust provider in Wasmtime and an equivalent supported JS/Wasm host path. Missing import, denied call, source mutation, evaluation order, error distinctions and source budgets need real tests. A handwritten WIT adapter that invokes the generated function is *not* sufficient. Core-Wasm plus an explicit component adapter may precede full compiler component emission.
2. **Shared UI service.** Add one resource/handle- or bounded-record-backed scene/action operation with the same observable semantics for a Rust consumer and compiled JS consumer. Test grant/deny, stale/foreign handle, ownership, activation ordering and actual pixel/text changes. Integrate with the A1 witness below.
3. **Provider substitution.** Resolve two independently supplied implementations of the same versioned WIT service, one possibly a Wasm component and one Rust host shim. Preserve grants and behavior; compare failure/cleanup.
4. **Interop/placement measurements.** Compare a direct trusted call, typed WIT call and optionally IPC on equivalent scene operations. Record CPU/wall time, call counts, allocation and copied bytes with isolation guarantees; reject any optimizer that changes permission or observability.
5. **Opt-in parallel-isolate prototype.** Run two explicitly requested JS/TS work items concurrently with independent state and owned inputs; return async results without disturbing single-agent Promise order. Include ordered callback delivery, cancellation/error, no-shared-identity and resource-limit controls. A single-thread async simulation is not evidence of true parallelism.

For A1 specifically, the smallest integration acceptance is: the same authorized note under both profiles, a host event/hit activating a control, a genuinely source-compiled JS operation changing shared scene state through an agreed interface, reflow and **readable rendered text** plus changed pixels, and no denied backing reads. A native Rust consumer must exercise the same published behavior. A headless frame/input provider is acceptable before adopting a real window backend. [A1 next-slice steering](../a1-note-preview.md#candidate-next-a1-acceptance-and-compiler-priorities) tracks the application-bound portion; it does not assign ownership to other streams.

## Explicit boundary statement

Define and test the **contract** between languages, components and hosts; allow placement-specific efficient implementations **only** when trust, authority and observable semantics are preserved. This ADR records candidate directions, not an already-built loader, an adopted security/process policy, or the full browser implementation. Public ABI details and implementation ownership require later bounded, evidence-based decisions.

## Related work and follow-up

- [ADR 0012](0012-guest-runtime-and-host-capabilities.md): guest-owned semantics and capability boundaries.
- [ADR 0013](0013-async-job-and-activation-ownership.md): guest suspension vs host scheduling; single-agent job ordering.
- [ADR 0011](0011-semantic-values-and-lifetime-boundaries.md): semantic values and cross-boundary ownership.
- [ADR 0008](0008-strict-core-wasm-composition.md): currently bounded, import-free composition.
- [A1 execution evidence](../a1-note-preview.md), [compiled-capability probe](../compiled-capability.md), [renderer](../block-layout.md), [A1 steering #52](https://github.com/tseguine/grunkalunka/issues/52).
- External specifications to inspect for each bounded implementation: [WIT/Component Model](https://component-model.bytecodealliance.org/design/wit.html), [WASI](https://wasi.dev/), [Electron window/view separation](https://www.electronjs.org/docs/latest/api/base-window). Tooling/version and license checks are still required.

## Summary

Make browser and application environments compositions of shared services and explicit authority profiles. Offer idiomatic Rust and JS/TS access to published WIT contracts, extensible providers, and placement-aware generated glue, with the right to optimize away unnecessary crossings only inside a compatible trust domain. Explore opt-in true JS parallelism through explicit isolated asynchronous work, never by silently changing ordinary `async` semantics. Prove the first source-compiled WIT service call and Rust/JS A1 scene operation before claiming the architecture works.
