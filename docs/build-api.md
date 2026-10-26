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

The compiler refuses existing destinations. `compile()` never removes outputs,
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

## Compiling source text

`compileSource()` from the same module accepts `source` as text and an explicit
`language: 'js' | 'ts'`, instead of source/destination paths. It returns
`{ wasm, stdout, stderr }`, where `wasm` is an owned Node `Buffer` suitable for
`WebAssembly.instantiate`. The same compiler, profile, working directory and
subprocess controls apply. No persistent output path is returned.

```js
import { compileSource } from './tools/build/compile.mjs';

const { wasm } = await compileSource({
  compiler: `./target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`,
  language: 'ts',
  source: 'export function run(a: number, b: number) { return a + b; }',
});
const { instance } = await WebAssembly.instantiate(wasm);
console.log(instance.exports.run(4, 1)); // 5
```

This is a source-text convenience API, not an in-process compiler. Each call
creates its own temporary directory, writes UTF-8 source, invokes the existing
CLI, reads the output and removes the directory before settling. CRLF and Unicode
text are preserved. Unpaired UTF-16 surrogates reject rather than silently becoming
replacement characters; literal JavaScript escape sequences remain ordinary text.
Empty source is passed through for the compiler to diagnose.

`tempDirectory` selects an existing parent, defaulting to the system temporary
directory; relative paths resolve against `cwd`. Only the newly created child
directory is removed, including partial output after failure or cancellation.
An already-aborted request may create scratch files but launches no child.
Filesystem errors reject normally; cleanup failure after another error produces
an `AggregateError` containing both errors, with the original error as its cause.
Forced termination of the calling process cannot guarantee cleanup.

Diagnostics may name temporary paths that no longer exist. This API neither
adds module resolution nor remaps diagnostics to a virtual filename. The timeout
and cancellation control the compiler subprocess, not filesystem operations;
`maxBuffer` bounds captured streams, not Wasm size. The output is read in full and
is not independently validated. The executable and temporary parent remain trusted.

`node --test tests/build/source.test.mjs` checks transport and workspace lifetimes
with subprocess fixtures. The existing compiler integration suite also exercises
source-text builds and rejected input. Both run through the canonical check script.
