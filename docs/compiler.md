# Numeric compiler and runtime

**Maintained implementation at development version 0.0.1:** Oxc parses JS/TS, a custom lowering
pass builds and verifies [typed numeric regions](ir.md), then emits WAT. `wat`
encodes it into real Wasm bytes, and Wasmtime executes them.
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
unimplemented surfaces independently of the replay harness. Corpus version 15
contains 240 paired programs, 288 rejection fixtures and five unexecuted feature
groups linked to milestone IDs. These are selected evidence and gaps, not an
inventory of the entire language or a compatibility percentage.

Corpus 5 includes 24 executed pairs and eight specific admission rejections adapted
from a pinned [TypeScript arithmetic fixture](../tests/compatibility/typescript/README.md).
The report records their upstream source, revision, hashes and ID/line mappings in
`externalSources`. These numeric adaptations do not claim upstream type conformance.

Corpus 6 also includes [four Test262 numeric adaptations](../tests/compatibility/test262/README.md),
covering 18 original assertions with independent zero-success checks.

Corpus 7 adds an [instrumented Mandelbrot orbit](../tests/compatibility/benchmarks-game/README.md)
with a separate original-loop JS oracle and ten named input pairs. Every fixture
retains its oracle source and hash separately from JS/TS compilation inputs.

Corpus 8 adds seven finite composition regressions for ordered operand writes,
comparison RHS effects after NaN, ternary state, per-entry initializer effects,
terminal loop-condition effects and restoration of written shadow bindings.
These exercise the existing numeric admission; they do not add language support.

Corpus 9 adds [72 lowering-gap references](lowering-gaps.md). Each runs independently
in Node against a specified result, then requires the current compiler's specific
rejection and absence of an artifact. Reports distinguish successful reference
execution from unexecuted lowering; these cases do not count as compiler support.

Corpus 10 promotes compound assignment, prefix/postfix increments and parenthesized
write statements into numeric support. Twenty-eight paired regressions cover all
four compound arithmetic operators, increments/decrements, result snapshots, RHS
ordering, lexical mutation and terminal loop-condition effects. That run left
68 reference-validated gaps.

Corpus 11 promotes the four TypeScript expression-wrapper gaps into 28 admitted
JS/TS pairs, including nested wrappers, side effects, predicates, depth bounds,
and four combinations with compound assignment and prefix/postfix updates.
The obsolete type-assertion rejection is retired; 35 focused boundary rejections
check unsupported operands, targets, statements, initialization and nesting.
That run left 64 lowering-gap references.

Corpus 12 adds 15 numeric comma-sequence regressions plus two TS-wrapper
compositions. Every operand runs left to right and the final operand supplies the
value. In a predicate position, the final operand may be a comparison; preceding
operands remain numeric. Comma statements remain rejected. That run left 63 reference-validated gaps.

Corpus 13 adds 19 paired numeric `&&`/`||` programs, including two TS-wrapper
compositions. Left operands execute once; the RHS executes only when selected,
and the result is an operand snapshot rather than a normalized Boolean. Both
operands must remain numeric, including in predicate positions. Nullish coalescing
and Boolean operands remain rejected. That run left 61 reference-validated gaps.

Corpus 14 promotes numeric `for` loops with 24 paired programs and 30 boundary
rejections. The original `gap-for` reference and obsolete blanket `for-loop`
rejection are retired. That run left 60 lowering-gap references.

Corpus 15 adds 16 paired Boolean-literal condition cases, including finite
`for` conditions. `true` and `false` produce typed predicates in conditions and
final predicate-position comma operands; source locals and returns stay numeric.
Boolean-valued ternary arms, Boolean equality operands and Boolean logical
operands remain rejected. The frontier still contains 60 reference-validated gaps.

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

It emits **93 bytes of portable core Wasm**, identical to the JS fixture's output.
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
- The body admits initialized numeric `const`/`let` declarations, assignment and
  update statements in source order, lexical blocks and statement `if`/`else`, and `while`/`for` loops.
  Early numeric returns are supported. Every structural path must return a number:
  a sequence must reach a guaranteed return; a block inherits that result; an `if`
  guarantees return only when both present arms do. Missing-return paths and
  statements following a guaranteed return are rejected, even if valid JavaScript.
  Each declaration has one identifier and initializer; the binding becomes available
  only after that initializer. References resolve through semantic symbol identities.
  Numeric expressions admit literals, initialized bindings, parentheses, binary
  `+ - * /`, unary `+ -`, and ternaries. Conditions admit Boolean literals, numeric
  `< <= > >= === !==` comparisons and primitive-number truthiness. Truthiness
  evaluates its expression once; ±0 and NaN are false, all other numbers are true.
  Numeric comma operands execute left to right and yield the final value snapshot;
  in predicate positions the final operand may instead be a comparison. Non-final
  comparisons and comma expression statements remain outside admission.
  Numeric `&&` and `||` evaluate the left operand once and return its original
  snapshot or the selected RHS result. The RHS is fully admitted but executes only
  in the selected branch. Boolean/comparison operands and `??` remain rejected.
  Both ternary arms must be admitted and lower to structured Wasm
  `if (result f64)`. Optional TS annotations must be exactly `number`.
  TypeScript `as`, angle-bracket assertions, postfix non-null `!`, and `satisfies`
  are erased in expression and predicate positions. Each wrapper counts toward
  nesting, and its runtime operand must independently satisfy admission. Wrapped
  comparisons stay predicates. No type checking or runtime conversion follows
  from a wrapper's asserted type; declaration annotations retain their restrictions.
  `=`, `+=`, `-=`, `*=` and `/=` expressions write to an initialized parameter or
  `let` binding and return the assigned number. Compound operations snapshot the
  target before evaluating the RHS. Prefix/postfix `++` and `--` store the updated
  number and return the new/old snapshot respectively. The RHS runs once; a typed
  IR value and separate
  Wasm temporary preserve its snapshot across the store. Assignment statements
  and update statements leave their results unused; parentheses around these
  writes are admitted. Writes to `const`, globals,
  self/forward locals, property/destructuring and TS-wrapped targets are rejected.
  Scope exit removes bindings from visibility without recycling their physical
  Wasm slots. Both branches are admitted independently before encoding. Explicit
  Wasm returns preserve completion; a final `unreachable` is a validator aid only
  after the structural return check succeeds. Each `while` is treated as possibly
  falling through, even when its body returns. A following numeric return is required
  on that path. Loop-body locals are initialized on each entry and disappear from
  visibility on exit; physical slots are not reused for sibling declarations.
  Numeric `for` initializers may be omitted, one initialized `let`/`const`, or an
  admitted numeric expression. Tests use the admitted predicate forms, or may be omitted for an
  unconditional test; updates are numeric expressions or omitted. Initialization
  runs once, then each iteration tests, executes the body, and executes the update.
  A return skips the update, but the compiler still admits the whole source body
  and update independently. Header bindings are visible only inside the loop;
  body locals are invisible to the update. `for` remains possibly falling through
  for structural return checking, including with an omitted test. The supported
  numeric subset cannot observe per-iteration environment identity because captures
  and `eval` remain unsupported.
- Everything else is rejected: extra statements/exports/imports, directives,
  hashbangs, defaults/rest/destructuring, async/generator/generic/declare functions,
  `this` parameters, modifiers, other annotations, calls, strings/objects/BigInts,
  type-only declarations, uninitialized locals and compound operators beyond the four above,
  `do`, `for-in`/`for-of`, `break`/`continue`, labels and nullish coalescing. Boolean arithmetic/returns and self/forward local references are
  outside this admission, even in unselected branches. These rejections describe
  the experiment, not the validity of arbitrary JavaScript.
  TypeScript-wrapped assignment/update targets and wrappers around an entire
  expression statement are still rejected.
- Admission covers the entire AST, including directives/modifiers stored outside
  ordinary statements. Oxc parser and semantic syntax diagnostics are checked.
  The compiler does not run a TypeScript type checker or infer arbitrary JS types.
- AST grouping is preserved; there is no optimizer, reassociation, constant folding
  or fused multiply-add lowering. Literal values use Oxc's binary64 parse followed
  by round-trippable WAT decimal spelling (explicit `inf` for overflow literals).
- Source limit 16 KiB; expression nesting limit 128; at most 128 numeric locals
  across all lexical scopes and 256 recursively counted statements, including
  block/`if`/`while`/`for` nodes, both arms, loop bodies and returns; a `for` header
  declaration counts once as a statement and once as a local. Header expressions
  use the ordinary expression nesting budget. Statement nesting limit 64;
  generated IR values, regions and Wasm temporaries do not consume source budgets;
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
PASS 601920 numeric comparisons; 288 rejected inputs; overwrite preserved
DEMO: affine.ts -> Wasm; run(4, 1) = 3
PASS all checks (Node v20.10.0)
```

240 programs, each written as JS and TS, produce 480 modules.
Each module is validated and run in Wasmtime and Node-Wasm over 627 argument pairs: the Cartesian
product of 19 named boundary values plus 256 reproducible raw-bit pairs from seed
`4752554e`, plus ten named Mandelbrot orbit pairs. This yields 601,920 engine-to-JS
comparisons. The explicit demo calls in both engines are additional assertions. All JS/TS module pairs emit identical bytes.

The [TypeScript erasure fixtures](../tests/compatibility/typescript-erasure.mjs)
use hand-paired JavaScript references. They check all four wrappers around numbers
and comparisons, nested forms, ordered writes, snapshots, loop-condition effects,
NaN/±0 behavior and the 128-node nesting boundary. Calls, strings and objects remain
rejected inside wrappers, including unselected arms. Const writes, uninitialized
bindings, wrapped targets and statements, and depth 129 also remain rejected.
This follows TypeScript's runtime treatment of
[assertions and non-null expressions](https://www.typescriptlang.org/docs/handbook/2/everyday-types.html#type-assertions)
and the type-only role of [`satisfies`](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-9.html#the-satisfies-operator),
without claiming upstream type checking or general TypeScript compatibility.

The [numeric for fixtures](../tests/compatibility/numeric-for.mjs) use finite,
input-independent counters or bounded returns. They test header omissions,
initializer-once and terminal-test effects, update ordering, return skipping the
update, nested shadows and body-local initialization. A composed header combines a
numeric logical condition with comma/update effects in the step. Exact source limits include
the header declaration charge and exclude generated regions. A finite mutation
moving the step before the body changes a result from 12 to 123 and is detected.
The admitted order follows [ForBodyEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-forbodyevaluation);
captures and observable per-iteration environment identity remain outside admission.

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

The numeric region emitter materializes ordered computations and source reads in
separate typed Wasm locals. The affine artifact consequently grows from 63 to 93
bytes (+30 bytes); the retained Mandelbrot orbit grows from 273 to 490 bytes
(+217 bytes). Both retain their results under the unchanged 10,000-fuel allowance.
These measurements describe artifact growth, with no execution-speed claim.

288 malformed or unsupported programs fail explicitly, each with a unique
absent artifact destination. A separate sentinel verifies refusal to overwrite an
existing file. A mismatch reports the seed, fixture, vector and input/result bits.
Formatting, Clippy with warnings denied, both earlier frontend checks, and resolved
dependency-license screening pass. The first Clippy run reported a collapsible
conditional; it was simplified and all affected checks rerun without suppression.

Dependency metadata: 192 packages in the workspace graph, 193 distinct versions
including the retained frontend research graph, with permissible selected alternatives and no selected GPL/LGPL license.
See [license screening](dependencies.md) for scope, alternatives and limitations.

## What this tells us

**Observed:** lowering parsed syntax through verified numeric regions to Wasm
preserves results for this admitted numeric ABI and corpus under two Wasm engines.
Unsupported code fails instead of silently
changing behavior. This is stronger evidence than a hand-written Wasm demo because
the emitted instructions originate from the parsed JS/TS expressions.

**Inference:** continuing with a custom lowering experiment is practical. The
internal numeric region IR now exists and has a verifier; its limited source
contract does not settle a stable full-language IR or frontend architecture.

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
