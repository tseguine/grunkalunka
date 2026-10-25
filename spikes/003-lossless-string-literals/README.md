# 003 — Lossless JS String Literals

Disposable evidence probe. Do not promote this harness or its Oxc-specific
structures into production interfaces.

## Question and result

**Does `JSStr::encode_utf16()` produce the same UTF-16 code unit sequence
that Node/V8 sees at runtime, across all adversarial JS string literal forms?**

**Yes.** 27 cases across 6 fixture files — basic escapes, Unicode escapes,
astral characters, lone surrogates, line continuations, and miscellaneous
edge cases — all passed. Oxc's `encode_utf16()` iterator is lossless and
agrees with Node v20.10.0 on every code unit.

## Replay

From the repository root (stable toolchain via `rust-toolchain.toml`):

```sh
cargo run --offline -p grunkalunka-string-spike \
  | node spikes/003-lossless-string-literals/oracle.mjs
```

First run after cloning requires `cargo fetch --locked` (or removing `--offline`)
to populate the registry cache. Subsequent runs use only cached artifacts.

## Observed evidence — 2026-10-25

Environment: Windows `x86_64-pc-windows-msvc`; rustc 1.99.1
(`58cf8f9c3`); Cargo 1.99.1; Node v20.10.0; debug/dev profile.

```
OK   01-basic-escapes.js: 6 case(s)
OK   02-unicode-escapes.js: 5 case(s)
OK   03-astral.js: 3 case(s)
OK   04-lone-surrogates.js: 6 case(s)
OK   05-line-continuation.js: 2 case(s)
OK   06-misc.js: 5 case(s)

PASS 27  FAIL 0  TOTAL 27
```

| Fixture | Cases | Representative finding |
|---|---|---|
| `01-basic-escapes.js` | 6 | `"\n"` → `[0x000a]`, `"\\"` → `[0x005c]`; standard single-escape chars unescaped correctly |
| `02-unicode-escapes.js` | 5 | `"\u{1F600}"` → `[0xd83d, 0xde00]` (surrogate pair); `"\u{10FFFF}"` → `[0xdbff, 0xdfff]` |
| `03-astral.js` | 3 | Literal 😀 and `\uD83D\uDE00` both yield `[0xd83d, 0xde00]`; indistinguishable as required |
| `04-lone-surrogates.js` | 6 | `"\uD800"` → `[0xd800]`; `"\u0041\uDC00"` → `[0x0041, 0xdc00]`; lone surrogates preserved exactly |
| `05-line-continuation.js` | 2 | `"abc\<newline>def"` → `[0x0061…0x0066]` (6 units, newline omitted); continuation stripped by Oxc correctly |
| `06-misc.js` | 5 | `""` → `[]`; `"\u0000"` → `[0x0000]`; NUL preserved |

**Critical finding — lone surrogates:** `JSStr` stores lone surrogates in
canonical WTF-8 (three bytes per lone surrogate, never paired). `encode_utf16()`
decodes them back to the original `u16` code unit. Node/V8 stores these as
UTF-16 and also returns the raw code unit from `charCodeAt()`. They agree
exactly — there is **no lossy conversion** at any point in the Oxc pipeline
for the cases exercised.

**Directive vs body distinction:** Oxc (like ESTree) places leading bare
string literals in `program.directives`, not `program.body`. Any consumer
extracting string literals from the AST must check both. The probe handles
this; production lowering code must too.

## Interpretation

- **Observation:** All 27 Oxc `encode_utf16()` outputs match Node's `charCodeAt`
  sequence. **Inference:** The compiler can safely call `sl.value.encode_utf16()`
  on a `StringLiteral` node and iterate the `u16` values to produce the
  correct JS runtime string — including for lone surrogates and astral characters.

- **Observation:** Line continuations yield a string with no trace of the
  backslash-newline. **Inference:** Oxc strips line continuations during
  parsing; production code does not need to handle them post-parse.

- **Observation:** `\uD83D\uDE00` (explicit surrogate pair escape) and the
  literal 😀 glyph produce identical code unit sequences. **Inference:** The
  AST value is already the JS runtime value; source form is irrelevant after
  parsing.

## Shortcut ledger

```
SHORTCUT: fixtures are hand-written, not exhaustive — cannot affect decision
  evidence because the adversarial cases (lone surrogates, astral, line
  continuation, NUL) are the specific boundary conditions the decision requires;
  a conformance suite is a separate concern.

SHORTCUT: debug build only, no release profile — cannot affect decision
  evidence because encode_utf16() is a pure iterator over WTF-8 bytes; its
  correctness is independent of optimization level.

SHORTCUT: only StringLiteral nodes tested, not TemplateLiteral cooked values —
  TemplateElementValue.cooked is also Option<JSStr<'a>> and uses the same
  encode_utf16(); the same inference applies but is not directly verified here.
  Template literals with invalid escapes (cooked = None) are a separate case.

SHORTCUT: no Wasm or runtime integration — cannot affect decision evidence
  because the question is about the Oxc-side extraction, not the downstream
  Wasm encoding; that is a separate concern.
```

## Recommendation

**Use `sl.value.encode_utf16().collect::<Vec<u16>>()` as the canonical string
extraction path for L4 string lowering.**

The iterator is correct and lossless for all tested literal forms. The
recommended production implementation:

1. At the AST lowering stage, call `string_literal.value.encode_utf16()` to
   obtain an ordered sequence of `u16` code units.
2. Store or emit these code units as the runtime string representation (e.g.,
   a length-prefixed `u16` array in Wasm linear memory, or a WIT `string`
   encoded as UTF-16 for JS interop — subject to the separate heap-layout spike).
3. **Do not** call `sl.value.as_str()` and treat `None` as an error — lone
   surrogate strings are valid JS and must be handled.
4. The `program.directives` field must be visited alongside `program.body`
   when walking top-level string expressions.

## Alternatives and uncertainty

- **WTF-8 bytes directly**: `JSStr::as_bytes()` + `has_lone_surrogate` are
  available for consumers that want the WTF-8 encoding. This is fine for
  in-process storage but requires WTF-8-aware downstream consumers. The
  `encode_utf16()` path is simpler and matches the JS runtime model.
- **Template literal cooked values**: `TemplateElementValue.cooked` is
  `Option<JSStr<'a>>`; `None` means an invalid escape sequence (tagged
  template raw-only mode per ECMA-262 §13.2.8.3). This case is not directly
  verified but uses the same `JSStr` type; a follow-up fixture should confirm.
- **Heap layout for emitted strings**: How the emitted `u16` sequence is
  stored in Wasm memory (length prefix, sentinel, arena block, etc.) is the
  subject of the separate heap-ownership spike and is not answered here.
