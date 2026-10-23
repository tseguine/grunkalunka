# Grunkalunka

**Gimme damn damn WASM!**

A Wasm-first, Rust-based, capability-oriented component and application platform.

The direction is TypeScript-first compilation to WebAssembly with vanilla JavaScript
compatibility, WIT/Component Model interfaces, and Wasmtime as the initial runtime.
Browser, Node-compatible, and desktop environments are applications built on the
platform. Their APIs should not become ambient authority in the platform core.

This repository starts with research and small disposable experiments. It does not
yet contain a compiler, runtime, or settled architecture.

- [Research roadmap](docs/roadmap.md): questions, next experiments, and exit criteria.
- [Frontend semantics spike](spikes/001-frontend-semantics/README.md): executable
  observations about Oxc's parsing, bindings, and diagnostics.

## Run the first experiment

Use a current Rust stable toolchain and Node.js 20 or newer. The first Cargo run
downloads the pinned dependencies; subsequent runs can use `--offline`.

```sh
cd spikes/001-frontend-semantics
cargo run --locked
node oracle.mjs
```

The Rust probe inspects source; Node supplies a behavior baseline for the JavaScript
fixtures. No Wasm is generated yet. Read the spike's playback for the precise claims.

From the repository root, `node scripts/check.mjs` runs formatting, linting, the Rust
probe, and the JS oracle. `npm run check` provides package-script wiring, but its
local execution fails before entering the script (`node` is not found in its child
shell; root cause unconfirmed). The dependency-free runner is also intended for Bun via
`bun run --bun check`; Bun execution has not yet been verified here. See the roadmap
for the build-support direction and the one-time dependency fetch command.

MIT licensed. Third-party dependencies retain their own licenses.
