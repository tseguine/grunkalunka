# Minimal diagnostic logging

The observability stream starts at external boundaries, not inside every compiler
operation or guest instruction. The purpose is to shorten agent debugging loops
by answering **which phase failed, on which input, and how can it be replayed?**

## CLI diagnostic mode

Set `GRUNKALUNKA_DIAGNOSTICS=1` for one invocation:

```sh
GRUNKALUNKA_DIAGNOSTICS=1 target/debug/grunkalunka wat example.js
GRUNKALUNKA_DIAGNOSTICS=1 target/debug/grunkalunka eval example.wasm < input-bits.txt
```

Diagnostic records go **only to stderr**. A record begins with `DIAG`, then
`phase=...` and `event=...` plus bounded key/value fields. They mark file
reads, compilation, instantiation, and execution completion or failure. Compile
and instantiate outcomes report elapsed milliseconds; these are local debugging
measurements, not performance benchmarks. On failed evaluation only, the record
includes a 1-based input line and the two original 16-digit binary64 bit patterns,
sufficient to replay that exact invocation. Invalid input instead reports its
line number and rejection class; successful values never generate per-pair logs.

Without the exact value `1`, diagnostic mode is off. Normal stdout (WAT,
compilation receipts, or machine-readable evaluation values), normal errors,
exit status, admission, ABI, and runtime behavior remain unchanged. The mode
does not grant Wasm capabilities, collect telemetry, or write diagnostic files.

## Rules for future instrumentation

- Log **boundaries and exceptional outcomes**, not each AST node, IR operation,
  host call, Promise job, or layout step. If a repeated event is necessary,
  keep it opt-in and bounded, with a stable correlation identifier.
- Preserve the original error rather than wrapping it in repeated generic logs.
  If a failure path needs a diagnostic, include the responsible stage and a
  minimal reproducible identifier. Avoid logging source contents, serialized
  documents, capabilities/tokens, pointers, or large object graphs.
- Keep source and argument values private by default. The explicitly enabled
  CLI mode records file paths and, on failed execution, numeric argument bits;
  use it only with inputs suitable for diagnostic output.
- Never mix host diagnostics with guest-visible stdout, JS `console` semantics,
  or host/guest ABI. Do not add mandatory log collection or broad tracing deps.
- Reuse ordinary package/integration tests and partial CI; do not add an
  independent GitHub Actions workflow for logs.

Possible next targets, to be driven by concrete debugging failures and coordinated
with the owning streams, are compiler parse/semantic/lowering/emit boundaries,
runtime import/capability/link failures, and abrupt test-process exits whose
current CI output does not identify the last failing case. See
[observability tracking #283](https://github.com/tseguine/grunkalunka/issues/283).
