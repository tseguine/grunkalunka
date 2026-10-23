# Grunkalunka

**Gimme damn damn WASM!**

The intended product is a web application framework with familiar JS/TS authoring,
portable deployment, and explicit capabilities: in the user's words, "a web
application framework that doesn't suck."

Its foundation is a Wasm-first, Rust-based, capability-oriented component and
application platform. The compiler/runtime machinery should serve a straightforward
build, run, debug, package and deployment experience.

The direction is TypeScript-first compilation to WebAssembly with vanilla JavaScript
compatibility, WIT/Component Model interfaces, and Wasmtime as the initial runtime.
Browser, Node-compatible, and desktop environments are applications built on the
platform. Their APIs should not become ambient authority in the platform core.

This repository starts with research and small disposable experiments. It now has
a working numeric compiler fragment; the production architecture remains open.

- [Research roadmap](docs/roadmap.md): questions, next experiments, and exit criteria.
- [Active milestones](docs/milestones.md): lowering, compatibility and capability
  demonstrations, with current status and observable completion criteria.
- [Frontend semantics spike](spikes/001-frontend-semantics/README.md): executable
  observations about Oxc's parsing, bindings, and diagnostics.
- [Numeric Wasm PoC](spikes/002-numeric-wasm/README.md): paired JS/TS arithmetic
  compiled into Wasm and compared with JavaScript under Wasmtime and Node-Wasm.

## Run the working PoC

```sh
# Initial dependency acquisition:
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo fetch --locked --manifest-path spikes/002-numeric-wasm/Cargo.toml
node scripts/poc.mjs
```

The demo compiles `(a * 2 + b) / 3` from TS/JS and executes the emitted Wasm.
For `(4, 1)` it returns `3`. The replay checks edge-case numeric results in two
Wasm engines and verifies rejection of unsupported syntax. See the PoC for its
explicit numeric ABI, limits, emitted files and license-screening details.

## Run the first experiment

Use a current Rust stable toolchain and Node.js 20 or newer. The first Cargo run
downloads the pinned dependencies; subsequent runs can use `--offline`.

```sh
cd spikes/001-frontend-semantics
cargo run --locked
node oracle.mjs
```

The first Rust probe inspects source; Node supplies its JavaScript behavior baseline.
That first spike generates no Wasm; the numeric PoC above adds compilation.
Read each spike's playback for the precise claims.

From the repository root, `node scripts/check.mjs` runs formatting, linting, the Rust
probes, JS oracles, dependency-license screening and the numeric PoC.
`npm run check` provides package-script wiring, but its
local execution fails before entering the script (`node` is not found in its child
shell; root cause unconfirmed). The dependency-free runner is also intended for Bun via
`bun run --bun check`; Bun execution has not yet been verified here. See the roadmap
for the build-support direction and the one-time dependency fetch command.

MIT licensed. Third-party dependencies retain their own licenses.
