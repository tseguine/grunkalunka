# Rust Markdown Wasm named-function body correlation (P1)

The existing [P1 section and function-body evidence](p1-markdown-guest-sections.md)
shows that independently compiled Rust Markdown Wasm guests differ widely at
the same function ordinal, and that ignoring ordinal does **not** recover
all exact body-byte matches. The remaining differences could involve index
renumbering, name changes, compiler code generation or other build inputs.

## Narrow diagnostic

The [bounded name-body analyzer](../tools/bench/wasm-named-bodies.mjs)
reads the **existing, unchanged** `outline_guest.wasm` built for
`wasm32-unknown-unknown`. It uses the existing section/body fingerprint
reader, then parses the standard **`name` custom section's function-name
subsection** and associates its declared function indices with raw
function-body SHA-256 digests.

This deliberately accepts **only a core-Wasm guest without an import
section**. That invariant holds for the currently observed Markdown guest,
whose defined-function indices therefore need no imported-function offset.
For any guest with imports, the analyzer fails closed rather than guessing
the function index mapping. The experiment is not a generic Wasm linker.

A recorded function name is encoded as a **full SHA-256 of its UTF-8 bytes**
at the declared function index; no symbol strings or source-path names
are transferred in reports. Missing name-map entries are explicit zero
slots and counts, not fabricated identities. The parser rejects duplicate
or out-of-range function indices, malformed LEB128 lengths, truncated
UTF-8 names, duplicate function-name subsections and missing required
metadata. It has strict Wasm size, function-count and saved JSON bounds.

The [optional hosted workflow](../.github/workflows/p1-markdown-named-bodies.yml)
compiles the unchanged Markdown guest independently on Linux x64, Windows
x64 and native Apple Silicon ARM64 with locked/offline Rust and Node 24.21.0.
Linux transfers only its bounded **JSON fingerprint receipt** through
ephemeral Actions outputs, not a guest binary. Each receiver compares:

- How many hashed **name occurrences** are shared across hosts, preserving
  duplicate names with multiset counts.
- How many shared name occurrences also have **identical raw body length
  and SHA-256**, independent of function order.
- How many names match at the same ordinal, and how many recorded names
  are missing or unmatched on either host.
- The original positional and order-independent raw-body counts from the
  existing comparator, for context.

Different raw bodies for matching name strings are *observations*, not
tests that must pass an artificial similarity threshold. Invalid provenance,
different repository revisions, malformed input, or failed builds remain
failures. The functional correctness baseline is still the separate
Markdown/WIT component parity and capability checks.

Local reproduction after installing the Rust Wasm target and compiling
the guest:

```sh
node --test tools/bench/tests/wasm-named-bodies.test.mjs
node tools/bench/wasm-named-bodies.mjs inspect \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm > host-names.json
node tools/bench/wasm-named-bodies.mjs compare linux-names.json host-names.json
```

## Important limits

The `name` custom section contains compiler-emitted, *optional diagnostic
symbol names*. Identical symbol-name bytes are **not** a standardized,
immutable source-level function identity: compiler configuration can change
mangling, codegen units, instantiation and symbol numbering. Hashed names
hide their text but do not establish provenance or prevent theoretical
SHA-256 collisions. Matching named bodies need not invoke identical runtime
call targets if other function indices differ.

A shared name with changed body bytes does not by itself show a semantic
difference: immediates encoding function indices, instruction selection
and equivalent code transformations all affect raw hashes. This analysis
does not decode/normalize instructions or independently attest the Rust
compiler. It measures no performance or guest-attributed memory, uploads
no binaries and changes no application/compiler/runtime sources.
