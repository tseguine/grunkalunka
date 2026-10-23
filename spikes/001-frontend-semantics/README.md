# 001 — Frontend semantics

Disposable research probe. Do not promote this harness or its Oxc-specific data
structures into production interfaces.

## Question and result

Can Oxc expose useful lexical bindings and diagnostics for the same small program
written as JavaScript and TypeScript?

Yes, for the exercised fixtures. Oxc 0.155.0 exposes exact shadowed and captured
reference targets, unresolved names, and separate parser/semantic diagnostics.
It accepts the deliberately invalid TypeScript number annotation without a
diagnostic. This is lexical analysis, not a substitute for a TS checker.

The result supports continuing to a tsrs type-query/identity experiment. It does
not establish JS compatibility, a compiler pipeline, capability enforcement, or
the best frontend architecture.

## Replay

From the repository root:

```sh
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo run --manifest-path spikes/001-frontend-semantics/Cargo.toml --locked --offline
node spikes/001-frontend-semantics/oracle.mjs
```

The repository-wide `node scripts/check.mjs` additionally executes formatting,
Clippy with warnings denied, and the later numeric PoC. Its setup requires fetching
both spikes' locked dependencies, as documented in the root README. Cargo
verification runs locked and offline after that setup; child failures fail the runner.

The provided package entry is `npm run check`. On this workstation it currently
fails through the local npm launch path before entering the script: `'node' is not
recognized as an internal or external command`. Direct `node scripts/check.mjs`
passes. A process-local PATH adjustment did not resolve the npm launch; no global
configuration was changed. Nodist is installed, but the root cause is unconfirmed.
This is a recorded package-entry verification gap.
Bun is not installed and its execution path remains unverified.

## Observed evidence — 2026-10-23

Environment: Windows `x86_64-pc-windows-msvc`; rustc 1.99.1
(`58cf8f9c3`, LLVM 23.1.1); Cargo 1.99.1; Node v20.10.0; debug/dev profile.
The repo selects the moving `stable` Rust channel; these versions identify this
run. Four direct Oxc crates are pinned to 0.155.0; `Cargo.lock` pins transitives.
JS and TS are explicitly parsed as ES modules. No timing claim is made.

| Fixture | JS / TS bytes | Symbols (each) | Resolved references (each) | Observation |
| --- | --- | --- | --- | --- |
| bindings | 136 / 168 | 5 | 4 | Both `x` references target their exact declarations; the inner parameter is captured by a closure. Node returns 42. |
| ambient | 46 / 54 | 1 | 0 | Only `clock` is unresolved. Invoking the JS fixture under Node throws `ReferenceError`. |
| coercion | 227 / 257 | 4 | 5 | Node returns 5, evaluation/conversion order `rhs,valueOf`, string `23`, negative infinity, and NaN. |
| type-mismatch | 62 / 78 | 2 | 1 | Both parse/analyze without diagnostics; the annotation-erased JS baseline returns a string. |

These byte counts reflect the retained LF fixtures. Source line ending conversion
can change byte counts and offsets; assertions derive offsets from each input.

The Rust probe compares the per-name binding/reference-count shape of each pair,
and separately checks exact source spans for both shadowed `x` symbols. Unexpected
parser diagnostics, fatal errors, semantic diagnostics or unresolved names fail it.

The negative probes observed:

```text
PASS invalid syntax: parser diagnostics=2
PASS duplicate binding: semantic diagnostics=1
PASS: 8 module fixtures + 2 rejection probes; type mismatch accepted (no TS type checker)
PASS all checks (Node v20.10.0)
```

The malformed function must produce parser diagnostics. The duplicate binding must
parse without recovery/fatal errors and then produce semantic diagnostics with
`SemanticBuilder::new_compiler()` (syntax checking enabled).

Development failures, now resolved: the first dependency fetch preceded creation
of a Rust target; the first compilation used an obsolete parser-return field name.
The installed source identifies `fatal_error`, and the corrected build/probe pass.
The two npm launcher failures above remain unresolved; they do not invalidate the
directly executed checks. No timing, memory or release footprint experiment ran.

## Interpretation, shortcuts and limits

- **Observation:** the paired source has matching binding shapes and the checked
  references resolve correctly. **Inference:** Oxc can supply inspectable lexical
  information to a later lowering experiment. This says nothing about all JS scopes.
- **Observation:** the mismatch gets no Oxc diagnostic. **Inference:** integrate a
  type checker separately; first compare a dual-frontend path with tsrs alone.
- **Observation:** `clock` is unresolved. This is not capability enforcement. Aliases,
  dynamic lookup, `eval`, imported authority and host policy are outside this probe.
- SHORTCUT: four small, hand-paired module fixtures — useful for the narrow binding
  observations, insufficient for language conformance or automatic TS erasure.
- SHORTCUT: Node executes original JS only — establishes fixture behavior without
  any generated output; this is not differential compiler verification.
- SHORTCUT: no object runtime, lowering, WIT, Wasmtime, tsrs or module resolver —
  none is necessary to inspect Oxc's lexical facts; no feasibility claim about
  those systems follows from this result.
- Type-only aliases, declaration merging, imports, UTF-16/span conversion, dynamic
  scope, temporal dead zones, exceptions, async and prototypes remain untested.
  Paired shape equality here is not a general cross-frontend identity algorithm.

## Next decision

Retain Oxc as a frontend candidate, not as an adopted architecture. Next, pin tsrs
and demonstrate a real type query linked to an expression while checking how its
AST/symbol identities relate to Oxc's. Compare against using tsrs alone before
building an adapter. See the [roadmap](../../docs/roadmap.md) for independent lowering,
component capability, and measurement experiments.

Retain this directory, small fixture corpus and lockfile as replayable evidence.
Generated `target/` files are ignored reusable build outputs. No services or ports
are used. The prototype has no production integration or published package.
