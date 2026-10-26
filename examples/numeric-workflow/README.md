# Local numeric build-and-run example

This is a complete local consumer of the [build API](../../docs/build-api.md):
read source, compile fresh Wasm, instantiate it in Node, and call `run(a, b)`.
The included source computes the greatest common divisor with a mutable loop,
`Math.abs` and remainder. It stays within the existing numeric profile.

Build the native compiler first from the repository root (Rust and Node required):

```sh
cargo fetch --locked
cargo build --workspace --locked --offline
npm run example:numeric
npm run example:numeric -- 1071 462
```

These print `6` and `21`. No `npm install`, package dependencies or network access
is needed to run the example after the native build. The equivalent direct command
is `node examples/numeric-workflow/run.mjs 1071 462`. From another directory, use an
absolute path to `run.mjs`; its default source and compiler are resolved relative
to the script, not your working directory.

To experiment, copy `gcd.js` to a new `.js` or `.ts` file and run:

```sh
node /path/to/grunkalunka/examples/numeric-workflow/run.mjs 84 30 ./edited.ts
```

Edit and repeat the same command. Explicit source paths and an optional
`GRUNKALUNKA_COMPILER` executable path resolve against the caller's working
directory. Inputs must be decimal safe integers; the original GCD example returns
zero for `(0, 0)`. Custom source still needs the existing two-number numeric ABI.
The wrapper does not infer imports, bundle modules or add language features.

Every invocation recompiles. There is no persistent Wasm output or fallback to an
older result after an invalid edit. Temporary build files are owned and cleaned
by `compileSource()`. Errors include the original input filename and any compiler
diagnostic; inner diagnostic paths may refer to deleted temporary files. Invalid
UTF-8 is rejected rather than silently replaced.

This runs trusted local source in Node's WebAssembly engine, not a sandboxed service.
Compilation retains the build API's timeout. Subsequent Wasm initialization and
execution use a fresh worker with a separate five-second deadline, so an infinite
loop reports an error without printing a result. See [numeric execution](../../docs/numeric-execution.md)
for cancellation, termination and resource limits. No imports are granted; this
implements no component/capability profile and does not complete A1.
No watch server, release packaging, publication or deployment is involved.

`node --test tests/build/example-options.test.mjs` checks the JS algorithm against
an independent divisor oracle and the argument/package-script contract. After the
native build, `node --test tests/build/example.test.mjs` runs the actual entry point
from another directory, repeats builds, changes JS/TS source, checks failed edits
and verifies recovery. Both are included in the canonical check script. The npm
script is checked for wiring; integration invokes its Node entry point directly.
