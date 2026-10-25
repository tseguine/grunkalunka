# Grunkalunka

**Gimme damn damn WASM!**

A web application framework with familiar JS/TS authoring, portable deployment,
and explicit capabilities. Its foundation is a Wasm-first Rust component and
application platform, using Wasmtime initially and WIT/Component Model interfaces
at component boundaries. Browser, Node-compatible and desktop environments are
applications on that platform.

The maintained implementation is at development version **0.0.1**. The project is
still in PoC development: useful pieces are implemented and tested directly in
the main workspace, while unresolved design questions may use disposable spikes.
The current compiler admits an explicit numeric subset and a primitive-return
profile; it does not yet implement full JavaScript/TypeScript, browser APIs or Node
runtime compatibility.

## Build and run

Install current Rust stable and Node.js 20 or newer. Install the core-Wasm target
and acquire the locked dependencies once, then build and verify offline:

```sh
rustup target add wasm32-unknown-unknown
cargo fetch --locked
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo build --workspace --locked --offline
node scripts/check.mjs
```

The complete check includes formatting, Clippy, workspace tests, dependency
screening, the historical frontend observations, the JSON guest on Node and
Wasmtime, and the differential regression corpus. The JSON guest is a data
transport foundation; JSON source values and builtins are not yet lowerable.
To replay only the compiler/runtime demonstration:

```sh
node scripts/poc.mjs
```

It compiles JS and TS to real portable Wasm, runs that Wasm in Wasmtime and Node,
and compares results against original JavaScript. The affine example computes
`(a * 2 + b) / 3`; `run(4, 1)` returns `3`.

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
imports; it is not a hardened arbitrary-module service.

## Workspace

- `crates/grunkalunka-compiler`: source text to portable typed Wasm; independent of Wasmtime.
- `crates/grunkalunka-runtime`: numeric module instantiation and execution.
- `crates/grunkalunka-cli`: file and standard-input handling, compile/inspect/evaluate commands.
- `crates/grunkalunka-markdown`: standalone CommonMark headings, links and images with original source spans.
- `tests/compatibility`: maintained JS/TS differential fixtures and evidence reports.

[Compiler contract](docs/compiler.md) | [Milestones](docs/milestones.md) ·
[Research roadmap](docs/roadmap.md) | [Dependencies](docs/dependencies.md)

See [Markdown inspection](docs/markdown.md) for the parsing API and its limits.

Package scripts provide `build`, `poc` and `check` entry points. Direct Node execution
is verified on this workstation; its npm launcher currently fails before entering
the runner because a child shell cannot find Node. Bun and other host platforms
remain unverified. The [frontend research](spikes/001-frontend-semantics/README.md)
is retained separately from the maintained workspace.

MIT licensed. Third-party material retains its applicable licenses and notices.
