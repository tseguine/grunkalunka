# Single-file numeric watch example

After building the native compiler, run `npm run example:watch`, or directly:

```sh
node examples/numeric-workflow/watch.mjs
node examples/numeric-workflow/watch.mjs 84 30 ./my-function.ts
```

The arguments and `GRUNKALUNKA_COMPILER` override match the one-shot numeric
example. Defaults use the checked-in GCD source and inputs 84 and 30; explicit
source paths resolve against the caller's directory. There is no implicit native
build or download. Each successful run prints `RESULT` and its numeric value.
Source/read/build/execution errors go to stderr, then watching continues. Correct
the source to retry; a rejected edit never falls back to an earlier Wasm artifact.

This is an opt-in development example, not a server, bundler or public watch API.
Only the selected file's bytes are polled, every 250 ms while idle. It handles
replacement saves and deletion/recreation without tracking a file's old identity.
Unchanged bytes do not rebuild, even after a timestamp-only update. Files are read
in full; there is no directory traversal, dependency graph or compiler-binary watch.
Restart after changing the executable or options.

Builds execute serially using owned source snapshots and temporary compilation
outputs. An edit during a build does not immediately cancel it. After the build,
the source is reread; a result or diagnostic for different observed bytes is
suppressed and the latest snapshot is built next. This coalesces edits rather than
queueing every intermediate revision. It is not an edit journal: transient writes
between polls can be missed, and no guarantee is made against a change occurring
immediately after the final comparison. Partially saved source can produce an
ordinary diagnostic before a later completed save is observed.

Ctrl-C/SIGTERM request shutdown and propagate cancellation to the active compiler
or numeric worker. The driver waits for that operation's cleanup before returning.
Compiler and execution timeouts retain their existing defaults; the latter also
bounds a guest loop introduced by an edit. There is no total filesystem deadline,
hostile-code memory sandbox, persistent output, release or deployment. A forcibly
terminated parent cannot guarantee temporary cleanup.

The existing example test suites cover the polling lifecycle and a real
valid/invalid/corrected JS/TS edit cycle. Run the canonical checks, or after a native
build run `node --test tests/build/example-options.test.mjs tests/build/example.test.mjs`.
