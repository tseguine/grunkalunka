# 002 — JS/TS numeric expressions to Wasm

**A working, disposable compiler fragment:** Oxc parses JS/TS, a custom lowering
pass produces WAT, `wat` encodes it into real Wasm bytes, and Wasmtime executes them.
Node supplies an independent original-JS reference and a second Wasm engine.

## Replay

From the repository root, with current Rust stable and Node available:

```sh
# First setup only; subsequent verification is offline and locked:
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo fetch --locked --manifest-path spikes/002-numeric-wasm/Cargo.toml
# Build the PoC, screen dependency metadata, and run comparisons:
node scripts/poc.mjs
# Also run both experiments' formatting and lint checks:
node scripts/check.mjs
```

The package script `npm run poc` is wired to the same replay. The workstation's
npm launch issue from spike001 remains unresolved, so the direct Node command is
the verified path. Bun and other platforms have not been exercised.

The harness prints a new `target/poc-runs/run-*` directory containing every input,
emitted `.wat` and `.wasm`, rejection fixture, and `report.json`. Keep the printed
directory for inspection. Build outputs and replay directories are ignored and
retained as useful evidence; no servers, ports or background services are used.

## Concrete demo

The retained `fixtures/affine.ts` is:

```ts
export function run(a: number, b: number): number {
  return (a * 2 + b) / 3;
}
```

It emits **61 bytes of portable core Wasm**, identical to the JS fixture's output.
Calling `run(4, 1)` returns `3` under both Wasmtime and Node-Wasm. This is artifact
size only; it excludes the compiler and runtime and is not a platform footprint claim.

For manual inspection, use the built `target/debug/grunkalunka-numeric-spike`
executable (`.exe` on Windows):

```text
grunkalunka-numeric-spike compile SOURCE.ts NEW-OUTPUT.wasm
grunkalunka-numeric-spike wat SOURCE.ts
grunkalunka-numeric-spike eval OUTPUT.wasm < input-bits.txt
```

The `eval` harness accepts two space-separated 16-digit hexadecimal binary64
patterns per line and emits one result pattern per line. For example, inputs
`4010000000000000 3ff0000000000000` represent `4` and `1`; the affine result is
`4008000000000000` (`3`). Hex transport preserves signed zero and non-finite values.
The host compiles and instantiates once per invocation and evaluates the whole
batch. No imports or WASI are linked. `eval` is for locally generated artifacts,
not a hardened arbitrary-module execution service.

## Experimental contract

- Exactly one module-level `export function run` with two ordinary parameters.
  Inputs are **already primitive JS numbers**, represented as Wasm `f64` values.
  This ABI contract is independent of whether TS annotations are present.
- The body admits numeric `const` declarations followed by one numeric return.
  Each declaration has one identifier and initializer; the binding becomes available
  only after that initializer. References resolve through semantic symbol identities.
  Numeric expressions admit literals, initialized bindings, parentheses, binary
  `+ - * /`, unary `+ -`, and comparison-controlled ternaries. Conditions use numeric
  `< <= > >= === !==`; both arms must be admitted and lower to structured Wasm
  `if (result f64)`. Optional TS annotations must be exactly `number`.
- Everything else is rejected: extra statements/exports/imports, directives,
  hashbangs, defaults/rest/destructuring, async/generator/generic/declare functions,
  `this` parameters, modifiers, other annotations, calls, strings/objects/BigInts,
  casts/assertions, mutation, statement `if`, loops, logical operators and numeric
  truthiness. Boolean arithmetic/returns and self/forward local references are
  outside this admission, even in unselected branches. These rejections describe
  the experiment, not the validity of arbitrary JavaScript.
- Admission covers the entire AST, including directives/modifiers stored outside
  ordinary statements. Oxc parser and semantic syntax diagnostics are checked.
  The compiler does not run a TypeScript type checker or infer arbitrary JS types.
- AST grouping is preserved; there is no optimizer, reassociation, constant folding
  or fused multiply-add lowering. Literal values use Oxc's binary64 parse followed
  by round-trippable WAT decimal spelling (explicit `inf` for overflow literals).
- Source limit 16 KiB; expression nesting limit 128; at most 128 const locals;
  host input batches at most 4096
  pairs and 10,000 fuel per call. Each harness subprocess has a timeout; the vector
  replay has a three-minute bound. These are experiment bounds, not platform policy.
- Compile finishes parsing/admission/encoding in memory before opening a fresh
  output file. Existing destinations are refused. Expected compile rejection must
  leave its previously nonexistent destination absent.

Calling a raw generated export with a string, missing argument or object is outside
the ABI: JS WebAssembly bindings can perform their own conversions. A future
application boundary must specify/guard its supported argument semantics. A TS
`number` annotation is not used to justify accepting those calls or erasing coercions.

## Observations — 2026-10-23

Environment: Windows `x86_64-pc-windows-msvc`; rustc/Cargo 1.99.1; Node v20.10.0;
Oxc 0.155.0; Wasmtime 50.0.0 with Cranelift/runtime/std/anyhow features, defaults off;
wat 1.262.0. Cargo lockfile pins transitives. Dev build, Rust debug info disabled in
this spike; neither compiler nor runtime was release-benchmarked.

The replay observed:

```text
PASS 76508 numeric comparisons; 64 rejected inputs; overwrite preserved
DEMO: affine.ts -> Wasm; run(4, 1) = 3
PASS all checks (Node v20.10.0)
```

Thirty-one programs, each written as JS and TS, produce 62 modules. Each module is
validated and run in Wasmtime and Node-Wasm over 617 argument pairs: the Cartesian
product of 19 named boundary values plus 256 reproducible raw-bit pairs from seed
`4752554e`. This yields 76,508 engine-to-JS comparisons. The explicit demo calls in
both engines are additional assertions. All JS/TS module pairs emit identical bytes.

The expressions cover basic operators, nested unary operators, signed-zero and
smallest-subnormal literals, overflow literals, integer literals rounded beyond
`2^53`, nondecimal literals, grouping/cancellation and separate rounding. Input
vectors include ±0, subnormals, extremes, infinities and NaN. Non-NaN results compare
exactly (including signed zero); NaNs compare by class, not payload or sign.

The added programs cover every comparison, NaN predicates, signed-zero branches,
nested conditions, chained locals, Unicode identifiers and local/function-name
shadowing. The original L0 run had 39,488 comparisons and 48 rejections.

Sixty-four malformed or unsupported programs fail explicitly, each with a unique
absent artifact destination. A separate sentinel verifies refusal to overwrite an
existing file. A mismatch reports the seed, fixture, vector and input/result bits.
Formatting, Clippy with warnings denied, both earlier frontend checks, and resolved
dependency-license screening pass. The first Clippy run reported a collapsible
conditional; it was simplified and all affected checks rerun without suppression.

Dependency metadata: 175 packages in this graph, 176 distinct versions across both
experiments, with permissible selected alternatives and no selected GPL/LGPL license.
See [license screening](LICENSES.md) for scope, alternatives and limitations.

## What this tells us

**Observed:** an AST-to-Wasm path preserves results for this admitted numeric ABI
and corpus under two Wasm engines. Unsupported code fails instead of silently
changing behavior. This is stronger evidence than a hand-written Wasm demo because
the emitted instructions originate from the parsed JS/TS expressions.

**Inference:** continuing with a custom lowering experiment is practical. No stable
IR or full-language frontend architecture follows from this one result.

SHORTCUT: pure numeric expressions — cannot establish behavior for side effects,
coercion of objects/strings, functions, closures, exceptions, promises or modules.
Left/right operand ordering is preserved structurally, but observable side-effect
ordering is not tested because side effects are rejected.

SHORTCUT: core Wasm and a tiny typed host — sufficient for arithmetic code generation;
does not test WIT/Component Model interfaces or capability resource semantics.

SHORTCUT: WAT as an inspectable encoding step — exposes the exact generated
instructions without committing to a production emitter or optimizer.

The next frontend question is the tsrs type-query/source-identity experiment. The
next platform question is a separate WIT capability host. Neither is a hidden
prerequisite for the numeric ABI test, and neither was implemented here.
