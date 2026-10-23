# Numeric compiler and runtime

**Maintained implementation at development version 0.0.1:** Oxc parses JS/TS, a custom lowering
pass produces WAT, `wat` encodes it into real Wasm bytes, and Wasmtime executes them.
Node supplies an independent original-JS reference and a second Wasm engine.

## Replay

From the repository root, with current Rust stable and Node available:

```sh
# First setup only; subsequent verification is offline and locked:
cargo fetch --locked --manifest-path spikes/001-frontend-semantics/Cargo.toml
cargo fetch --locked
# Build the PoC, screen dependency metadata, and run comparisons:
node scripts/poc.mjs
# Also run both experiments' formatting and lint checks:
node scripts/check.mjs
```

The package script `npm run poc` is wired to the same replay. The workstation's
npm launch issue from spike001 remains unresolved, so the direct Node command is
the verified path. Bun and other platforms have not been exercised.

The harness prints a new root `target/compatibility-runs/run-*` directory containing every input,
emitted `.wat` and `.wasm`, rejection fixture, and `report.json`. Keep the printed
directory for inspection. Build outputs and replay directories are ignored and
retained as useful evidence; no servers, ports or background services are used.

### Compatibility corpus and report

[corpus.mjs](../tests/compatibility/corpus.mjs) versions the selected programs, expected rejections and
unimplemented surfaces independently of the replay harness. Corpus version 5
contains 96 paired programs, 108 rejection fixtures and five unexecuted feature
groups linked to milestone IDs. These are selected evidence and gaps, not an
inventory of the entire language or a compatibility percentage.

Corpus 5 includes 24 executed pairs and eight specific admission rejections adapted
from a pinned [TypeScript arithmetic fixture](../tests/compatibility/typescript/README.md).
The report records their upstream source, revision, hashes and ID/line mappings in
`externalSources`. These numeric adaptations do not claim upstream type conformance.

The JSON report separates each fixture's expected admission from its observed
test outcome. A passing rejection test means a diagnostic was produced and no
artifact was created; it does not count as supported execution. Unimplemented
entries carry `executed: false` and `outcome: "not-executed"`. Every executable
entry must produce its expected variants before a completed report is written.
A failed run exits nonzero with the fixture path or ID and diagnostics; partial
artifacts remain in the run directory, without a completed success report.

Report schema 1 records the corpus version, actual reference runtime/engine
versions, host OS/architecture, locked Wasmtime version, lockfile and compiler
binary SHA-256 hashes, seed, counts, per-fixture diagnostics and source/artifact
paths. Paths are relative to the run directory, except milestone links, which
are relative to the repository root. Source and Wasm hashes identify the exact
bytes used, including file-backed fixtures. Locked dependency versions describe
build inputs; they are not runtime introspection of an arbitrary binary.

## Concrete demo

The retained `tests/compatibility/fixtures/affine.ts` is:

```ts
export function run(a: number, b: number): number {
  return (a * 2 + b) / 3;
}
```

It emits **63 bytes of portable core Wasm**, identical to the JS fixture's output.
Calling `run(4, 1)` returns `3` under both Wasmtime and Node-Wasm. This is artifact
size only; it excludes the compiler and runtime and is not a platform footprint claim.

For manual inspection, use the built `target/debug/grunkalunka`
executable (`.exe` on Windows):

```text
grunkalunka compile SOURCE.ts NEW-OUTPUT.wasm
grunkalunka wat SOURCE.ts
grunkalunka eval OUTPUT.wasm < input-bits.txt
```

The `eval` harness accepts two space-separated 16-digit hexadecimal binary64
patterns per line and emits one result pattern per line. For example, inputs
`4010000000000000 3ff0000000000000` represent `4` and `1`; the affine result is
`4008000000000000` (`3`). Hex transport preserves signed zero and non-finite values.
The host compiles and instantiates once per invocation and evaluates the whole
batch. No imports or WASI are linked. `eval` is for locally generated artifacts,
not a hardened arbitrary-module execution service.

The implementation is split into `crates/grunkalunka-compiler` (source to portable
Wasm, no Wasmtime dependency), `crates/grunkalunka-runtime` (numeric module
instantiation and execution), and `crates/grunkalunka-cli` (files, standard input
and command handling). The root Cargo workspace and lockfile are authoritative.
The two libraries take source text or Wasm bytes; filesystem access remains in the CLI.

## Experimental contract

- Exactly one module-level `export function run` with two ordinary parameters.
  Inputs are **already primitive JS numbers**, represented as Wasm `f64` values.
  This ABI contract is independent of whether TS annotations are present.
- The body admits initialized numeric `const`/`let` declarations and simple
  assignment statements in source order, lexical blocks and statement `if`/`else`, and `while` loops.
  Early numeric returns are supported. Every structural path must return a number:
  a sequence must reach a guaranteed return; a block inherits that result; an `if`
  guarantees return only when both present arms do. Missing-return paths and
  statements following a guaranteed return are rejected, even if valid JavaScript.
  Each declaration has one identifier and initializer; the binding becomes available
  only after that initializer. References resolve through semantic symbol identities.
  Numeric expressions admit literals, initialized bindings, parentheses, binary
  `+ - * /`, unary `+ -`, and ternaries. Conditions admit numeric
  `< <= > >= === !==` comparisons or primitive-number truthiness. Truthiness
  evaluates its expression once; ±0 and NaN are false, all other numbers are true.
  Both ternary arms must be admitted and lower to structured Wasm
  `if (result f64)`. Optional TS annotations must be exactly `number`.
  Simple `=` expressions write to an initialized parameter or `let` binding and
  return the assigned number. The RHS runs once; Wasm `local.tee` preserves its
  value. Assignment statements discard that value. Writes to `const`, globals,
  self/forward locals, property/destructuring and TS-wrapped targets are rejected.
  Scope exit removes bindings from visibility without recycling their physical
  Wasm slots. Both branches are admitted independently before encoding. Explicit
  Wasm returns preserve completion; a final `unreachable` is a validator aid only
  after the structural return check succeeds. Each `while` is treated as possibly
  falling through, even when its body returns. A following numeric return is required
  on that path. Loop-body locals are initialized on each entry and disappear from
  visibility on exit; physical slots are not reused for sibling declarations.
- Everything else is rejected: extra statements/exports/imports, directives,
  hashbangs, defaults/rest/destructuring, async/generator/generic/declare functions,
  `this` parameters, modifiers, other annotations, calls, strings/objects/BigInts,
  casts/assertions, uninitialized locals, compound assignments, increment/decrement,
  `for`/`do`, `break`/`continue`, labels and logical operators. Boolean arithmetic/returns and self/forward local references are
  outside this admission, even in unselected branches. These rejections describe
  the experiment, not the validity of arbitrary JavaScript.
- Admission covers the entire AST, including directives/modifiers stored outside
  ordinary statements. Oxc parser and semantic syntax diagnostics are checked.
  The compiler does not run a TypeScript type checker or infer arbitrary JS types.
- AST grouping is preserved; there is no optimizer, reassociation, constant folding
  or fused multiply-add lowering. Literal values use Oxc's binary64 parse followed
  by round-trippable WAT decimal spelling (explicit `inf` for overflow literals).
- Source limit 16 KiB; expression nesting limit 128; at most 128 numeric locals
  across all lexical scopes and 256 recursively counted statements, including
  block/`if`/`while` nodes, both arms, loop bodies and returns; statement nesting limit 64;
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
this development build; neither compiler nor runtime was release-benchmarked.

The replay observed:

```text
PASS 236928 numeric comparisons; 108 rejected inputs; overwrite preserved
DEMO: affine.ts -> Wasm; run(4, 1) = 3
PASS all checks (Node v20.10.0)
```

Ninety-six programs, each written as JS and TS, produce 192 modules. Each module is
validated and run in Wasmtime and Node-Wasm over 617 argument pairs: the Cartesian
product of 19 named boundary values plus 256 reproducible raw-bit pairs from seed
`4752554e`. This yields 236,928 engine-to-JS comparisons. The explicit demo calls in
both engines are additional assertions. All JS/TS module pairs emit identical bytes.

The expressions cover basic operators, nested unary operators, signed-zero and
smallest-subnormal literals, overflow literals, integer literals rounded beyond
`2^53`, nondecimal literals, grouping/cancellation and separate rounding. Input
vectors include ±0, subnormals, extremes, infinities and NaN. Non-NaN results compare
exactly (including signed zero); NaNs compare by class, not payload or sign.

The added programs cover every comparison, NaN predicates, signed-zero branches,
nested conditions, chained locals, Unicode identifiers and local/function-name
shadowing. The original L0 run had 39,488 comparisons and 48 rejections.

Mutation fixtures observe assignment values and later reads, chained assignments,
statement/operand evaluation order, effects during initialization and comparisons,
selected-branch effects and signed zero. A boundary fixture uses 128 locals and
256 statements. Removing the emitted assignment state update leaves valid Wasm
but causes the corpus to fail on an observed result mismatch. This demonstrates
sensitivity to lost writes; it is not exhaustive mutation testing.
Simple identifier targets have no observable target-evaluation effects here;
general property-target assignment behavior remains untested.

Block/branch fixtures cover same-named sibling locals, outer mutation, parameter
shadowing, initialization boundaries, dangling `else`, both-arm return and one-arm
early return followed by a fallback. Negative fixtures cover scope leaks, branch
fallthrough, unreachable extra statements and recursive limits. Replacing a nested
early return with `drop` still validates as Wasm but returns the fallback value;
the behavioral comparison detects it. Loop fixtures are finite independently of
input values and test zero iterations, nested loops, body initialization, final
condition effects and early return. Omitting the loop backedge emits valid Wasm
but fails the differential result check. A separate Rust runtime test deliberately
exhausts fuel, checks `Trap::OutOfFuel`, then successfully reuses the same instance.
It never executes the infinite fixture in JavaScript or Node-Wasm. Fuel bounds
apply to Wasmtime execution; this is not a static termination proof or a universal
execution bound across engines. The affine artifact grew from 61 to 63 bytes
when explicit return/completion handling was added.

One hundred and eight malformed or unsupported programs fail explicitly, each with a unique
absent artifact destination. A separate sentinel verifies refusal to overwrite an
existing file. A mismatch reports the seed, fixture, vector and input/result bits.
Formatting, Clippy with warnings denied, both earlier frontend checks, and resolved
dependency-license screening pass. The first Clippy run reported a collapsible
conditional; it was simplified and all affected checks rerun without suppression.

Dependency metadata: 192 packages in the workspace graph, 193 distinct versions
including the retained frontend research graph, with permissible selected alternatives and no selected GPL/LGPL license.
See [license screening](dependencies.md) for scope, alternatives and limitations.

## What this tells us

**Observed:** an AST-to-Wasm path preserves results for this admitted numeric ABI
and corpus under two Wasm engines. Unsupported code fails instead of silently
changing behavior. This is stronger evidence than a hand-written Wasm demo because
the emitted instructions originate from the parsed JS/TS expressions.

**Inference:** continuing with a custom lowering experiment is practical. No stable
IR or full-language frontend architecture follows from this one result.

The numeric ABI exercises local mutation and its observable evaluation order. It
does not establish behavior for external effects, coercion of objects/strings,
functions, closures, exceptions, promises or modules.

SHORTCUT: core Wasm and a tiny typed host — sufficient for arithmetic code generation;
does not test WIT/Component Model interfaces or capability resource semantics.

SHORTCUT: WAT as an inspectable encoding step — exposes the exact generated
instructions without committing to a production emitter or optimizer.

The next frontend question is the tsrs type-query/source-identity experiment. The
separate [WIT capability host experiment](capabilities.md) now exercises resource
profiles and lifecycle checks. Neither is a hidden
prerequisite for the numeric ABI test, and neither was implemented here.
