# Node compiler build API

`tools/build/compile.mjs` is a dependency-free ESM adapter for build scripts. It
invokes an existing native compiler; it does not build, install or download one.
Build the workspace first with `cargo build --workspace --locked --offline` after
fetching its locked dependencies. From a script in the repository root:

```js
import { compile } from './tools/build/compile.mjs';

const result = await compile({
  compiler: `./target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`,
  source: './example.ts',
  destination: './example.wasm',
});
console.log(result.destination);
```

Supply `compiler`, `source` and `destination` as nonempty path strings. Relative
paths resolve against `cwd` (default: the caller's current working directory).
There is no PATH lookup, shell expansion, command string or added quoting. Use a
native executable, not a Windows `.cmd`/`.bat` launcher. The output directory must
already exist. The caller selects and trusts the executable; this is not a sandbox.

`profile` defaults to `numeric`, invoking `compile`; `primitive` invokes
`compile-value`. JS/TS selection and source admission remain the executable's
responsibility. No module bundling, type checking or additional language support
is introduced. Success returns `{ destination, stdout, stderr }`, with an absolute
destination and separate UTF-8 output streams. The adapter trusts a successful
compiler exit; it does not independently validate the emitted Wasm.

Invalid arguments reject with `TypeError` or `RangeError`. Process failures reject
with `CompilerError`, preserving the underlying `cause`, `code` (exit number or
Node error code), `signal`, `killed`, `stdout` and `stderr`. Launch failures such as
`ENOENT`, rejected source and output-buffer overflow are not successful builds.
Do not depend on platform-specific wording in diagnostic messages.

Optional `signal` accepts an `AbortSignal`. An already-aborted call launches
nothing. Aborting a running invocation forcefully terminates its direct child and
waits for that child's close event before rejecting. `timeout` defaults to 30,000
milliseconds; `maxBuffer` defaults to 1 MiB per stream. Both accept positive
integers up to 2,147,483,647; zero does not disable either bound. These options use
Node's [execFile semantics](https://nodejs.org/api/child_process.html#child_processexecfilefile-args-options-callback).
This is direct-process control, not process-tree supervision.

The compiler refuses existing destinations. The wrapper never removes outputs,
including after failure, timeout or cancellation: a failed or interrupted write
can leave a partial file, which callers must handle explicitly. Output publication
is not transactional. Independent calls may run concurrently; callers should use
distinct output paths rather than relying on implicit serialization.

Run `node --test tests/build/process.test.mjs` for subprocess contracts. After
building the CLI, run `node --test tests/build/compile.test.mjs` for actual JS/TS
compilation, both profiles, Wasm execution, paths, diagnostics and no-overwrite
behavior. Both are included in `node scripts/check.mjs` and existing PR CI.

This is a repository-local Node API, not a published npm package or a binary
installer. Bun compatibility, packaged platform selection, watch mode and release
support policy remain separate work. No release or deployment is performed.
