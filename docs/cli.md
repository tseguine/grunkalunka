# Command-line discovery

After building the workspace, `target/debug/grunkalunka --help` lists the maintained
commands and their transport contract. Add `.exe` on Windows. `-h` is an identical
alias. `--version` and `-V` print `grunkalunka` followed by the Cargo package version;
this is not a Git revision, language-compatibility level or runtime-engine version.

These standalone options write to stdout, leave stderr empty and exit successfully.
They do not require a source file, Wasm artifact or evaluation input. Extra arguments
are errors rather than being silently ignored. An invocation with no arguments,
an unknown command or an incomplete command retains the existing usage error and
exit status 1. Per-command help flags and combined short options are not introduced.

The numeric commands are `compile`, `wat` and `eval`; the primitive-return commands
are `compile-value`, `wat-value` and `eval-value`. Both use the same two numeric
inputs. Source extension selects JS or TS. Compilation refuses existing output
files, and WAT output goes to stdout.

Evaluation's stdin protocol is not decimal input: each line contains two 16-digit
hexadecimal binary64 bit patterns, with at most 4096 input pairs. The numeric output
is one pattern per line; primitive output carries the existing kind prefix.
See the [compiler contract](compiler.md) for complete admission and runtime limits,
and the [Node build API](build-api.md) for programmatic compilation.

`cargo test -p grunkalunka-cli --locked --offline` tests the real CLI subprocess:
help content and aliases, package version, stdout/stderr placement, and preserved
usage failures. Existing workspace CI includes these tests. Existing compile/evaluate
behavior, source acceptance, generated Wasm, runtime ABI and dependencies are unchanged.
