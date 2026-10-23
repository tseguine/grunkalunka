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
unimplemented surfaces independently of the replay harness. Corpus version 24
contains 404 paired programs, 411 rejection fixtures and five unexecuted feature
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
operands remain numeric. At that stage, comma statements were rejected. That run left 63 reference-validated gaps.

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
operands remain rejected. That run still contained 60 reference-validated gaps.

Corpus 16 promotes numeric `do...while` loops with 21 paired programs and 18
boundary rejections. The original `gap-do` reference and blanket `do-loop`
rejection are retired. That run left 59 lowering-gap references.

Corpus 17 adds 15 paired logical-negation cases. `!` accepts an admitted predicate
or numeric truthiness input and produces a predicate, with its operand evaluated
once. Nested negation, negated comparisons and loop conditions are included.
The `gap-unary-not` reference is promoted; that run left 58 lowering-gap references.
Numeric locals, arithmetic and returns still reject Boolean values.

Corpus 18 admits Number-only `==` and `!=` predicates with 19 paired programs
and 20 boundary rejections. The numeric loose-equality rejection and
`gap-loose-inequality` reference are promoted; coercive equality remains rejected.
That run left 57 lowering-gap references.
Corpus 19 adds 21 paired logical-condition cases. `&&` and `||` may combine
numeric truthiness, comparisons, Boolean literals and negation in predicate
positions. The left operand runs once and the right runs only when selected;
both source operands must still satisfy admission. Numeric value expressions
retain their original selected operand rather than a normalized Boolean.
Boolean locals/returns, Boolean-valued ternary arms and equality operands remain
outside admission. That run retained 57 lowering-gap references.

Corpus 20 admits discarded numeric expression statements with 20 paired programs
and 20 boundary rejections. Three numeric-statement and four whole-TypeScript-wrapper
rejections are promoted. The lowering-gap frontier remains 57 references.

Corpus 21 adds 19 paired conditional-predicate cases. In predicate positions,
ternaries may select Boolean, comparison, negation or numeric-truthiness arms.
The test runs once and only the selected arm executes, while both arms undergo
source admission. Numeric-value ternaries keep numeric arms. Boolean locals,
returns, arithmetic and equality operands remain unsupported. The original
Boolean-conditional rejection is promoted; the 57 gap references are unchanged.

Corpus 22 admits empty statements with 12 paired programs and eight boundary
rejections. The former `statement-empty` rejection is promoted. The gap frontier
remains 57 references.

Corpus 23 admits multiple initialized numeric `let`/`const` declarators, including
`for` headers. Eighteen paired programs cover left-to-right initialization,
earlier-binding dependencies, lexical shadowing, effects and source budgets.
Each initializer runs before its binding becomes available; later initializers
may use earlier bindings. Twenty-one boundary rejections preserve TDZ, numeric
admission and cumulative limits. The multiple-declarator gap is promoted, leaving
56 reference cases.

Corpus 24 admits unresolved `Infinity` and `NaN` reads as numeric constants.
Nineteen paired programs cover values, effects, shadowing, scope restoration,
TypeScript erasure and depth bounds; 32 boundary rejections preserve TDZ (including
later declarators), write, exact-name and property-access restrictions. The two former global-value gap
references are promoted, leaving 54 lowering-gap references.

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
- The body admits initialized numeric `const`/`let` declarations and
  discarded numeric expression statements in source order, lexical blocks and
  statement `if`/`else`, and `while`/`for`/`do...while` loops. Empty statements
  fall through without generating IR operations, values or source cells. They
  count once toward the statement budget and use ordinary source nesting. An
  empty body does not establish return coverage or prove termination.
  Early numeric returns are supported. Every structural path must return a number:
  a sequence must reach a guaranteed return; a block inherits that result; an `if`
  guarantees return only when both present arms do. Missing-return paths and
  statements following a guaranteed return are rejected, even if valid JavaScript.
  Each declaration has one or more initialized identifiers, evaluated left to right;
  each binding becomes available only after its own initializer. References resolve
  through semantic symbol identities.
  Unresolved references named exactly `Infinity` and `NaN` read their standard
  primitive numeric values. Resolved local and parameter symbols retain normal
  binding, mutation and TDZ rules, including when they shadow those names.
  These reads generate constants without host access; other global references and
  property access such as `globalThis.Infinity` or `Number.NaN` remain rejected.
  Numeric expressions admit literals, these two intrinsic reads, initialized bindings, parentheses, binary
  `+ - * /`, unary `+ -`, and ternaries. Conditions admit Boolean literals, logical negation, numeric
  `< <= > >= == != === !==` comparisons and primitive-number truthiness. Truthiness
  evaluates its expression once; ±0 and NaN are false, all other numbers are true.
  Both equality operands must lower to numbers. On this domain, `==`/`!=` use
  the same comparisons as `===`/`!==`: NaN is unequal to every number and signed
  zeros compare equal. Ordered snapshots preserve operand effects. No coercion
  is introduced, and equality results remain predicates rather than numeric
  locals, arithmetic operands or direct ABI results.
  Numeric comma operands execute left to right and yield the final value snapshot;
  in predicate positions the final operand may instead be any admitted predicate. Non-final
  comparisons remain outside admission. Predicate-valued expression statements,
  including comparisons and logical negation, remain outside admission.
  Numeric `&&` and `||` evaluate the left operand once and return its original
  snapshot or the selected RHS result. The RHS is fully admitted but executes only
  in the selected branch. In predicate positions `&&` and `||` may also combine
  comparisons, Boolean literals, negation and numeric truthiness. Their internal
  result is a canonical predicate, with the same evaluation order and selected
  effects. Boolean/comparison operands remain rejected in numeric value positions;
  `??` remains rejected in both contexts.
  Both ternary arms must be admitted. In Number-value positions they yield numbers
  through `if (result f64)`; in predicate positions they yield predicates through
  `if (result i32)`, converting only the selected arm to truthiness at runtime.
  Optional TS annotations must be exactly `number`.
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
  Wasm temporary preserve its snapshot across the store. Numeric expression
  statements evaluate their full expression and discard its Number result. This
  includes writes, arithmetic, comma expressions, numeric short-circuit and ternary
  expressions, parentheses and runtime-transparent TypeScript wrappers. Writes to `const`, globals,
  self/forward locals, property/destructuring and TS-wrapped targets are rejected.
  Scope exit removes bindings from visibility without recycling their physical
  Wasm slots. Both branches are admitted independently before encoding. Explicit
  Wasm returns preserve completion; a final `unreachable` is a validator aid only
  after the structural return check succeeds. Each `while` is treated as possibly
  falling through, even when its body returns. A following numeric return is required
  on that path. Loop-body locals are initialized on each entry and disappear from
  visibility on exit; physical slots are not reused for sibling declarations.
  Numeric `for` initializers may be omitted, an initialized `let`/`const` binding
  list, or an admitted numeric expression. Tests use the admitted predicate forms, or may be omitted for an
  unconditional test; updates are numeric expressions or omitted. Initialization
  runs once, then each iteration tests, executes the body, and executes the update.
  A return skips the update, but the compiler still admits the whole source body
  and update independently. Header bindings are visible only inside the loop;
  body locals are invisible to the update. `for` remains possibly falling through
  for structural return checking, including with an omitted test. The supported
  numeric subset cannot observe per-iteration environment identity because captures
  and `eval` remain unsupported.
  Numeric `do...while` executes its body before every test, including the first.
  Body locals are invisible to the condition and initialize on each body entry.
  A return skips the test at runtime; the entire test must still satisfy admission.
  A `do` guarantees a numeric return exactly when its body does, so a guaranteed
  returning body needs no fallback and any following statement is rejected.
- Everything else is rejected: extra statements/exports/imports, directives,
  hashbangs, defaults/rest/destructuring, async/generator/generic/declare functions,
  `this` parameters, modifiers, other annotations, calls, strings/objects/BigInts,
  type-only declarations, uninitialized locals and compound operators beyond the four above,
  `for-in`/`for-of`, `break`/`continue`, labels and nullish coalescing. Boolean arithmetic/returns and self/forward local references are
  outside this admission, even in unselected branches. These rejections describe
  the experiment, not the validity of arbitrary JavaScript.
  TypeScript-wrapped assignment/update targets are still rejected.
- Admission covers the entire AST, including directives/modifiers stored outside
  ordinary statements. Oxc parser and semantic syntax diagnostics are checked.
  The compiler does not run a TypeScript type checker or infer arbitrary JS types.
- AST grouping is preserved; there is no optimizer, reassociation, constant folding
  or fused multiply-add lowering. Literal values use Oxc's binary64 parse followed
  by round-trippable WAT decimal spelling (explicit `inf` for overflow literals).
- Source limit 16 KiB; expression nesting limit 128; at most 128 numeric locals
  across all lexical scopes and 256 recursively counted statements, including
  block/`if`/`while`/`for`/`do` nodes, both arms, loop bodies and returns; a `for` header
  declaration counts once as a statement, with one local per declarator. Header
  expressions use the ordinary expression nesting budget. Statement nesting limit 64;
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
PASS 1013232 numeric comparisons; 411 rejected inputs; overwrite preserved
DEMO: affine.ts -> Wasm; run(4, 1) = 3
PASS all checks (Node v20.10.0)
```

404 programs, each written as JS and TS, produce 808 modules.
Each module is validated and run in Wasmtime and Node-Wasm over 627 argument pairs: the Cartesian
product of 19 named boundary values plus 256 reproducible raw-bit pairs from seed
`4752554e`, plus ten named Mandelbrot orbit pairs. This yields 1,013,232 engine-to-JS
comparisons. The explicit demo calls in both engines are additional assertions. All JS/TS module pairs emit identical bytes.

The [TypeScript erasure fixtures](../tests/compatibility/typescript-erasure.mjs)
use hand-paired JavaScript references. They check all four wrappers around numbers
and comparisons, nested forms, ordered writes, snapshots, loop-condition effects,
NaN/±0 behavior and the 128-node nesting boundary. Calls, strings and objects remain
rejected inside wrappers, including unselected arms. Const writes, uninitialized
bindings, wrapped targets, and depth 129 also remain rejected.
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

The [numeric do fixtures](../tests/compatibility/numeric-do.mjs) check at-least-once
execution with zero, signed zero and NaN tests, repeated condition effects, per-entry
initialization, nested scopes and early returns. Guaranteed returning bodies need
no fallback, while unsupported conditions are rejected even after such returns.
A finite `false` condition composes with the admitted Boolean-literal predicates.
Exact statement/local/depth limits remain source-based. A finite mutation treating
the loop as a pretest loop changes a result from 5 to 2 and is detected. The order follows
[DoWhileLoopEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-runtime-semantics-dowhileloopevaluation).

The [numeric equality fixtures](../tests/compatibility/numeric-equality.mjs) check
both operators over all boundary and raw-bit vectors, ordered operand mutations,
TS wrappers, comma and negated predicates, and finite loop tests. This Number-only mapping
follows the same-type branch of
[IsLooselyEqual](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islooselyequal)
and [Number::equal](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-equal).
The retained coercive reference still rejects its string operand; only its
expected diagnostic changes from an unsupported operator to an unsupported expression.
A focused mapping-swap mutation changes an equality branch result from -0 to +0
and is detected; the original mapping is restored before the full replay.

The [numeric statement fixtures](../tests/compatibility/numeric-statements.mjs)
check discarded literals, reads, arithmetic, ordered mutation, selected branches,
blocks, finite loop bodies, all four whole TypeScript wrappers and source budgets.
An asserted type does not convert a runtime value or justify numeric admission.
The admitted behavior follows expression evaluation and value retrieval in
[ExpressionStatement evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-expression-statement-runtime-semantics-evaluation);
completion values cannot cross this experiment's explicit numeric return ABI.
A finite mutation skipping statement lowering changes a returned result from 3
to 2 and is detected; the full replay runs after exact restoration.

The [empty statement fixtures](../tests/compatibility/empty-statements.mjs) check
standalone/nested empties, branches and finite loop bodies with input-independent
counters. Exactly 255 empty statements followed by a return fit the 256-statement
budget; one more empty is rejected. A mutation skipping empty-statement charges
wrongly admits the latter program and is detected. Sixty-four nested `if(false)`
statements with an empty body fit the nesting bound; 65 do not. Missing returns,
statements after a guaranteed return, a module-level prefix and bare `return;`
remain rejected. This follows
[EmptyStatement evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-empty-statement-runtime-semantics-evaluation).

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

The [binding-list fixtures](../tests/compatibility/multiple-declarators.mjs) compare
left-to-right initializer effects, earlier-binding reads and lexical scope.
A finite mutation that reversed declarator order changed an independent-effects
result from 14 to 13 and was detected; restoring source order restored the checks.
Each initializer is checked before publishing its own binding, preserving
self/forward-reference rejection even when a later binding shadows an outer one.
The admission follows [BindingEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-runtime-semantics-bindingevaluation).

411 malformed or unsupported programs fail explicitly, each with a unique
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
