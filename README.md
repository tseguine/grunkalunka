# Grunkalunka

[![CI (master)](https://img.shields.io/github/check-suites/tseguine/grunkalunka/master?label=CI)](https://github.com/tseguine/grunkalunka/actions/workflows/ci.yml?query=branch%3Amaster%20event%3Apush)

**Gimme damn damn WASM!**

A web application framework with familiar JS/TS authoring, portable deployment,
and explicit capabilities. Its foundation is a Wasm-first Rust component and
application platform, using Wasmtime initially and WIT/Component Model interfaces
at component boundaries. Browser, Node-compatible and desktop environments are
applications on that platform.

The maintained implementation is at development version **0.0.1**. The project is
still in PoC development: useful pieces are implemented and tested directly in
the main workspace, while unresolved design questions may use disposable spikes.
Compilation is deliberately profile-scoped: numeric and four-kind primitive
entry points, experimental async/closure paths, and a synchronous
[WasmGC-backed data-object slice](docs/managed-values.md). The object slice
supports fresh literals, static own-property reads/writes, aliases,
identity/truthiness and managed references through ordinary named helpers.
Known Number operations retain direct f64 execution and public exports retain
the current numeric/primitive contracts. This is **not** full JavaScript/TypeScript,
a general object/prototype implementation, browser APIs, or Node compatibility.

## Build and run

Install Rust through rustup and Node.js 24. Rust uses the repository-selected
toolchain; Node 24 is the CI runtime. Install the core-Wasm target and acquire the
locked dependencies once, then build and verify offline:

```sh
rustup target add wasm32-unknown-unknown
cargo fetch --locked
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo build --workspace --locked --offline
node scripts/check.mjs
```

To opt in to the lightweight Rust formatting pre-commit check, run
`chmod +x .githooks/pre-commit && git config --local core.hooksPath .githooks`
on macOS/Linux (or ensure the hook is executable in your Git environment).
The hook checks formatting for the main workspace and frontend spike; it does
not format files automatically. GitHub API commits bypass local hooks, so agents
must run `cargo fmt --all` themselves before committing Rust changes.

The complete check includes formatting, Clippy, workspace tests, dependency
screening, the historical frontend observations, the JSON guest on Node and
Wasmtime, and the differential regression corpus. [CI](docs/ci.md) runs the canonical check on Linux and Windows for `master`
pushes and on native macOS through its separate `master`-triggered workflow. This is coverage of those runner
configurations, not every host/runtime combination.

String and JSON source experiments still use a separate, narrower
[heap profile](docs/heap-literals.md). The newer
[managed-value path](docs/managed-values.md) instead uses the shared IR, while
retaining narrow source admission and primitive host boundaries. Neither path
implements general JS heap semantics, prototype lookup, arbitrary arrays or
public reference-valued exports.
To replay only the differential compiler/runtime demonstration:

```sh
node scripts/poc.mjs
```

It compiles JS and TS to real portable Wasm, runs that Wasm in Wasmtime and Node,
and compares results against original JavaScript. The affine example computes
`(a * 2 + b) / 3`; `run(4, 1)` returns `3`.

### A local build-and-run example

After building the CLI, the [numeric example](examples/numeric-workflow/) compiles
its checked-in GCD source and executes the resulting Wasm:

```sh
node examples/numeric-workflow/run.mjs
node examples/numeric-workflow/run.mjs 1071 462
npm run example:numeric
```

The default prints `6`; the explicit inputs print `21`. An optional third argument
selects a JS/TS file exporting the supported two-number `run` function. Edit it and
rerun the command to rebuild. Failed edits report diagnostics rather than executing
an earlier output. Compilation uses temporary files; execution uses a disposable
worker with a deadline. These are numeric examples, not general Node applications.

### CLI commands

```text
target/debug/grunkalunka compile SOURCE.ts NEW-OUTPUT.wasm
target/debug/grunkalunka wat SOURCE.ts
target/debug/grunkalunka eval OUTPUT.wasm < input-bits.txt

target/debug/grunkalunka compile-value SOURCE.ts NEW-VALUES.wasm
target/debug/grunkalunka wat-value SOURCE.ts
target/debug/grunkalunka eval-value VALUES.wasm < input-bits.txt
```

The `-value` commands select tagged undefined/null/Boolean/Number returns with the
same two numeric inputs. Output lines are `undefined`, `null`, `boolean:0`,
`boolean:1` or `number:` followed by 16 hexadecimal binary64 digits. The ordinary
commands retain their numeric interface. The PoC runner reports these profiles
separately; selected examples do not establish full language compatibility.

Add `.exe` on Windows. See the compiler contract for the binary64 input protocol,
admitted language, limits and diagnostics. Compilation refuses existing output
files. The numeric runtime executes locally generated modules without ambient
imports; it is not a hardened arbitrary-module service. For opt-in phase/failure
records on **stderr**, set `GRUNKALUNKA_DIAGNOSTICS=1`; see
[observability](docs/observability.md).

### Programmatic builds and measurements

The [Node build API](docs/build-api.md) provides file-to-file `compile()` and
source-text-to-bytes `compileSource()`, both using an explicitly selected native
compiler with bounded captured output and cancellation. The
[numeric worker runner](tools/runtime/numeric.mjs) executes a trusted import-free
numeric artifact without blocking the caller's event loop on a guest loop.

`node tools/bench/numeric.mjs` prints a [numeric cost report](docs/numeric-benchmark.md)
with build API timing, startup observations, warmed JS/Wasm samples and provenance.
Its default compiler is the debug build. Follow the measurement guide before
comparing results; the smoke test checks correctness, not a speed threshold.

### Experimental Markdown document capability

The completed [bounded document demonstration](docs/document-outline-authority.md)
builds the existing Markdown library into a core-Wasm guest, then runs it through
a WIT component with explicit in-memory document grants. To reproduce the
native/component comparison and denial/handle-isolation checks after installing
the Rust target and building the workspace:

```sh
node scripts/check-document-outline.mjs
```

The host receives owned heading lists, Unicode labels and original source ranges.
Unauthorized documents cannot be selected by supplying a backing index or forged
guest borrow. This is a test-driven component example, **not** a filesystem vault,
editable notes app or JavaScript/TypeScript document API. See [milestones](docs/milestones.md)
for the remaining P1 and A1 goals.

### A1 application and source-to-service experiments

The [runnable headless note preview](docs/a1-note-preview.md) combines an
authorized Markdown/WIT guest, compiled JS selection, native semantic layout and
RGB painting. Its host-managed hit/action/repaint proof is separate from the
experimental [source-compiled WIT UI import](docs/a1-ui-source-import.md),
which compiles a statically bound scalar JS/TS service call and executes against
a Rust WIT provider. The two paths are **not yet joined** into a full
event-to-source-import-to-render application. Text glyphs, a windowed UI,
general module imports and a public UI contract remain open.

## Workspace

- `crates/grunkalunka-compiler`: source text to portable typed Wasm; independent of Wasmtime.
- `crates/grunkalunka-runtime`: experimental Wasmtime execution of numeric, primitive, async and heap modules, with bounded capability-host examples.
- `crates/grunkalunka-cli`: file and standard-input handling, compile/inspect/evaluate commands.
- `crates/grunkalunka-markdown`: CommonMark summaries, code blocks, heading outlines and prose projection.
- `crates/grunkalunka-json` / `crates/grunkalunka-json-guest`: bounded JSON data operations and a separate guest transport, not a general JS heap.
- `crates/grunkalunka-wasm-link`: bounded core-Wasm composition, not a general linker.
- `crates/grunkalunka-layout`: independent headless block geometry and typed semantic-tree projection, not an HTML/CSS renderer. See the [layout experiment](docs/block-layout.md).
- `tests/compatibility`: maintained JS/TS differential fixtures and evidence reports.
- `tools`: repository-local build, execution and measurement adapters.

[Documentation guide](docs/README.md) | [Compiler contract](docs/compiler.md) |
[Milestones](docs/milestones.md) · [Research roadmap](docs/roadmap.md) |
[Dependencies](docs/dependencies.md)

See [Markdown inspection](docs/markdown.md) for the parsing API and its limits.

Package scripts also provide `build`, `poc` and `check` entry points. Bun
compatibility has not been established by the Node CI runs. Historical workstation
observations remain in the [frontend research](spikes/001-frontend-semantics/README.md),
separately from the maintained workspace.

MIT licensed. Third-party material retains its applicable licenses and notices.
