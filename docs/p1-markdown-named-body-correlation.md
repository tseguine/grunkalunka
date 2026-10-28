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

The [optional hosted workflow](https://github.com/tseguine/grunkalunka/blob/4977a9d8d8025252c48c6fcd7fa87f858bb35035/.github/workflows/p1-markdown-named-bodies.yml)
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

## Narrow Rust terminal hash-suffix control

The [exact-head #119 hosted experiment](https://github.com/tseguine/grunkalunka/actions/runs/40701176158)
observed a name entry for every one of the 1,897 defined functions on Linux,
Windows and Apple Silicon. However, relative to Linux, **each receiver shared
only 195 exact raw diagnostic function-name occurrences**. Of those shared
names, 89 also had identical raw bodies and 106 had changed raw bodies. This
does not establish that the other 1,702 functions represent different source
functions: diagnostic names can embed build-specific symbol disambiguators.

As a controlled follow-up, the same bounded analyzer now computes a **second,
separate diagnostic name fingerprint** after applying precisely one
textual transformation:

```text
<nonempty prefix>::h<exactly 16 lowercase hexadecimal digits>
    → <nonempty prefix>
```

Only a *terminal* Rust legacy-style hash suffix is removed. Embedded hashes,
uppercase suffixes, shorter/longer hashes, raw Rust v0-mangled symbols and
other names are **not** modified. The original raw name digest remains
available and the tool records how many names matched this narrow pattern
on each host. Missing optional names stay missing.

The native Linux x64, Windows x64 and Apple Silicon ARM64 workflow now
compares both raw and transformed name **multisets**, preserving duplicate
occurrences. It additionally compares exact name-plus-body occurrences,
so matching a transformed name need not imply that raw code bytes match.
The output reports how many additional matches arise after this precise
normalization. A change in match counts is recorded, **not** used as a
pass/fail similarity target.

This is a diagnostic heuristic. Two genuinely different functions may
collapse to one transformed name; conversely, compiler symbol naming
may differ in ways this transformation cannot address. The hashed
transformed names do not establish source-level identity or independently
attest the compiler's mangling. The experiment does not normalize Wasm
instructions, function-index immediates, or run a performance comparison.
The existing named-body parser's bounds and import-free restriction remain
in effect; older receipts containing only raw names remain readable, but
one-sided normalization reports fail closed rather than silently comparing
different kinds of evidence.

## Legacy Rust mangled `17h…E` control

The [#121 exact-head native workflow](https://github.com/tseguine/grunkalunka/actions/runs/40706269118)
found **zero** raw names ending in the readable `::h` + 16 lowercase
hexadecimal digit pattern on Linux, Windows and Apple Silicon. Its
transformed-name overlap therefore remained exactly the original **195**
shared diagnostic names and **89** shared names with byte-identical bodies
on each receiving host. This is a useful negative result for that *one*
naming convention, not evidence that compiler symbol hashes cannot affect
other spellings.

A second, independent control tests the *legacy Itanium-like Rust mangled*
spelling directly, without decoding or demangling symbols. It removes only
the exact terminal substring `17h` followed by 16 lowercase hex digits
and final `E`, from names beginning `ZN`, `_ZN` or `__ZN` with at least
one preceding encoded path character. The final `E` is retained as an
opaque transformed-name token. For example:

```text
_ZN4core4item17h0123456789abcdefE
    → _ZN4core4itemE
```

Names with v0-style `_R...` mangling, path-like readable names, uppercase
hashes, embedded hashes, additional suffixes, or unrelated strings are
left intact. The previous readable `::h` control and the complete raw
name/body fingerprints remain independently available; the two heuristics
are **not composed**.

The analyzer now records only a four-category count of syntactic name
shapes (legacy-mangled, v0-mangled-like, readable path-like, other), how
many names matched the exact legacy tail pattern, and a separate
SHA-256 vector of transformed diagnostic names. No original symbol text
is transferred. It computes and reports the additional shared name
occurrences and shared name-plus-exact-body occurrences independently
of the earlier readable-suffix results. Duplicate names are counted
with multiplicity, missing names remain missing, and malformed/asymmetric
receipts fail closed.

An improvement would indicate that this *specific textual legacy hash
component* explains some previously unmatched name strings. It would
**not** establish source-function identity, a reproducible build or
semantic equivalence. Stripping hash components can merge legitimately
distinct symbols, and the statistical name categories do not independently
validate mangling grammar. A zero count would merely rule out this exact
pattern; it would not authorize broad name-stripping heuristics.
