# Numeric compiler and runtime

The separate [experimental async profile](async.md) now preserves undefined/null/
Boolean/Number through await continuations with saved state and host Promise jobs. It does not change this
numeric profile's synchronous return contract or its rejection inventory.

**Maintained implementation at development version 0.0.1:** Oxc parses JS/TS, a custom lowering
pass builds and verifies [typed numeric regions](ir.md), then emits WAT. `wat`
encodes it into real Wasm bytes, and Wasmtime executes them.
Node supplies an independent original-JS reference and a second Wasm engine.

Private IR cells have a fixed `Number`, `Predicate` or `General` type. Source lowering
keeps Number and immutable Boolean const cells scalar where admitted;
General const/let cells hold the represented primitive kinds.
In the default numeric profile, all entry/helper parameters and returns remain
Number-only. This adds no dynamic
type inference or boxing; see the [bounded source contract](#immutable-boolean-predicate-locals).

## Expression evaluation control

Number, Predicate, discarded evaluation and arithmetic-operand conversion use one
[explicit pending-work evaluator](../crates/grunkalunka-compiler/src/lower/evaluate.rs).
Requests borrow original AST nodes; a cursor owns unfinished child regions, and
continuations own completed children until their parent operation is emitted.
Evaluation modes, original depth charges, first diagnostics, snapshots and region
allocation order retain their existing source contract. This changes control flow,
not source admission, IR types or the public compile API.

The [decision and alternatives](adr/0001-explicit-evaluator-control.md) distinguish
this maintained implementation from the disposable feasibility study. Evaluator
requests do not recursively enter the evaluator. Parsing/semantic analysis,
statements, shape classification, IR verification/emission, debug rendering and
destruction retain separate recursion concerns; this is no whole-frontend liveness,
heap bound, portability or performance claim.

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
directory for inspection while diagnosing a run. Replay directories are disposable:
capture useful findings in tests or concise notes, then remove obsolete runs.
No servers, ports or background services are used.

### Compatibility corpus and report

[corpus.mjs](../tests/compatibility/corpus.mjs) versions the selected programs, expected rejections and
unimplemented surfaces independently of the replay harness. Corpus version 65
contains 1781 paired programs, 919 rejection fixtures and five unexecuted feature
groups linked to milestone IDs. These are selected evidence and gaps, not an
inventory of the entire language or a compatibility percentage.

The source-bound [rejection inventory](../tests/compatibility/rejection-inventory.json)
classifies all 919 rejections in 56 groups: 31 invalid sources in their selected
strict Module dialect, 144 explicit resource restrictions and 744 open requirements.
Each record pins its fixture ID, JS/TS extension, `module-strict` source mode and
original UTF-8 source hash. Group evidence names the applicable ECMAScript rule,
official TypeScript syntax reference or explicit source-budget contract. A compiler
diagnostic alone does not establish source invalidity. TypeScript assessments do
not add a type checker or treat contradictory annotations as syntax errors.

The harness validates complete, exact inventory coverage before executing fixtures.
Its additive `rejectionInventory` report field carries the group assessments,
descriptive feature dependencies, harness needs and observable completion criteria.
Each rejection result records its resolved `classification` and a separate
`lowering: { executed: false, outcome: 'not-executed' }`; a passing rejection check
is not compiled language support. Final report validation rejects missing or changed
classification. Only the remaining 39 explicit curated oracle opt-ins execute in Node.
Other rejection sources may throw or diverge and are never executed as JS.

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

Corpus 25 admits unlabelled `break` in while/for/do bodies. Twenty paired
programs cover nearest-loop targeting, nested blocks/branches, skipped for steps
and do tests, mixed break/return paths, body initialization and source budgets.
Eighteen boundary rejections preserve whole-AST admission, guaranteed-abrupt tail
rejection and numeric return coverage. Four old break rejections are promoted.
A finite mutation retaining an outer break target instead of the nearest target
emits valid Wasm but changes the nested-loop result from 33 to 1; the regression
detects it. That run left 53 reference gaps. Continue and labelled control remained
unsupported.

Corpus 26 admits unlabelled `continue` in while/for/do bodies. Twenty-seven paired
programs cover nearest-loop targeting, outer-target restoration, for updates, do
tests, mixed abrupt paths, lexical initialization and exact source budgets.
Twenty-six boundary rejections preserve numeric headers, whole-source admission,
return coverage and the remaining control exclusions. Four finite continue
rejections are promoted; the old potentially infinite while-continue rejection is
retired without executing its source. That run left 52 reference gaps.

Corpus 27 admits Oxc-recognized module/function directive prologues. Nineteen
paired full-source programs retain their exact JS/TS module text, covering strict,
unknown, escaped, repeated, ASI and comment-separated directives plus shared source
budgets. Twenty-two boundary rejections preserve strict syntax diagnostics,
non-prologue string rejection, the entry contract and hashbang refusal. The two
reference gaps and two ordinary directive rejections are promoted, leaving 50
reference gaps. Skipping directive charges wrongly admits 257 statements and is
detected; ignoring full-source overrides fails the source-construction tests.

Corpus 28 admits numeric unary `~` with 21 paired programs and 20 boundary
rejections. The former `gap-bit-not` reference is promoted, leaving 49 reference
gaps. Operands truncate toward zero and wrap modulo `2^32`; NaN, infinities and
signed zero convert to zero before complement, producing -1. The result remains
a Number. That unit left Boolean, string and BigInt operands, binary bitwise operators
and shifts outside admission. A full-source directive/postfix-update case checks
that directives erase while the complement operand still runs once.

Corpus 29 admits only hashbang comments recognized by Oxc at the start of the
unchanged source. Eighteen paired full-source programs cover empty and Unicode
comments, LF/CR/CRLF/LS/PS endings, directives, effects and source budgets. The
former `gap-hashbang`, ordinary hashbang and directive/hashbang rejections are
promoted, leaving 48 reference gaps. Twenty-two new boundary rejections preserve
misplaced/BOM-prefixed hashbang syntax errors, the entry contract, unsupported
bodies and original-source limits. Restoring the refusal, stripping a leading BOM
or charging a hashbang as a statement is detected by finite mutation probes.

Corpus 30 admits Number-only binary `&`, `|` and `^` with 30 paired programs and
30 boundary rejections. Three reference gaps and the ordinary bitwise rejection
are promoted, leaving 45 reference gaps. Both operands execute eagerly, left to
right, with the left value captured before RHS effects. Each operand truncates and
wraps to 32 bits; the signed result converts back to a Number, with positive zero.
That unit left shifts, bitwise compound assignments and mixed-type coercion outside admission.
One full-source case combines a hashbang and module/function directives with a
RHS write, preserving the captured left value and the exact original JS oracle.

Corpus 31 admits Number-only `<<`, `>>` and `>>>` with 34 paired programs and 31
boundary rejections. Three reference gaps and the bitwise/shift composition
rejection are promoted, leaving 42 reference gaps. Counts use their low five bits
after truncation and wrapping. `<<` and `>>` return signed numeric results; `>>>`
zero-fills and returns an unsigned Number. That unit left bitwise/shift compound
assignments and mixed-type coercion outside admission.

Corpus 32 admits TypeScript parameter, return and local annotations as unvalidated
metadata. Twenty-three paired programs cover unknown and contradictory types,
compound types, unresolved names, type queries, source effects and budgets.
Four ordinary annotation rejections are promoted; the 42 reference gaps are
unchanged. Twenty-four new rejections preserve runtime numeric admission,
binding/parameter restrictions, parser/semantic diagnostics and source limits.
Restoring a number-only local annotation guard is detected by the artifact test.
The original annotated AST remains available through compilation; no early
stripping, checker, persistent metadata API or optimization is introduced. A full-source
composition case combines unknown/compound annotations with an unsigned shift
whose RHS writes its left binding, retaining the captured left value.

Corpus 33 admits Number-only `&=`, `|=`, `^=`, `<<=`, `>>=` and `>>>=` by mapping
them to the existing Binary operations. Thirty-nine paired programs and 29 boundary
rejections cover target snapshots, stored/yielded results, RHS effects, unsigned
results, wrappers, finite loops and source budgets. Ten ordinary rejections are
promoted; the curated frontier remains 42 gaps. Remainder, power, logical and
nullish assignments and unsupported targets remain rejected.

Corpus 34 admits explicit TypeScript aliases and interfaces as statements,
including direct named exports and default interface exports. Thirty-six paired
programs and 40 new rejections cover module/function/block positions, declared
and generic forms, type/value names, abrupt tails and source budgets. The former
`gap-type-alias` and two annotation/declaration rejections are promoted, leaving
41 reference gaps. Type declarations consume one statement each, including export
wrappers, but no runtime locals, expressions or IR. Omitting their charge and
admitting arbitrary runtime statements past an abrupt completion are detected by
finite mutations. The original AST and semantic analysis remain intact.
A full-source composition case combines a module alias, annotated numeric locals
and a post-return alias with an unsigned shift assignment whose RHS writes its
target, preserving the captured left value and stored result.

Corpus 35 adds direct calls to plain top-level numeric helpers, including forward,
self and mutual calls. Thirty-nine paired full-source programs and 51 new boundary
rejections cover signatures, argument snapshots, separate invocation state, TS
metadata, finite recursion and global source budgets. No prior rejection is
promoted; all 41 reference gaps retain their original programs and diagnostics.

Corpus 36 admits Number `%` and `%=`. Sixteen rejections retain their original
IDs and source bytes as admitted pairs, including all eight selected TypeScript
remainder expressions; 24 additional pairs check ordered operand/target snapshots,
selected effects, finite loops, wrappers and direct/recursive helper composition.
The now-empty inventory remainder group is removed. The 666 remaining negatives
retain source/mode/outcome; the two admitted-operator diagnostic lists add exactly
`%` and `%=` and their copied expectations change accordingly. Other diagnostic
text, spans and frontend stages remain unchanged.

Number remainder follows [Number::remainder](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-remainder):
NaN/infinite dividends and zero divisors yield NaN; an infinite divisor returns the
finite dividend. Zero results keep the dividend sign. The quotient is truncated in
real arithmetic, so rounded `a - trunc(a/b)*b` is not an implementation of this rule.
An original private WAT helper reduces integer significands exactly. It is emitted
only when any emitted source function/region uses remainder, after all source
functions; source call indices and private local namespaces remain independent.
It introduces no imports, memory, source bindings or source-budget charges.
Programs without remainder retain their Wasm bytes. No new coercion or BigInt
admission follows. Repeated operations still share the existing 10,000-fuel budget.

Corpus 37 adds 29 Math.abs pairs and 35 new rejection boundaries. Only the original
`call` and `gap-math-call` programs are promoted, with their original JS source,
mode and oracle retained. The curated frontier is now 39 references. The durable
[compiled kernel test](../tests/compatibility/numeric-math-abs-kernel.test.mjs)
checks 4,109 binary64 inputs (4,096 reproducible raw patterns plus named boundaries)
against pristine Node Math.abs in both Wasm engines, using batches within the existing
CLI input limit. Negating instead of taking the magnitude fails on positive 3;
ignoring semantic binding identity wrongly admits a local Math shadow. Restored
code passes before the canonical check. All 1,500 prior JS/TS source/Wasm variants,
750 oracle sources, 664 remaining rejection diagnostics and 39 gap oracles remain
unchanged. Affine stays 93 bytes and Mandelbrot 504; fuel remains 10,000, with no
speed claim.

Corpus 38 promotes exactly 14 original TypeScript wrapped-target rejections,
preserving their TS bytes and IDs with hand-paired unannotated full JS oracles.
Another 37 pairs cover all supported compound operators, prefix/postfix snapshots,
misleading type assertions, numeric shadow bindings, helpers, finite loops and depth.
Target resolution peels one retained wrapper or inner parenthesized node at a time
and passes the original identifier to the existing symbol/const/initialization checks.
Depth starts at the containing assignment/update; every retained target wrapper or
inner parenthesis adds one, with 128 admitted and 129 rejected. Oxc discards outer
target parentheses before lowering, so those discarded nodes cannot be counted.
The original AST and semantics stay intact. No type checker, coercion, new IR or
additional write target kind is introduced. The empty wrapped-target group is removed;
all 685 other rejection observations and 39 curated references remain unchanged.

Corpus 39 adds the fixed-opcode sqrt/floor/ceil/trunc family with 49 paired
programs and 70 new boundary rejections. Only the original
`math-abs-other-method` floor source is promoted; the 39 curated references remain.
Each production-compiled kernel compares 4,147 inputs (51 named boundary/square
neighbours plus 4,096 seeded raw patterns) with original JS in both Wasm engines:
33,176 comparisons across the family. Wrong-opcode probes for all four, a signed-zero
loss probe and a semantic-shadow bypass probe fail; restored code passes. All 1,660
prior source/Wasm variants and 830 oracles remain unchanged; 684 other rejection
sources/modes/diagnostics are retained exactly, including every Math.abs diagnostic.
No import, helper, ABI, fuel or source-budget change follows from these intrinsics.

Corpus 40 promotes exactly four Boolean-complement rejections and adds 36 pairs.
Numeric `~` accepts a Boolean literal, logical negation or an admitted numeric
comparison/equality through retained parentheses and TypeScript wrappers. A pure
bounded shape scan runs before emission; assertions never establish a Boolean type.
The original operand lowers exactly once through the existing predicate path at its
original argument depth. An explicit `NumberFromPredicate` bridge converts canonical
false/true predicates to Number zero/one before the unchanged Number complement.
Ordinary Number operands retain their existing path, including `(a as boolean)`.
This follows [bitwise NOT evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-bitwise-not-operator)
and [ToNumber](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber).
It does not admit general Boolean values, storage, arithmetic, calls or root logical
and conditional Boolean operands. Logical `!` retains its existing predicate domain.
All 750 other rejections and 39 curated reference observations stay unchanged.

Corpus 41 admits discarded Number/Boolean evaluation with 45 paired programs,
including 13 exact ordinary promotions, and 30 new boundary rejections. Effects,
short-circuit selection and original nesting depth survive discarded results.
All 737 retained rejection diagnostics remain exact; all 39 curated gap references
remain. This introduces no Boolean storage, call arguments, ABI results or coercion.

Corpus 42 adds 41 paired primitive-Boolean unary `+`/`-` cases and 18 explicit
boundary rejections, with no old rejection promoted. The already-whitelisted
operators reuse the unchanged Boolean-shape classifier and predicate-to-Number
bridge used by `~`; `+false` is +0 and `-false` is -0. Original operands execute
once at their original depth. Assertions never establish a Boolean runtime kind;
raw Boolean identifiers and root Boolean logical/conditional/sequence operands
remain outside this bridge. Logical `!` alone remains outside the numeric-return
contract. The post-local baseline contains 797 earlier rejection observations and
38 curated references; unary plus/minus promotes none.
The rules follow [unary plus](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-plus-operator),
[unary minus](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-unary-minus-operator)
and [ToNumber](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber).

The ordinary admitted tuple contains JS/TS function bodies and optionally an
independent JS oracle body. An optional sixth metadata field instead supplies
complete `sourceJS`/`sourceTS` text; its default oracle is the exact full JS source.
An explicit full `oracleSource` cannot be combined with a legacy oracle body.
Metadata is validated for the whole inventory before executing fixtures. No full
source is wrapped, trimmed or normalized. The affine files and Mandelbrot's
independent body oracle retain their existing provenance. Retained inputs and
oracles are read back and checked against their recorded source hashes.

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

- One runtime module-level `export function run` with two ordinary parameters,
  alongside plain unexported numeric helper declarations and the explicit type declarations described below.
  Inputs are **already primitive JS numbers**, represented as Wasm `f64` values.
  This ABI contract is independent of whether TS annotations are present.
- Helper signatures are collected before any body is lowered, so calls can precede
  declarations. A bare callee must resolve by semantic SymbolId to a collected
  function; local/parameter/TDZ shadows never fall back by spelling. All arguments
  must be Numbers and match the declared count exactly, including zero. Arguments
  execute once from left to right and retain snapshots across later argument
  writes. Each invocation has independent mutable parameters and locals. Every
  body, including unused helpers, must satisfy numeric return coverage. Function
  values, aliases, nested declarations/captures and writes to function bindings
  are rejected, including writes in dead or unused code. Missing/extra arguments,
  spread, defaults, rest, optional and generic calls remain outside this subset.
  Self/mutual calls share the existing Wasmtime store's fuel budget; they do not
  renew it. The existing engine stack guard also remains in place. Neither is a
  static termination proof or a guarantee for arbitrary host-thread stacks.
- The self-contained numeric execution profile provides pristine specified
  Math.abs, Math.sqrt, Math.floor, Math.ceil and Math.trunc intrinsics, rather than
  references to an external JavaScript realm's mutable Math object. Each requires
  a nonoptional static call, exactly one Number argument and an actual unresolved
  semantic value reference for Math. Resolved parameter/local/TDZ/module-helper
  shadows never fall back by spelling. Ordinary numeric bindings named Math may
  still mutate; type-only Math names do not create value bindings. Arguments run
  eagerly once and produce captured Number snapshots. Abs emits `f64.abs`;
  sqrt/floor/ceil/trunc emit their corresponding `f64` opcode directly. There is
  no ToNumber coercion, integer conversion or generated arithmetic helper.
  Non-NaNs compare exact binary64 bits; NaNs compare by class, not payload/sign.
  Abs maps negative zero to positive zero; the other four preserve input negative
  zero. Ceil/trunc may also return negative zero for negative fractions. Negative
  nonzero sqrt inputs, including negative infinity, return NaN. Computed, optional,
  generic, wrapped-callee, spread, aliased and first-class forms remain rejected.
  Global Math writes, property patches, reification and external realm integration
  remain open work; Math and its methods are not immutable language globals.
  See [Math.abs](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.abs),
  [Math.sqrt](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.sqrt),
  [Math.floor](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.floor),
  [Math.ceil](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.ceil),
  [Math.trunc](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.trunc)
  and [Wasm floating numerics](https://webassembly.github.io/spec/core/exec/numerics.html#op-fsqrt).
- The body admits initialized numeric `const`/`let` declarations and
  discarded Number/Boolean expression statements in source order, lexical blocks and
  statement `if`/`else`, and `while`/`for`/`do...while` loops. Empty statements
  fall through without generating IR operations, values or source cells. They
  count once toward the statement budget and use ordinary source nesting. An
  empty body does not establish return coverage or prove termination.
  Early numeric returns are supported. Every structural path must return a number:
  a sequence must reach a guaranteed return; a block inherits that result; an `if`
  guarantees return only when both present arms do. Missing-return paths and
  runtime statements following a guaranteed return, break or continue (including
  mixed abrupt branches) are rejected, even if valid JavaScript. Recognized type
  declarations may follow them while retaining the same completion alternatives.
  Each numeric variable declaration has one or more initialized identifiers, evaluated left to right;
  each binding becomes available only after its own initializer. References resolve
  through semantic symbol identities.
  Unresolved references named exactly `Infinity` and `NaN` read their standard
  primitive numeric values. Resolved local and parameter symbols retain normal
  binding, mutation and TDZ rules, including when they shadow those names.
  These reads generate constants without host access; other global references and
  property access such as `globalThis.Infinity` or `Number.NaN` remain rejected.
  Numeric expressions admit literals, these two intrinsic reads, initialized bindings, parentheses, binary
  `+ - * / % & | ^ << >> >>>`, unary `+ - ~`, and ternaries. Numeric `~`, `&`, `|` and `^` use signed 32-bit wrapping
  semantics and yield Numbers for Number operands.
  Binary bitwise operands both run eagerly, left to right; RHS writes cannot
  replace the captured left value. Results use signed conversion, including positive zero.
  Shifts also capture both Number operands eagerly in source order. The count is
  truncated and wrapped, then masked to five bits. `<<` shifts left and `>>`
  sign-extends; both return signed results. `>>>` zero-fills and alone uses unsigned
  result conversion, so `-1 >>> 0` returns `4294967295`. Shift results are Numbers
  and zero results are positive zero.
  Unary `+`, `-` and `~` additionally admit Boolean literals, logical negation and
  numeric comparison/equality through retained TS/parenthesis wrappers. They lower
  the original operand once to a predicate and explicitly convert false/true to
  Number +0/1. Unary plus copies that Number; unary minus uses f64 negation, so
  `-false` retains -0. Number operands keep their existing path. The shape classifier
  is unchanged: assertions do not prove Boolean kinds, root Boolean logical,
  conditional and comma shapes do not qualify, and `!` itself is not a numeric return.
  Conditions admit Boolean literals, logical negation, numeric
  `< <= > >= == != === !==` comparisons and represented primitive truthiness. Truthiness
  evaluates its expression once; null, undefined, false, ±0 and NaN are false.
  True and other represented Numbers are true.
  Number equality operands keep their original lowering. On that domain, `==`/`!=` use
  the same comparisons as `===`/`!==`: NaN is unequal to every number and signed
  zeros compare equal. Ordered snapshots preserve operand effects. No coercion
  is introduced by equality itself. Strict `===`/`!==` also accept two proven Boolean
  operands through the bounded same-kind bridge below. The four represented kinds
  additionally admit mixed-kind strict equality through General operands; loose Boolean
  equality and Boolean relational operands remain outside admission. Results remain
  predicates, and may enter the
  explicit primitive Number bridges; direct ABI results remain Number-only.
  Comma operands execute left to right; non-final Number or Boolean results are
  evaluated and discarded. The final operand retains its surrounding Number or
  predicate contract. Expression statements and expression-form for initializers
  and updates also discard Number/Boolean results. A pure bounded syntax inspection
  selects predicate lowering only for Boolean-producing result shapes, including
  mixed logical/conditional results; Number-only trees retain their original path.
  Only the selected branch executes. Type assertions never prove runtime kinds;
  mutable storage, call arguments, assignment RHS values and returns remain Number-only.
  Binary arithmetic has the bounded inline and immutable-Predicate operand conversions described below.
  Immutable Boolean const locals have the predicate-position boundary described below.
  Numeric `&&` and `||` evaluate the left operand once and return its original
  snapshot or the selected RHS result. The RHS is fully admitted but executes only
  in the selected branch. In predicate positions `&&` and `||` may also combine
  comparisons, Boolean literals, negation and represented General truthiness. Their internal
  result is a canonical predicate, with the same evaluation order and selected
  effects. Raw Boolean/comparison operands remain rejected outside the admitted unary
  conversion in numeric value positions;
  Binary `??` uses the separate nullish-selection contract below.
  Both ternary arms must be admitted. In Number-value positions they yield numbers
  through `if (result f64)`; in predicate positions they yield predicates through
  `if (result i32)`, converting only the selected arm to truthiness at runtime.
  TS parameter, return and local type annotations are unvalidated metadata.
  Unknown names, type queries, compound types and declarations contradicting the
  numeric body are accepted without assignability checking. They do not convert
  runtime values or prove numeric admission. The original Oxc AST and arena retain
  this syntax through compilation; the resulting artifact contains only WAT/Wasm,
  without persistent declared-type metadata. The numeric IR types come from the
  ABI and validated runtime operations, independently of annotation spelling.
  TypeScript `as`, angle-bracket assertions, postfix non-null `!`, and `satisfies`
  are erased in expression and predicate positions. Each wrapper counts toward
  nesting, and its runtime operand must independently satisfy admission. Wrapped
  comparisons stay predicates. No type checking or runtime conversion follows
  from a wrapper's asserted type. Generic runtime functions remain unsupported.
  Explicit type aliases/interfaces are admitted wherever Oxc parses them as
  statements, with either declare flag. Direct named exports wrapping exactly
  those declarations and default interface exports are also admitted. Each whole
  declaration, including its export wrapper, charges one source statement and
  retains ordinary statement depth checks. Type members emit no operations and
  consume no runtime locals or expression depth. Module admission selects one
  exported entry and any plain unexported helpers;
  declaration-level `import type`/`export type` items also erase and charge one
  statement each. Runtime imports, inline type specifiers, runtime export/re-export
  lists, ambient values, enums, namespaces and module/global augmentations remain
  unsupported. Type names do not create numeric bindings. See
  [type-only module items](#type-only-module-items).
  `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=` and `>>>=`
  expressions write to an initialized parameter or
  `let` binding and return the assigned number. Compound operations snapshot the
  target before evaluating the RHS once, apply the corresponding numeric Binary
  operation, then store and yield the computed result. `>>>=` retains unsigned
  Number results. Prefix/postfix `++` and `--` store the updated
  number and return the new/old snapshot respectively. The RHS runs once; a typed
  IR value and separate
  Wasm temporary preserve its snapshot across the store. Numeric expression
  statements evaluate their full expression and discard its Number result. This
  includes writes, arithmetic, comma expressions, numeric short-circuit and ternary
  expressions, parentheses and runtime-transparent TypeScript wrappers. Writes to `const`, globals,
  self/forward locals and property/destructuring targets are rejected.
  TypeScript `as`, angle assertions, non-null and `satisfies` wrappers may surround
  numeric assignment/update targets; their original identifier references retain
  semantic binding identity. Types do not coerce or validate the Number operand.
  Scope exit removes bindings from visibility without recycling their physical
  Wasm slots. Both branches are admitted independently before encoding. Explicit
  Wasm returns preserve completion; a final `unreachable` is a validator aid only
  after the structural return check succeeds. Each `while` is treated as possibly
  falling through, even when its body returns. A following numeric return is required
  on that path. Loop-body locals are initialized on each entry and disappear from
  visibility on exit; physical slots are not reused for sibling declarations.
  Numeric `for` initializers may be omitted, an initialized `let`/`const` binding
  list, or an admitted discarded Number/Boolean expression. Tests use the admitted predicate forms, or may be omitted for an
  unconditional test; updates are discarded Number/Boolean expressions or omitted. Initialization
  runs once, then each iteration tests, executes the body, and executes the update.
  A return or break skips the update, but the compiler still admits the whole source body
  and update independently. Header bindings are visible only inside the loop;
  body locals are invisible to the update. `for` remains possibly falling through
  for structural return checking, including with an omitted test. The supported
  numeric subset cannot observe per-iteration environment identity because captures
  and `eval` remain unsupported.
  Numeric `do...while` executes its body before every test, including the first.
  Body locals are invisible to the condition and initialize on each body entry.
  A return or break skips the test at runtime; the entire test must still satisfy admission.
  A `do` guarantees a numeric return exactly when its body does, so a guaranteed
  returning body needs no fallback and any following statement is rejected.
  Unlabelled `break` exits the nearest enclosing while/for/do loop, including
  through nested blocks and branches. It never supplies a numeric return; mixed
  break/return paths remain distinct until the loop consumes its own breaks.
  Unlabelled `continue` ends the nearest loop body, reaching the while test,
  the for update then test, or the do test. Nested loops restore both outer targets.
  Continue is distinct from normal completion and return until its loop consumes it;
  a do body with both return and continue paths still needs a following return.
  Explicit condition/yield regions and for updates must complete normally, and
  source for initializers remain numeric. Each source break or continue counts
  once as a statement, while generated exit/body-end blocks do not consume source
  budgets. The emitter's additional loop blocks may change
  artifact bytes and fuel cost; the 10,000-fuel allowance is unchanged.
- Oxc-recognized directive prologues at module/function entry are admitted.
  These initial string-literal expression statements are evaluated normally in
  ECMAScript; their completion values are unobserved by this explicit-number-return
  ABI, so no runtime values or IR are emitted. Module code and its contained
  function are already strict. Unknown directives have ordinary ECMAScript literal
  semantics; no framework or implementation-specific directive meaning is defined.
  Each module/function directive counts once in the shared statement budget,
  including before the numeric body, without consuming locals or nesting depth.
  Parser and semantic diagnostics remain authoritative. Parenthesized, wrapped,
  nested-block and later string statements remain outside numeric admission.
  The admission follows [Directive Prologues and strict module code](https://tc39.es/ecma262/multipage/ecmascript-language-source-code.html#sec-directive-prologues-and-the-use-strict-directive).
- An initial Oxc-recognized hashbang is an ordinary discarded comment. The
  original source is parsed once: no BOM stripping, trimming or hashbang removal
  occurs. A BOM, whitespace or another comment before `#!` remains a syntax error;
  a BOM after the hashbang's line terminator is ordinary whitespace. LF, CR, LS
  and PS terminate the comment, including CRLF. No terminator means the comment
  extends to EOF, so an export on that same line is not an entry declaration.
  Hashbangs emit no IR and consume no statements, locals or nesting depth, while
  every original UTF-8 byte still counts toward the 16 KiB limit. The entry and
  whole-body admission rules remain unchanged. See [Hashbang Comments](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-hashbang-comments).
- Everything else is rejected: extra runtime statements/exports/imports,
  defaults/rest/destructuring, async/generator/declare functions,
  `this` parameters, modifiers, indirect/property/optional calls except the specified numeric Math intrinsics, strings/objects/BigInts,
  other declaration forms, uninitialized locals, `**=`/`??=` assignments,
  `for-in`/`for-of`, `switch` and labels. Boolean arithmetic outside the explicit operand bridges, Boolean returns and self/forward local references are
  outside this admission, even in unselected branches. These rejections describe
  the experiment, not the validity of arbitrary JavaScript.
- Admission covers the entire AST, including directives/modifiers stored outside
  ordinary statements; directives and explicit type declarations still charge
  their source statements. Oxc parser and semantic syntax diagnostics are checked.
  The compiler does not run a TypeScript type checker or infer arbitrary JS types.
- AST grouping is preserved; there is no optimizer, reassociation, constant folding
  or fused multiply-add lowering. Literal values use Oxc's binary64 parse followed
  by round-trippable WAT decimal spelling (explicit `inf` for overflow literals).
- Source limit 16 KiB; expression nesting limit 128; at most 128 source locals
  across all functions and lexical scopes and 256 recursively counted statements, including
  module/function directives, explicit type declarations, block/`if`/`while`/`for`/`do`
  nodes, both arms, loop bodies and returns. Each added helper declaration charges
  once; the historical entry wrapper remains uncharged. Helper parameters are
  separate from the declared-local budget and limited to 1,000 per function by the
  backend resource bound. A `for` header
  declaration counts once as a statement, with one local per declarator. Header
  expressions use the ordinary expression nesting budget. Statement nesting limit 64;
  generated IR values, regions and Wasm temporaries do not consume source budgets;
  host input batches at most 4096
  pairs and 10,000 fuel per call. Each harness subprocess has a timeout; the whole
  compatibility replay uses the [inventory-derived watchdog](testing.md), with a
  three-minute floor, ten-minute cap and one-minute outer subprocess margin. These
  are engineering experiment bounds, not measured performance or platform policy.
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

The corpus-43 replay reports:

```text
PASS 2796420 numeric comparisons; 839 rejected inputs; overwrite preserved
DEMO: affine.ts -> Wasm; run(4, 1) = 3
PASS all checks (Node v20.10.0)
```

1115 programs, each written as JS and TS, produce 2230 modules.
Each module is validated and run in Wasmtime and Node-Wasm over 627 argument pairs: the Cartesian
product of 19 named boundary values plus 256 reproducible raw-bit pairs from seed
`4752554e`, plus ten named Mandelbrot orbit pairs. This yields 2,796,420 engine-to-JS
comparisons. The explicit demo calls in both engines are additional assertions. All JS/TS module pairs emit identical bytes.

The [TypeScript erasure fixtures](../tests/compatibility/typescript-erasure.mjs)
use hand-paired JavaScript references. They check all four wrappers around numbers
and comparisons, nested forms, ordered writes, snapshots, loop-condition effects,
NaN/±0 behavior and the 128-node nesting boundary. Unsupported calls, strings and objects remain
rejected inside wrappers, including unselected arms. Const writes, uninitialized
bindings and depth 129 also remain rejected. Wrapped writes now have separate
hand-paired [target fixtures](../tests/compatibility/numeric-ts-write-targets.mjs).
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

The [numeric complement fixtures](../tests/compatibility/numeric-bit-not.mjs)
check truncation, wrapping, nonfinite inputs, nested complements, update snapshots,
TypeScript wrappers, predicates, finite loops and source budgets. Complement follows
[Number::bitwiseNOT](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-bitwiseNOT)
and [ToInt32](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toint32).
The emitter reduces a captured value modulo `2^32`, converts without trapping to
`i64`, wraps to `i32`, complements, then converts the signed result to `f64`.
It reads the captured snapshot twice without repeating source evaluation. A
separate emitted-Wasm probe checks 4,265 inputs in both engines, including raw
bits and neighbors around `2^31`, `2^32`, `2^53` and `2^84`. Finite mutations
flooring the initial value or saturating directly to `i32` are detected and restored.

The [binary bitwise fixtures](../tests/compatibility/numeric-bitwise-binary.mjs)
check all three operations over the full vectors, operand snapshots, eager RHS
effects, nested complements, TypeScript wrappers, finite loops and source limits.
They follow [NumberBitwiseOp](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numberbitwiseop)
and [ordered binary evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-evaluatestringornumericbinaryexpression).
Both captured values use the unchanged complement conversion helper before
`i32.and`, `i32.or` or `i32.xor`, then signed conversion returns a Number. An
additional emitted-Wasm probe checks 5,110 input pairs in both engines, covering
raw bits, fractions, wrapping boundaries and nonfinite inputs on either side.
Finite mutations mapping AND to OR or evaluating the RHS before capturing the
left operand change expected results from 1 to 7 or 3 and are detected.

The [numeric shift fixtures](../tests/compatibility/numeric-shifts.mjs) check all
three shifts, count masking, signed and unsigned results, eager operand effects,
wrappers, finite loops and source budgets. They follow the Number
[shift operations](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-leftShift).
The unchanged helper supplies both captured bit patterns; Wasm's `i32.shl`,
`i32.shr_s` and `i32.shr_u` mask counts modulo 32. Only `>>>` uses
`f64.convert_i32_u`; the other integer operations retain signed conversion.
Both engines check an additional 9,504 edge/raw input pairs and three specified
fixed results. Finite mutations using zero-fill for `>>` or signed conversion for
`>>>` are detected and restored before replay.

The [bitwise/shift compound fixtures](../tests/compatibility/numeric-bitwise-compounds.mjs)
exercise all six mappings through the existing assignment path, following
[compound assignment evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation).
They separately observe the stored value, yielded snapshot and RHS count,
including writes to the target during RHS evaluation. A finite `&=`-to-OR mapping
mutation changes a stored/yielded result from 11 to 77 and is detected; the original
mapping is restored before the canonical replay. No conversion, IR or emitter
change is needed.

The [direct-call fixtures](../tests/compatibility/numeric-calls.mjs) use complete
JS/TS modules and exact full-source JS oracles. Recursion counters and loops are
finite independently of the 627 input pairs. The 1,000-parameter case is validated
and executed in both engines; 1,001 parameters reject before emission. A finite
argument-order mutation changes a stored result from 533 to 353 and is detected.
A separate Wasmtime-only finite-depth recursion test observed `OutOfFuel`, then
successfully reused the same instance. It does not claim that `StackOverflow` was
observed, and is never executed in a JS oracle. Adding calls leaves all 1,342 prior
JS/TS source and Wasm variants byte-identical, including affine at 93 bytes and
Mandelbrot at 504 bytes. The fuel allowance remains 10,000; no speed claim follows.

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

The continue emitter adds a generated body-end block per loop. The affine artifact
stays at 93 bytes; the retained Mandelbrot orbit now measures 504 bytes, up 14 bytes
from 490. The full corpus uses the same 10,000-fuel allowance. Finite mutation probes
detect skipping the for update (40 instead of 43) and skipping the do test (31
instead of 11). Neither probe requires an unbounded loop in JavaScript or Node-Wasm.

Adding numeric `~` leaves the non-complement affine and Mandelbrot artifacts
byte-identical at 93 and 504 bytes respectively. Complement expressions add an
inline conversion sequence; no execution-speed claim follows. The full corpus
retains the 10,000-fuel allowance.

Adding binary `&`, `|` and `^` also leaves all 1,018 previous JS/TS artifacts
byte-identical, including affine at 93 bytes and Mandelbrot at 504 bytes. The new
operators add two inline operand conversions and keep the same fuel allowance.

Adding Number shifts leaves all 1,078 previous JS/TS artifacts byte-identical,
including affine at 93 bytes and Mandelbrot at 504 bytes. No helper function,
import or new IR type is added, and the fuel allowance remains 10,000.

Adding bitwise/shift compound mappings leaves all 1,192 previous JS/TS artifacts
byte-identical. Affine remains 93 bytes, Mandelbrot 504 bytes and fuel 10,000.

The [binding-list fixtures](../tests/compatibility/multiple-declarators.mjs) compare
left-to-right initializer effects, earlier-binding reads and lexical scope.
A finite mutation that reversed declarator order changed an independent-effects
result from 14 to 13 and was detected; restoring source order restored the checks.
Each initializer is checked before publishing its own binding, preserving
self/forward-reference rejection even when a later binding shadows an outer one.
The admission follows [BindingEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-runtime-semantics-bindingevaluation).

839 malformed or unsupported programs fail explicitly, each with a unique
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

## Immutable Boolean predicate locals

`const` initializers may be Boolean literals, logical negation, admitted numeric
comparisons/equalities, or initialized immutable Boolean aliases, through retained
parentheses and TypeScript expression wrappers. Runtime initializer shape determines
the fixed `Predicate` slot type; type annotations/assertions do not coerce values.
The binding becomes visible only after its initializer succeeds. Symbol identity,
TDZ, lexical scope and once-only initializer effects remain authoritative.

Reads are admitted in existing predicate positions, including conditions, logical
negation and predicate logical/conditional operands. Unary `+`, `-` and `~` also
read initialized immutable Predicate bindings through the separate unary bridge
below. Binary arithmetic also uses that binding eligibility as described below. Mutable Boolean
locals and other Number positions (including returns and call arguments), root logical/conditional
initializer joins and discarded `flag && (a=b)` remain outside this bounded unit.
Bare/wrapped immutable Predicate reads in discarded contexts are covered below.
All source locals share the unchanged cumulative 128 limit across functions/scopes;
its diagnostic now says “source locals” for both Number and Predicate cells.

Corpus 42 adds 44 pairs, including four original promotions, and 34 boundary
rejections. Only `gap-boolean-value` leaves the curated frontier: its original
source, independent oracle, values/L4 metadata, empty prerequisites, inputs `[4,2]`
and positive-zero result are retained. The other three IDs are
`boolean-literal-local`, `not-local` and `equality-numeric-local`.
Two retained return diagnostics advance to an explicit Boolean-in-Number rejection;
exactly 13 existing local-budget diagnostics change “numeric” to “source”.
That predicate-local run recorded 1008 pairs, 2016 modules, 2,528,064 engine-to-JS comparisons,
797 rejections, 38 curated references and 56 inventory groups: 26 invalid sources,
99 resource restrictions and 672 open requirements. The Boolean-values group retains
90 records. Existing Number source/WAT/Wasm artifacts remain unchanged.

## Inline Boolean arithmetic operands

Binary `+`, `-`, `*`, `/` and `%` accept inline Boolean-producing operand shapes:
Boolean literals, logical negation and admitted numeric comparisons/equalities,
through retained parentheses and TypeScript wrappers. An arithmetic-only lowering
helper evaluates each recognized predicate once and uses `NumberFromPredicate`
to obtain false/true as +0/1. Left evaluation and its captured result finish before
right evaluation. Primitive conversion has no object hooks; ordinary Number
operands keep their original lowering and artifact bytes.

This leaves equality/comparison, bitwise/shift and compound operand dispatch
unchanged. Initialized immutable Predicate bindings now also enter the five arithmetic
operators through the bounded binding bridge below; other bare-binding binary positions remain rejected;
`!flag` may use existing predicate-position reads. Root Boolean logical/conditional
or comma-result operands are not inferred as Boolean shapes, while ordinary Number
sequence/conditional/logical expressions continue to compose. Strings, objects,
BigInts and general coercion remain outside this profile. Remainder still uses its
existing private helper, including in nested regions, and preserves signed zero.

Corpus 43 adds 66 pairs (six exact ordinary promotions) and 30 classified negatives.
Three former Rust-only rejection bodies are retained as positive assertions and
execution witnesses. No curated reference is transferred: all 38 remain. That run
recorded 1115 pairs, 2230 modules, 2,796,420 comparisons and 839 rejections in 56
groups (26 invalid, 109 resource restrictions, 704 open); 102 Boolean frontiers remain.

## Unary reads of immutable Predicate bindings

Unary `+`, `-` and `~` recognize initialized immutable Predicate const bindings,
including through retained TypeScript and parenthesis wrappers. Eligibility uses
the original reference's semantic SymbolId, published binding and fixed slot kind;
annotation spelling does not prove a runtime Boolean. The original operand is
lowered once as a predicate and the existing `NumberFromPredicate` conversion
feeds the unchanged Number unary operation. Wrappers retain original depth.

The free Boolean-shape classifier, initializer, discarded value and numeric-identifier
admission remain unchanged by that unary step. The later arithmetic bridge below
extends only five binary operand callers; the later discard selector admits only
leaf reads in discarded contexts. A bare `flag` still cannot be returned, passed,
assigned or used in bitwise/shift/compound positions; strict same-kind equality
is covered below. Mutable Boolean storage and root Boolean joins remain
outside admission. The resulting Number may compose through existing numeric paths.

Corpus 44 adds 43 pairs and 25 source-bound negatives. Exactly three ordinary IDs
move: `const-boolean-complement`, `const-boolean-plus` and
`boolean-numeric-unary-reject-identifier`; the latter two deliberately retain the
same original JS source under distinct IDs. No curated reference transfers.
Three former Rust guard spellings are positive assertions, with the additional
`const x=true; return ~x;` body retained as an execution witness. The existing
1115 admitted pairs and 836 other rejection observations remain the baseline.

## Arithmetic reads of immutable Predicate bindings

The existing bounded eligibility method is shared by unary operands and only the
five binary arithmetic operators `+`, `-`, `*`, `/` and `%`. Its body is unchanged:
original SymbolId, initialized immutable binding, fixed Predicate slot, and original
wrapper depth supply evidence. Original operands lower once to predicates and then
NumberFromPredicate; the left result is captured before the right is evaluated.
Number fallback, the binary dispatcher and the free Boolean classifier are unchanged.

That arithmetic step did not extend bitwise/shift/equality/compound operands, Boolean returns or
arguments, mutable storage or root Boolean joins. Discarded binding reads are
covered by the separate leaf-only selector below. The
native frontend liveness incident on the deferred bitwise candidate is not repaired
or retried here: the retained exact 554-byte Number depth rejection and all other
retained nesting rejections are checked before expanded focus; any native failure
stops this independent unit.

Corpus 45 adds 88 pairs and 27 classified negatives. Exactly four ordinary IDs move:
`const-boolean-arithmetic`, `boolean-arithmetic-bare-const`,
`unary-predicate-binding-reject-arithmetic` and `boolean-arithmetic-bare-const-right`.
The first three retain the same original JS bytes under distinct IDs. Two former
Rust guards transfer verbatim into positive assertions. No curated reference moves.
That arithmetic step recorded 1246 pairs, 884 rejections and 38 curated references; all 1158
prior pairs and 857 other rejection observations remain the baseline.

## Discarded immutable Predicate binding reads

Expression statements, comma prefixes and expression-form for initializers/updates
now admit bare/wrapped initialized immutable Predicate identifiers. The free
`discarded_boolean_shape` check runs first; only when false does the unchanged
`operand_boolean_shape` leaf check run. Both bodies and all result-join rules are
unchanged. The original operand lowers once through `predicate` to an unused Load;
there is no Number conversion, initializer replay or new storage/value position.
SymbolId, publication, immutable slot kind, original wrappers/depth and lexical
visibility remain authoritative. Ordinary Number and previously admitted inline
Boolean discard paths remain byte-identical.

This does not newly classify root logical/conditional/sequence results as Boolean.
For example `flag&&(a=b)` still rejects; new comma-prefix reads may naturally
compose inside otherwise admitted Number trees. Boolean returns/arguments, mutable
storage, assignment results and other excluded operator operands stay separate.

Corpus 46 adds 53 pairs and 26 classified negatives. Exactly five ordinary IDs
move: `const-boolean-statement`, `unary-predicate-binding-reject-discard`,
`arithmetic-predicate-binding-reject-discard`, `const-boolean-comma-prefix` and
`const-boolean-for-update`. The first three preserve the same original JS source
under distinct IDs. One former Rust guard transfers verbatim positive. No curated
reference moves; all 38 remain. That discarded step recorded 1299 pairs, 905 rejections and 56
groups (26 invalid, 127 resource, 752 open requirements).

## Strict same-kind Boolean equality

Predicate-position `===` and `!==` accept two operands recognized by the unchanged
bounded operand eligibility method: selected inline Boolean-producing syntax or
initialized immutable Predicate bindings, through original TS/parenthesis wrappers.
Both operands must qualify. Original SymbolId, publication and fixed slot kind
supply binding evidence; annotations never establish a runtime kind.

The left original operand lowers once as a predicate and converts through existing
`NumberFromPredicate` before the right does the same. Existing Number Compare and
`f64.eq/ne` compare the captured canonical zero/one values. The original two Number
descents remain the fallback. No classifier, binary dispatcher, IR, emitter, ABI,
runtime, source budget or native stack setting changes.

Mixed Boolean/Number operands, loose Boolean equality, Boolean relational operands,
mutable storage, root logical/conditional/comma operands and direct Boolean returns
remain outside this bridge. A valid predicate result may compose through existing
unary/arithmetic bridges and immutable initializers; no general kind policy follows.

Corpus 47 adds 47 pairs and 21 classified negatives. Exactly two ordinary sources
move unchanged: `boolean-equality-operands` and `not-equality-operand`, both JS strict
Modules. No curated reference or Rust-only negative body transfers. That step recorded
1346 pairs, 924 rejections and 38 curated references; all 1299 prior pairs and
903 other rejection observations remain the baseline.

## Numeric logical assignment

This records the original Number-only step; [General logical assignment](#general-logical-assignment)
below extends the represented value domain while retaining these scalar paths.

Initialized mutable Number parameters and `let` identifiers admit `&&=` and `||=`.
The original target is validated and loaded once before RHS effects. Number
truthiness selects the RHS for `&&=` on truthy values and `||=` on falsy values;
`0`, `-0` and NaN are falsy. The selected RHS must be an admitted Number expression.
It runs once, stores inside the selected child region and yields its captured
result. The skipped child performs no RHS evaluation or Store and yields the
original captured Number, preserving signed zero and NaN class. A later sibling
write cannot change the assignment result snapshot. The complete RHS must satisfy
source admission even when runtime selection skips it.

Original target SymbolId resolution, wrapper depth, initialization/publication and
const restrictions remain. Even valid JavaScript const writes that would short
circuit stay outside this admission profile; they are not classified as invalid
syntax. `??=`, `**=`, property/global/uninitialized targets, general values and bare
Boolean RHS results remain outside admission. Explicit existing numeric bridges
may supply Number RHS values. No classifier, IR/backend/runtime, ABI, dependency
or source budget changes.

Corpus 48 contributes 64 paired programs (including two exact ordinary promotions)
and 28 classified negatives. `unsupported-compound-8` and `unsupported-compound-9`
retain their original JS strict Module sources; no curated reference or Rust-only
guard transfers. The two retained `**=`/`??=` observations change only their honest
operator-whitelist text. The logical-assignment group remains open with an explicit
Number-only assessment. That step recorded 1410 pairs, 950 rejections and 38 curated
references; 922 old rejection observations remain, with those two exact diagnostic
text exceptions. This is bounded source support, not a general assignment policy.

## Type-only module items

Declaration-level TypeScript `import type` and `export type` items erase after the
original Module parse and Semantic pass. Import declarations, named export lists,
named re-exports and star/namespace re-exports qualify only when the declaration's
own Oxc kind is `Type`. Erasure applies only at module top level. Function/block
statements keep their original type-declaration eligibility and abrupt-tail checks;
the four newly erased module-item families do not become body statements.
Specifier-level `type` inside an ordinary import/export is
not this admission: runtime module edges and module initialization remain unsupported.
This follows [type-only erasure](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-3-8.html)
and the [declaration/specifier distinction](https://www.typescriptlang.org/tsconfig/verbatimModuleSyntax.html).

The same predicate excludes these items from runtime entry selection and charges
each original item once against the shared 256-statement budget. Original UTF-8
source bytes, directives, helpers, source order and 16 KiB bound remain intact.
No module is resolved or loaded. Type-only items cannot manufacture an exported
entry, numeric binding or runtime value. Existing SymbolId/value eligibility is
unchanged: unknown type-only names used as values reject; otherwise eligible
pristine globals and ordinary value shadows keep their existing behavior.

Corpus 49 adds 32 paired programs, including four exact original TS promotions,
and 33 classified negatives. No curated reference or Rust-only guard transfers,
retained diagnostic changes, IR/backend/runtime changes, dependency or ABI changes
are involved. That step recorded 1442 pairs, 979 rejections, 38 curated references;
56 groups classify 29 invalid sources, 141 resource restrictions and 809 open
requirements. Ambient declarations/augmentations and runtime module support remain open.

## Generic declaration metadata

Type parameters on otherwise admitted entry and helper function declarations erase
as unvalidated metadata. The original TypeScript Module AST and Semantic pass remain;
constraints, defaults and `const` type parameters do not create runtime bindings,
change numeric parameter eligibility or establish inferred runtime types. This follows
the declaration syntax in the [TypeScript generics handbook](https://www.typescriptlang.org/docs/handbook/2/generics.html).
Explicit type arguments on calls remain unsupported. Async/generator/ambient functions,
`this` parameters and other parameter-shape restrictions retain their existing guards.

Original source bytes, statement/depth budgets, SymbolIds, ordered argument snapshots,
helper signatures, const-write checks and numeric IR/Wasm remain unchanged. Corpus 50
adds 24 pairs, including three exact original exclusions, and 19 classified negatives.
Eleven retained function-shape diagnostics remove only the obsolete `non-generic`
wording; two belong to unchanged curated reference cases. No curated reference or
Rust-only guard transfers. One TypeScript runtime-forms assessment records partial
support; namespaces, enums and ambient forms remain open. That generic-declaration step recorded 1466 pairs,
995 rejections and 38 curated references; 56 groups classify 31 invalid sources,
144 resource restrictions and 820 open requirements. These are selected fixtures,
not an exhaustive language inventory.


## Primitive runtime boundary

`grunkalunka-runtime::Value` distinguishes undefined, null, booleans and numbers.
`ValueInstance` executes import-free `(f64,f64)->(i32,f64)` modules and returns that
sum type, renewing the same fuel budget as `NumericInstance`. Invalid tags or
noncanonical non-Number payloads return errors. The numeric ABI remains available.
See [ADR0002](adr/0002-tagged-primitive-values.md) for the wire contract and alternatives.
`compile_value(source, typescript)` now emits this ABI through the original Oxc
parser/semantic pass, explicit evaluator and verified IR. `compile` and the original
`compile`/`wat`/`eval` CLI commands retain the numeric profile and its artifacts/diagnostics.
The `compile-value`/`wat-value`/`eval-value` commands select the primitive-return profile.

The value profile admits explicit returns of null, unresolved global `undefined`,
Boolean literals/comparisons/negation/immutable Predicate cells, and admitted Number
expressions. Conditional arms may return different primitive kinds. Comma-expression
prefixes retain the existing discarded-expression contract; only the final operand
uses General evaluation. TS assertions and declaration metadata do not establish
runtime kinds. An initialized Number binding named `undefined` remains a Number.

The entry has two Number parameters and a General result. Helpers retain Number
parameters/results, including the existing 1000-source-parameter limit. A direct call
to the entry can produce a General result. Conditions convert it with ToBoolean;
using it in a numeric operand or numeric helper return remains rejected.
Original symbol identity, argument snapshots, source budgets and selected-arm effects
are preserved. Scalar computations retain scalar IR until an explicit `Pack` creates
a General value; there is no heap allocation or NaN boxing.

The separate [primitive-return tests](../crates/grunkalunka-runtime/tests/compiled-values.rs)
exercise 27 JS/TS pairs, 54 compiled modules and 14 input pairs: 756 calls per Wasm
engine (1512 comparisons against original JS). These finite cases do not promote
numeric-corpus or curated rejection fixtures. At that primitive-return step, corpus 50 retained 1466 pairs and 995
rejections under the numeric profile. General parameter writes, compound/update/logical assignments, mixed-kind
operators beyond strict equality, numeric coercions, empty/implicit returns, strings/objects/BigInts and
general heap integration remain open. Logical expressions still use the
existing scalar operand contracts; this is not general JavaScript expression support.


### CLI value protocol and separate reporting

Both evaluation profiles read the same two 16-digit hexadecimal binary64 input
patterns per line, with at most 4096 lines. `eval-value` writes one line per successful
call: `undefined`, `null`, `boolean:0`, `boolean:1`, or `number:<16 lowercase hex digits>`.
Number output keeps the runtime's bits, including signed zero and NaNs. Compatibility
comparisons use NaN class rather than requiring an identical NaN payload.
Value runtime errors include their cause chain; numeric error formatting is unchanged.
The evaluator uses `ValueInstance`, including its import policy, typed signature and
renewed fuel. Compilation completes admission/encoding before creating a new output;
existing files are never overwritten.

`node scripts/poc.mjs` builds offline and runs both profile reports. The separate
[primitive profile runner](../tests/compatibility/primitive-values-check.mjs) executes
46 JS/TS pairs on 14 input pairs: 2576 comparisons with original JS across Wasmtime
and Node-Wasm. It checks 4 rejected pairs and six CLI control groups, covering input
validation/batch bounds, signature separation, no-overwrite/no-artifact failures and
fuel exhaustion. Its report records exact sources, artifact/output hashes, input bits,
compiler/lock hashes, host versions, standard references and invocation results.

Twelve exact original numeric rejection sources (`boolean`, `null`, `comparison`,
`predicate-conditional-numeric-return`, and `logical-unsupported-4` through
`logical-unsupported-7`, `discarded-number-rhs`, `const-boolean-assignment-rhs`,
`discarded-predicate-binding-reject-assignment-result` and
`predicate-conditional-assignment`) execute successfully in this value profile;
the report also verifies their continued numeric-profile rejection. This is
profile-specific support, not removal of those twelve numeric rejections. The four selected value rejection pairs are not an
exhaustive value-profile gap inventory. Numeric and value reports live separately
under `target/compatibility-runs/` and `target/value-compatibility-runs/`.

## Primitive strict equality

Both profiles admit `===` and `!==` between admitted Undefined, Null, Boolean
and Number expressions in predicate positions. The Value profile can also return
the Boolean comparison result. Different kinds compare unequal; Numbers retain
ECMA-262 equality (`NaN` differs from itself, and both signed zeros compare equal).
Boolean payloads compare by value; null and undefined each equal only their own kind.
See [IsStrictlyEqual](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-isstrictlyequal).

Operands are evaluated once, left to right, into snapshots before comparison.
Existing Number/Number and same-kind scalar Boolean comparison paths retain their
original IR and Wasm. A bounded, non-emitting Number-result shape scan routes those
paths; actual lowering still validates every operand. Symbol identity determines
bindings and calls. TypeScript assertions and return annotations never prove runtime kinds.
Other admitted operand shapes use General tag/payload pairs and produce Predicate IR.

Corpus 51 promotes eight exact original rejection sources, including `gap-null`
and `gap-undefined`, with their original independent `(4,2) -> 4` curated goldens.
Ten further paired programs cover conditional/sequence operands, effects, shadowing,
helpers and contradictory TS annotations. Totals are 1484 pairs, 987 rejections and
36 curated references; the 56 groups retain 31 invalid sources, 144 resource
restrictions and 812 open requirements. A separate runtime matrix checks all 144
pairs of twelve primitive representatives with both strict operators and both
profiles: 576 modules executed by Wasmtime and Node against standard-derived results.
These finite fixtures do not establish complete language support.

Loose equality, General parameter writes,
General helper parameters/results, strings, BigInts and objects remain separate work.
Strict equality does not coerce operands and adds no heap or host capability.

## Immutable General bindings

Initialized `const` identifiers can hold admitted primitive expressions, including
null/undefined, mixed-kind conditionals, sequences and aliases of earlier General
bindings. Existing scalar Number and immutable Predicate initializer paths remain
specialized. Other immutable initializers use General evaluation, which still rejects
unsupported source operations and preserves original source depth charges.

Each General cell occupies a tag/payload pair of Wasm locals; scalar cells retain
one local. Logical SlotIds map to physical offsets independently of snapshot ValueIds.
Initialization stores both components and a read snapshots both components. This
applies in helper bodies as well as the entry, in either result profile. Parameters
and helper results remain Number-only. TS annotations do not select cell kinds.

Bindings become visible only after their initializer. Shadowing, TDZ, const writes,
lexical region ownership and per-entry loop initialization keep their existing rules.
General aliases can be returned through the Value profile, used in primitive strict
equality, or discarded as identifier expressions, including erased wrappers.
Numeric coercion, General compound/update/logical assignments and implicit
initialization remain separate work. A numeric return of a General binding is
rejected explicitly even when its current payload happens to be a Number.

Corpus 52 adds ten paired programs, including one exact original numeric rejection
promotion, for 1494 pairs, 986 rejections and 36 curated references. Two retained
rejections now diagnose their unsupported numeric read after successful initialization:
`annotation-runtime-null` and `predicate-conditional-numeric-local`. Their original
sources and inventory hashes remain unchanged. The separate Value-profile fixture
`general-local` transfers from rejection to execution with its original JS/TS sources;
that step recorded 20 pairs and seven selected rejection pairs.

## General primitive truthiness

Conditions, logical negation and predicate-position conditional/logical composition
admit null, unresolved global undefined, General bindings and General entry
call results. Each operand is captured once. Existing Number and Predicate paths
remain unchanged. The [ToBoolean rule](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toboolean)
makes null, undefined, false, signed zero and NaN false; true and other represented
Numbers are true. TypeScript assertions never change runtime truthiness.

`GeneralTruthy` converts one visible General snapshot into a Predicate. The current
canonical representation makes its payload alone sufficient: nullish payloads are
+0, Boolean payloads are 0/1, and Number payloads retain their value. Emission tests
`abs(payload) > 0`, which also rejects NaN. This proof covers only the four admitted
kinds; String, BigInt, object and HTMLDDA semantics are not represented.

Existing predicate short-circuit regions preserve skipped effects while still
validating all source branches. Nullish coalescing, numeric
coercion, General parameter writes, compound/update/logical assignments and additional runtime kinds remain separate work.

Corpus 53 adds sixteen paired programs, reaching 1510 pairs, 986 rejections and
36 curated references. No original numeric rejection transfers in this step.
The separate Value profile promotes `general-condition` with its exact original
JS/TS source: 21 pairs, six selected rejection pairs and 1176 engine comparisons.
The runtime matrix covers 144 modules with standard-derived goldens plus 48
conditional/loop/effect modules across JS/TS, both profiles and both engines.

## General-valued logical selection

`&&` and `||` accept admitted General operands in primitive value positions.
The left operand is evaluated and captured once. `GeneralTruthy` supplies its
condition; `&&` selects the right operand only for truthy left values, while `||`
selects it only for falsy left values. The skipped arm yields the original left
snapshot and the selected arm yields the right snapshot, preserving kind and
payload rather than returning a Boolean. Both source operands must be admitted.
See [logical evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-binary-logical-operators-runtime-semantics-evaluation).

The existing pending logical continuations build a General-valued `If`; no new IR
operation, heap, import, ABI or dependency is introduced. Number-only logical trees
retain scalar evaluation followed by the existing Pack in the Value profile.
Old Number/Predicate discard routes also remain unchanged. Newly eligible discarded
logical expressions use General evaluation only after those original classifiers.
TypeScript wrappers retain source depth charges and never establish runtime kinds.

Corpus 54 promotes four original numeric rejection observations of three distinct
source strings, preserving their IDs and hashes, and adds sixteen other paired
programs. Totals are 1530 pairs, 982 rejections and 36 curated references. One retained
source, `predicate-logical-numeric-local`, now diagnoses its unsupported numeric read
after successful General initialization. A General binding still cannot be returned
through the numeric ABI or used in a numeric operand without further support.

The Value profile adds four exact remaining numeric rejection sources, reaching
25 pairs, six selected rejection pairs and eight recorded numeric origins. The
runtime matrix executes 1152 JS/TS modules across both profiles and engines, checking
selected kinds, signed zero, NaN and strict-equality observations; 38 effect modules
check selected/skipped effects and aliases. Twelve saved prior numeric logical
Value-profile modules also retain exact WAT/Wasm. These are finite coverage witnesses.
Nullish coalescing, numeric coercion, General parameter writes and General logical
assignments remain separate work.

## Write-aware mutable General locals

Initialized `let` locals can change represented primitive kind through plain `=`
assignments, including a Number-initialized local later receiving null or Boolean.
A readonly pass over Oxc's original nodes and SymbolIds classifies storage from
initializers and plain writes, propagating requirements through aliases. Existing
Number-only locals keep scalar cells; immutable Predicate const specialization
also remains. TypeScript annotations never prove a runtime kind. The
[storage decision](adr/0003-local-storage-planning.md) records the conservative
fixed point, alternatives and node-index memory tradeoff.

Planning does not publish bindings or create live aliases. An alias captures the
value at its own initialization; later writes affect only the target cell. Normal
TDZ, const-write, lexical-owner and per-loop-entry initialization checks remain.
All source-local/statement/expression budgets retain their existing meaning.
Parameters retain Number source cells and the existing signature.

Plain assignment captures its RHS once, stores it and returns that snapshot.
A General result preserves tag/payload. In an admitted Number-result position,
lowering captures a Number RHS, packs it only for the General store and returns
the original Number snapshot. Conditions explicitly apply ToBoolean; no implicit
numeric conversion is introduced. General compound/update/logical assignments,
General parameter writes and implicit initialization still fail with source errors.

Corpus 55 promotes eight exact original rejection observations and adds sixteen
other paired programs: 1554 pairs, 974 rejections and 36 curated references. The
56 groups classify 799 open, 144 resource and 31 invalid observations. Two remaining
mutable Boolean numeric-read diagnostics advance past initialization without
changing their source hashes. The separate Value profile adds four mutable cases,
reaching 29 pairs, six selected rejection pairs and eight exact numeric origins.
The runtime transition matrix checks 576 JS/TS modules across both profiles and
engines, with additional alias, assignment-result, scope and budget witnesses.

## Explicit primitive numeric conversion

Unary `+`/`-` and binary `+`, `-`, `*`, `/`, `%` now accept admitted operands
of the four represented primitive kinds. A typed `NumberFromGeneral` operation
implements the [ToNumber algorithm](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumber):
undefined becomes NaN, null/false become +0, true becomes 1, and Number retains its
payload. Strings, BigInts, Symbols and objects are not represented by this operation.
For the admitted primitive domain, arithmetic follows
[ApplyStringOrNumericBinaryOperator](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-applystringornumericbinaryoperator)
without a string-concatenation branch.

The pending evaluator keeps existing Number and Predicate operand paths before
using General evaluation plus conversion. Original operands are evaluated once,
left before right, with captured values retained across later writes. Conditional,
logical, sequence and plain-assignment results can supply General operands.
General-returning calls can supply arithmetic operands in the Value profile.
Numeric return and parameter contracts remain unchanged. This does not add
relational/loose equality conversions, compound/update operators, additional
bitwise/shift operands, Math intrinsic conversions or additional source kinds.

Corpus 56 promotes nineteen exact original numeric rejection observations and
adds sixteen other paired programs: 1589 pairs, 955 rejections, 36 curated references,
and 780 open/144 resource/31 invalid observations across 56 groups. The separate
Value profile promotes the exact `coercion` sources and adds four programs, reaching
34 pairs, four selected rejected pairs and 1904 result comparisons. The exact former
`general-number-call` rejection sources now compile but recurse unconditionally;
they are retained as two fuel-exhaustion observations, excluded from result
comparisons and feature-completion counts. Existing five control groups remain,
with a sixth for that termination-only case.

The runtime matrix checks 2976 JS/TS modules across both result profiles and engines
against fixed numeric goldens, including signed zero, NaN, infinities and subnormals.
Additional effect, scope, call and original-source witnesses are finite evidence;
they do not establish exhaustive language compatibility.

## Source parameter cells

Parameters can receive represented primitive kinds through plain `=`. A readonly
plan selects a General source cell when initial Number values and later writes
cannot remain Number. The physical signature and input slots stay Number; selected
cells are initialized by loading and packing those incoming values before body
execution. Original SymbolIds then resolve to the initialized source cell.
[ADR 0004](adr/0004-general-parameter-cells.md) records the representation,
alternatives and worklist analysis.

Number-only parameters retain their previous cells and artifacts. General cells
keep captured assignment results and aliases; later writes cannot change an
already captured value. Entry and helper functions use the same rule. Generated
parameter storage does not consume the source-local budget. Existing source,
statement, local, parameter-count, signature and execution limits remain.
General call arguments, defaults/rest/destructuring and compound/update/logical
writes remain separate requirements. Discarded comma expressions with a final
General result also remain a separately observed gap.

Corpus 57 adds sixteen numeric pairs, reaching 1605 pairs with the same 955
rejections and 36 curated references. Seven rejection diagnostics advance from
parameter writes to numeric reads/returns of General cells; their exact sources
and inventory records remain. The Value profile executes the four distinct
original JS sources represented by those seven observations, with four pinned
numeric origins, and adds four parameter cases: 42 pairs, four selected rejected
pairs, twelve numeric origins, 2352 result comparisons and six control groups.
The two existing termination-only observations remain outside result comparisons.

Finite runtime coverage includes 96 kind-transition modules, 64 snapshot/effect
modules, a 1000-parameter helper with a changed source parameter, and the unchanged
128-source-local budget. A 1000-parameter dependency chain is compiled/validated
without claiming execution within the normal call-fuel budget. Its discarded
sequence ends with scalar zero to isolate that existing expression-routing gap.

## Discarded primitive expressions

When an expression result is unused by the supported function contract, lowering
still evaluates it and performs GetValue. Nullish literals and admitted General
sequence/conditional/call results can now be discarded. Existing positive General
binding/assignment, Predicate, General logical and Number routes retain priority;
only the remaining expressions use General evaluation followed by Forget.
No numeric conversion is added. The same original AST nodes, pending continuations
and source-depth charges remain in use.

Sequence operands retain left-to-right evaluation, and only a selected conditional
or logical branch executes. Discard does not excuse TDZ, unknown references,
const writes, unsupported operands in unselected branches, source budgets or return
policies. General calls keep their existing argument/signature contracts. No new
operator, source kind, host capability, ABI or IR operation is introduced.

Corpus 58 promotes four exact original numeric observations and adds sixteen
other pairs: 1625 pairs, 951 rejections, 36 curated references, and 776 open/144
resource/31 invalid observations in 56 groups. Remaining rejection diagnostics
and records retain their original identities. The Value profile adds four cases,
reaching 46 pairs, four selected rejected pairs, twelve numeric origins, 2576 result
comparisons, six controls and the same two termination-only observations.

Finite runtime checks cover 288 primitive-form modules, 64 effect modules, fourteen
exact former guard modules and existing depth/statement/local limits. The original
1000-parameter chain that ended in a discarded General assignment now compiles and
validates without a trailing scalar. It is instantiated/validated in both engines;
no execution or call-fuel result is claimed for that large witness.

## Loose equality of represented primitives

Loose `==` and `!=` support the four represented primitive kinds: undefined,
null, Boolean and Number. Both nullish kinds compare equal to each other; neither
compares equal to a Boolean or Number. Boolean/Number comparisons use the Boolean
numeric value. NaN remains unequal, and signed zeros compare equal. This follows
[ECMAScript IsLooselyEqual](https://tc39.es/ecma262/#sec-islooselyequal).
String, BigInt, Symbol, objects and the optional IsHTMLDDA behavior remain outside
this domain; the curated string-coercion equality gap remains rejected.

The typed `PrimitiveLooseEqual` operation consumes two General snapshots and
produces a Predicate. With canonical nullish payloads of +0 and Boolean payloads
of 0/1, emission compares the nullish/non-nullish partition and the numeric
payload. This formula depends on the four-kind invariant; adding another kind
requires revisiting it. Both operands evaluate once, left to right. Existing
positive Number comparisons and strict equality retain their previous routes.

Corpus 59 promotes 19 exact rejection observations (17 distinct JS sources),
paired with TypeScript companions, and adds effect and type-wrapper cases:
1660 pairs, 932 numeric rejections and 36 curated references. The 56 inventory
groups now contain 757 open requirements, 144 resource restrictions and 31 invalid
sources. No retained rejection diagnostic changes. Value reporting adds four
cases, reaching 50 pairs, four selected rejected pairs and 2800 result comparisons;
twelve numeric origins, six control groups and two termination-only observations
remain unchanged. These finite cases do not establish complete language support.

## Primitive numeric updates

Prefix/postfix `++` and `--` now admit initialized mutable identifier bindings
holding undefined, null, Boolean or Number. Within this represented domain,
[ToNumeric](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-tonumeric)
uses the existing numeric conversion. Lowering loads once, converts the old value,
adds/subtracts one and stores a canonical Number in the original cell. Postfix
returns the converted old Number; prefix returns the new Number, following
[update-expression evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-update-expressions).
Existing Number slots retain their scalar operation sequence. No new IR operation,
ABI, dependency or storage-planning rule is needed.

Const writes, TDZ, unknown/function/property targets, source budgets and numeric
return restrictions remain in force. BigInt, String and object update semantics
remain separate requirements. Compound and logical assignments are separate work.

Corpus 60 adds 20 paired programs, reaching 1680 pairs. The existing rejection
inventory stays at 932 records and 36 curated references: none of those original
negative programs is newly admitted by this unit. Three exact former Rust update
guards are now executed. Value reporting adds four pairs, reaching 54 pairs and
3024 result comparisons, with four selected rejected pairs, twelve numeric origins,
six control groups and two termination-only observations unchanged. Finite update
coverage includes 384 modules with independent numeric goldens and 64 effect/order
modules across JS/TS, both profiles and Wasmtime/Node.

## Implicit let initialization

A bare `let` identifier initializes canonical Undefined when its declaration
executes, following [lexical BindingEvaluation](https://tc39.es/ecma262/multipage/ecmascript-language-statements-and-declarations.html#sec-let-and-const-declarations-runtime-semantics-bindingevaluation).
It uses a General slot and the existing initialization/publication path. The value
is intrinsic; a lexical binding named `undefined` cannot change it. Declarators
execute in source order, and declarations inside loops initialize on each execution.
Existing explicit-initializer Number/Predicate/General routes retain their operations.

Pre-declaration reads, initializer self-reference, shadowed TDZ references, missing
const initializers, var, destructuring and TS definite-assignment assertions remain
outside admission. Scope and local/statement limits are preserved. No IR, ABI,
capability, dependency or storage-planning change is introduced.

Corpus 61 promotes four exact original sources (three JS, one TS) and adds sixteen
pairs: 1700 pairs, 928 numeric rejections, 36 curated references. The 56 inventory
groups classify 753 open, 144 resource and 31 invalid observations. The exact
`gap-uninitialized-let` source remains rejected in the numeric profile: its diagnostic
advances to the separate General-cell return restriction. Its original source now
executes in the Value profile as a thirteenth numeric origin; it is not counted as
numeric support. Value reporting reaches 59 pairs and 3304 result comparisons,
with four selected rejected pairs, six control groups and two termination-only
observations unchanged. Coverage includes 56 fixed-golden modules, 64 effect/order
modules, shadowed `undefined`, loop reinitialization and a default-kind mutation.

## Primitive arithmetic compound assignments

The five arithmetic `+=`, `-=`, `*=`, `/=` and `%=` operators now admit represented
primitive RHS values and mutable Number/General identifier targets. Lowering captures
the old left value before evaluating the RHS, computes a Number, stores it in the
original slot and returns that Number snapshot. General slots receive a canonical
packed Number; Number slots retain scalar storage. This follows
[compound-assignment evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation).

RHS evaluation uses the existing arithmetic conversion. A captured General left
value uses the existing NumberFromGeneral operation before the binary operation.
Conversions are pure for the four represented primitive kinds. Adding object or
other observable conversion behavior requires revisiting conversion order; this
unit does not admit those kinds. Existing positive Number operand paths retain
output. No IR, ABI, dependency or storage-planning change is introduced.

Const/TDZ/property/function targets and source budgets retain their restrictions.
String/BigInt/object semantics and bitwise, shift, exponent and logical compound
assignments remain separate requirements. Numeric General-cell return restrictions
remain; the assignment expression itself has a Number result.

Corpus 62 promotes five exact rejection observations (three distinct original JS
sources) and adds sixteen other pairs: 1721 pairs, 923 numeric rejections and 36
curated references. The 56 inventory groups contain 748 open, 144 resource and 31
invalid observations. No retained rejection diagnostic changes. Value reporting
adds four cases, reaching 63 pairs/3528 comparisons; thirteen numeric origins,
four selected rejected pairs, six controls and two termination-only observations
remain unchanged. Runtime coverage includes 1280 independent-golden modules,
64 effect/order modules and 26 exact former Rust guard modules across eligible
JS/TS/profile combinations, plus two semantic mutations.

## Primitive relational comparisons

`<`, `<=`, `>` and `>=` now admit the four represented primitive kinds through
existing arithmetic conversion and Number comparisons. Undefined converts to NaN,
null to +0, and Booleans to 0/1. All four relations are false when either converted
operand is NaN. In particular, `<=` and `>=` cannot be implemented as simple
negations of the opposite strict relation. This follows
[IsLessThan](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-islessthan)
and [relational evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-relational-operators-runtime-semantics-evaluation).

Both operands evaluate once, left to right, retaining the left snapshot across RHS
writes, including `>`/`>=`. Conversions are pure for these four kinds. Equality
routes and their strict Boolean specialization remain separate; positive Number
operand paths retain their previous output. No IR, ABI, dependency, storage-planning
or source-budget change is introduced. Strings, BigInts, Symbols, objects and
observable ToPrimitive conversion remain outside this domain.

Corpus 63 promotes two exact original JS observations and adds sixteen pairs:
1739 pairs, 921 numeric rejections and 36 curated references. Its 56 inventory
groups contain 746 open, 144 resource and 31 invalid observations. No retained
rejection diagnostic changes. Value reporting adds four pairs, reaching 67 pairs
and 3752 comparisons; thirteen numeric origins, four selected rejected pairs, six
control groups and two termination-only observations remain unchanged. Coverage
includes 1024 independent-golden modules, 64 effect/order modules, twelve exact
former Rust guard modules and an operand-order mutation, across JS/TS and both
profiles. Predicate-depth, local and statement limits retain their guards.

## General logical assignment

Initialized mutable identifier cells admit `&&=` and `||=` for represented
Undefined, Null, Boolean and Number values. Following
[logical-assignment evaluation](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-assignment-operators-runtime-semantics-evaluation),
lowering captures the old value once, tests its truthiness and evaluates/stores the
RHS only when selected. The result is the exact selected RHS or skipped old
snapshot, preserving kind, signed zero and NaN class. Later writes cannot change it.
The complete RHS must still be admitted when runtime selection would skip it.

Read-only storage planning includes possible logical RHS writes and propagates
kind changes through alias dependencies. Number-initialized locals and source
parameters can therefore use existing General cells; physical Number parameters
and Number-only paths remain unchanged. Existing General `Load`, `GeneralTruthy`,
typed `If` and selected-child `Store` operations suffice, without new IR or ABI.
Numeric results use an explicit admitted conversion such as `+(value ||= true)`;
a General assignment result is still rejected in a Number-only return position.
Const/TDZ/property/global targets, `??=`, unrepresented kinds and source budgets
retain their restrictions.

Corpus 64 adds twenty numeric pairs, reaching 1759 pairs and 921 rejections with
all 36 curated references retained. Eight individually pinned diagnostics advance
to the General-result restriction; no numeric rejection is promoted. The Value
profile adds six pairs, including two unchanged numeric rejection sources, reaching
73 pairs/4088 comparisons and fifteen numeric origins. Four selected rejected
pairs, six controls and two termination-only observations remain. Runtime tests
cover 1600 independent-golden modules, 64 effect/order modules and sixteen exact
former Rust guard modules across eligible JS/TS/profile combinations. Two semantic
mutations detect a missing selected Store and reversed branch selection.


## Local arrow calls and shared captures

Synchronous arrows with expression or block bodies can be bound to `const`, aliased by another
`const`, or called immediately. Calls require exactly the declared number of
numeric arguments and a numeric body result. Parentheses and erased TypeScript
wrappers preserve the underlying function; annotations do not coerce values.

Known concise arrow factories can also return an arrow or immutable closure
alias into a `const` binding. New arrows retain fresh creator parameter cells;
existing returned aliases keep their cells. Leaf calls retain the numeric contract.
See [returned closures](returned-closures.md)
for specialization, the shared generation budget and remaining function-value gaps.

Initialized local function values can also be selected at runtime and called through
an identifier or alias in the Number, primitive and primitive async profiles. Internal code/environment
pairs and table calls retain typed captures; packed argument frames handle up to
1,000 logical Number/callback arguments with fixed physical parameter lists. The
exported ABI is unchanged. See [runtime closures](runtime-closures.md) for the bounded source forms,
environment verifier, allocation, async frame retention and remaining general-value exclusions.
`let` function slots support same-full-signature `=`, `&&=`, `||=` and `??=` assignments
and constructor aliases. The shared logical-assignment cursor uses typed function
loads, stores and joins: functions are truthy and non-nullish, so only `&&=` selects
its RHS. No new IR operation or representation is needed.
An indirect call captures its callee pair before argument side effects or await;
assignment cannot retroactively change that snapshot. Captured mutable function
slots can be rebound and can form recursive calls through initialized cells.
Known local arrows and factories also accept fixed-signature scalar-result callbacks.
Original-symbol demand analysis checks fixed callback arities through aliases and
forwarding; actual specialization initializes typed function parameter cells.
See [local callback arguments](local-callback-arguments.md) for snapshot order,
factory capture and definition checking. Runtime-selected higher-order functions
accept finite scalar-result callbacks, including callbacks taking callbacks; [runtime factories](runtime-function-results.md)
return function pairs with Number or these callback parameters through a separate private pair-return call
type. Selected and assigned signatures include both parameter and result kinds.
Complete block creators use the existing owned-return query and typed `InlineCall`
boundary for branches, loops and early returns; their implicit/bare or mixed-kind
creator completion remains rejected. The signature query and concise-only guard
were extended without a new IR operation or backend path.
Returned functions may themselves return functions through strictly earlier,
acyclic signature IDs. The shared result validator walks that chain without native
recursion, admitting Number parameters or acyclic scalar-result callbacks at each
returned layer; primitive leaves
retain the existing General or Number call ABI. Exact signature matching remains.
Returned callbacks retain both code/environment fields in the existing ordered
argument frames and mutable parameter cells, including across async caller waits.
Callback arguments return Number or General represented primitives; their
parameters may themselves be callbacks. Original-symbol shape refinement uses
concrete constructors and demanded formal forwarding without forcing generic identity formals. Both refinement phases
are bounded to eight rounds. The IR verifier walks callback parameter graphs
iteratively, checking earlier signature references, Number/General results and widths.
General-result callback facts propagate through storage planning and constructor
signatures rather than retaining a Number assumption. Async callbacks, ordinary
General parameters and cyclic callback type metadata remain unsupported.

The compiler specializes these statically known calls into the caller's typed regions.
Capture metadata records lexical SymbolIds and original variable slots. A call
reads the current contents of those slots, and writes from sibling closures share
the same cells. Capturing a value does not snapshot it. Arguments evaluate left to
right before entering the body, with fresh parameter slots per expansion; caller
bindings are restored after lowering the body. Unused arrow bodies are checked in
a scratch lowering context without emitting their effects into the caller.

Block bodies can use the admitted statements, local declarations, branches and
loops, and must return a Number on every structural path. A typed `InlineCall`
region gives the body its own return boundary: an early return ends the closure
call, then the caller continues. Captured writes remain visible to the caller.
Fresh locals and nested closure metadata belong to that specialization. The body
has its own loop-control and return-type context, including in primitive or async
callers; it cannot break the caller's loop or settle the caller's async result.

Actually referenced captures now use stable typed cells in a bounded private
guest-memory arena. Helpers can return while their cells remain intact; external
invocations reset the arena and internal recursion preserves it. The async frame
retains cell pointers across jobs. This adds a storage lifetime prerequisite,
without admitting returned function values. See [captured cells](captured-cells.md)
for layout, the 1 MiB bound, reclamation and remaining environment work.

The original source limits still count source locals/statements. Generated arrow
parameter slots do not consume the source-local quota. Separate generation limits
allow at most 256 arrow-call expansions per function and at most 128 nested
expression regions while expanding a concise arrow. Block calls additionally allow
eight active block-body expansions and 128 aggregate nested regions across their
statement/expression callers. This bounds the statement lowerer's native recursion;
the eight-call case is exercised on a 2 MiB thread stack. Source nesting inside each
body is checked independently. Body statements and locals are charged once at the
definition, including unused bodies; aliases and specialization do not recharge them.
These limits bound specialization, not JavaScript
language validity or runtime termination; the existing runtime fuel is unchanged.

Captures must already be initialized when the arrow is defined. General first-class
function values, recursive initialization, implicit undefined returns, defaults/rest,
optional/generic calls, lexical `this`/`arguments` and async arrows remain outside
this slice. General closure environments remain subsequent work; numeric suspension
is available in the separate [async profile](async.md).

Corpus 65 adds 22 pairs, including the two exact original `gap-call` and
`gap-closure-capture` sources retained with their hashes. It has 1781 pairs,
919 rejections and 34 curated references; 744 observations remain open requirements,
144 resource restrictions and 31 invalid sources. The other 919 original rejection
sources retain their diagnostics. This is a demonstrated closure slice, not full
closure or async support. See the
[arrow creation semantics](https://tc39.es/ecma262/multipage/ecmascript-language-functions-and-classes.html#sec-runtime-semantics-instantiatearrowfunctionexpression).

Known concise local calls also return existing same-arity function pairs and can
be invoked immediately. Classification is effect-free; actual arguments execute
once in caller order. See [local function results](local-function-results.md).

Represented synchronous function values now enter Boolean contexts through their
existing typed value evaluator and a true predicate. Source expression effects run
once without invoking the tested function. Numeric coercion and function transport
remain separate; see [function truthiness](function-truthiness.md).

## Binary nullish coalescing

Binary `??` admits Undefined, Null, Boolean and Number operands in the Value and
async profiles. It evaluates the left operand once and keeps its exact kind and
payload unless it is Null or Undefined. Only then does it evaluate the RHS.
False, zero, negative zero and NaN keep the left value. Both operands still undergo
source admission, scope checks and resource limits, even when RHS execution is skipped.

Number-only value positions admit `??` when both operands lower as Numbers.
Predicate contexts first select the primitive value, then apply ToBoolean; `0??b`
therefore remains false regardless of b. Discarded results retain operand effects.
Async operands preserve the selected awaits, fulfillment/rejection and catch behavior;
skipped awaits create no reaction jobs. TypeScript claims do not change these facts.

The three exact former numeric rejections in `primitive-nullish.mjs` now execute.
The Value profile also executes its original `nullish-operator` rejection and the
unchanged curated `gap-nullish` source. Although that source always selects a Number,
the compiler conservatively retains a General join and still rejects its numeric-only
return. This is a missing narrowing proof, not a possible null result. The primitive
report at that step had 82 JS/TS
pairs, 16 exact numeric origins, 6 rejection observations and 4592 comparisons.

Function-only `??` still requires matching complete signatures. The constructor-free
`(x,y)=>x??y` selector now passes preliminary Number checking and specializes from
its actual function arguments. General function/primitive joins, String/BigInt/object
operands remain separate requirements. Mutable primitive `??=` and void are described below.
No dependency or host ABI changes.

## Primitive results from concise closures

Concise synchronous local arrows now return represented Undefined, Null, Boolean
and Number values through direct specialization and runtime closure calls. For
example, `const f=()=>null; const g=()=>false; let h=a?f:g; return h();`
retains the selected primitive kind. General results use the existing `(i32,f64)`
layout and a matching private table signature. Number-only bodies keep their
Number result specialization. Signature checks include the result kind.

Calls preserve callee and argument snapshots, captured cells, effects, and async
caller job order. Primitive results may initialize or update General cells and
enter predicates or explicit numeric coercions. Existing local function-return
paths can forward these closures; this includes the two former maintained
rejections `id(()=>true)` and a block returning `()=>true`. A dormant factory
containing `()=>true` is also valid when the entry returns a Number. Runtime factories
now return primitive-result leaves as well, while retaining Number leaf parameters
and matching full signatures. Arbitrary primitive arguments and
exported function values remain separate requirements. Primitive block returns
and suspending known async bodies are described below. All 916 numeric rejection sources and diagnostics
are unchanged. Nine maintained primitive corpus pairs exercise these results.

## Explicit primitive returns from block closures

Synchronous block-bodied local arrows now return represented primitive expressions
through specialization and runtime calls. For example,
`const f=()=>{if(a)return null;return false;}; return f();` preserves the selected
kind. Returns inside nested blocks and loops follow the existing structural
completion rules. A nested arrow's returns belong only to that nested arrow.
That initial slice required an explicit represented return on every structural
path; local bare and implicit returns are described below.

The existing typed `InlineCall` result now accepts General. The verifier checks
each return against the result type; existing backend layouts carry `(i32,f64)`
without a new IR operation or emitter path. Number bodies retain their Number
specialization, and function-returning bodies retain their existing signature
rules. Captured cells, argument/callee snapshots, early returns and loop effects
are preserved. Async callers retain primitive values across jobs and match native
fulfillment, rejection and catch ordering.

Eight additional maintained primitive corpus pairs brought that profile to 99 pairs
and 5544 comparisons. Suspending async arrow bodies are described below;
arbitrary primitive arguments, mixed function/primitive returns and expanded
runtime factories remain separate work.

## Local bare returns and normal completion

Synchronous local primitive closures now yield Undefined for a bare `return;`
or normal completion. A partial return such as `()=>{if(a)return b;}` retains
Number on its returning branch and Undefined on the other path. Empty bodies,
early returns, nested blocks and loop exits follow the same rule. The synthesized
value is intrinsic: a source binding named `undefined` cannot change it.

A read-only completion query uses the existing completion algebra to retain
Number specialization only when every structural path returns a Number. Lowering
then validates the body and adds the Undefined return only to normal paths.
Bare returns are admitted inside local primitive block lowering; existing entry
and named-helper completion contracts remain unchanged. Function/primitive mixed
returns still require a broader value domain. No IR operation, emitter path,
dependency or host ABI was added.

JS/TS tests compare native JavaScript, Node Wasm and Wasmtime for kinds, bit
patterns, captures, callee/argument snapshots and effects. Async callers also
match fulfillment/rejection and reaction-job order. Ten additional maintained
primitive corpus pairs bring that profile to 109 pairs and 6104 comparisons.

## Directly awaited known async arrows

The async entry profile now admits directly awaited calls to known const, alias
and direct async arrows whose bodies return or throw represented primitives.
Ordered arguments and the synchronous body prefix execute before the caller's
reaction. Both body forms may await primitives or known async calls, including
rejected waits. Explicit block, bare and implicit primitive returns reuse
the existing local evaluation paths. TypeScript type claims do not replace the
actual result kind.

These async arrows have no ordinary scalar closure signature. Calls in other
positions and dynamic/mutable async function values remain rejected, as do
Promise/function-valued outcomes. Pristine
`Promise.resolve` wrapping is supported. A rejected async-call result would be a
Promise-valued reason and is rejected; awaiting the call first yields a supported
primitive reason. See [the async profile](async.md#suspension-and-host-jobs).

Known async bodies preserve their own await reactions before
the separate caller reaction. `const f=async()=>await a; return await f();` has
two jobs. Nested fulfilled calls, conditional waits, argument waits and retained
callback pairs use the same existing continuation state. Block bodies also retain
local/captured cells, loop control and early, bare or implicit return completion.
The structured General-result `AwaitCall` operation verifies a complete callee
return/throw region. Its async transformation sends each completion through the
caller's separate reaction, retaining nested callee catches and enclosing caller
catches. Arguments execute outside that protected region. Both settlement paths
count as one logical await site, and the existing runtime job bound still applies.
Synchronous emitters and lifted ordinary functions reject this operation. Number
results are packed at the boundary and keep their proved numeric use afterward;
immediately invoked async blocks without a positive Number proof keep General.
No dependency, Promise-object representation or host ABI was added.

The root async entry also fulfills intrinsic Undefined on bare return or normal
completion. Lowering adds that result only to normal paths, preserving existing
return/throw alternatives and await/catch reaction jobs. Numeric and primitive
entries and named helpers keep their explicit completion rules. Source bindings
and TypeScript return claims cannot replace the synthesized result.

Existing synchronous Number source helpers are also available in async programs.
Calls remain synchronous and preserve ordered argument snapshots around waits;
helpers keep ordinary local activations, existing arity limits and Number
completion rules. The async emitter retains verified logical function IDs by
placing `$advance` at ID zero and helper/lifted definitions before the exported
protocol wrappers. Closure table entries still name their lifted bodies. Helpers
with captured cells request the shared cell arena even without a function table.
Calls back to the async source entry remain forbidden throughout the module, and
helper traps stay terminal rather than entering JavaScript catches.

## Void expressions

`void operand` evaluates an admitted operand completely, including its GetValue,
effects and abrupt completion, then yields intrinsic Undefined. Its result is
false in predicates and becomes NaN through the existing explicit numeric
conversion paths. Discarding a void expression still evaluates the operand.
Closures preserve that result kind; async operands retain their reactions and
rejections. A source binding named `undefined` cannot change the result.

The expression cursor uses existing General values and the Undefined IR operation;
no emitter, IR schema or host ABI was added. Numeric bare-Undefined returns,
unsupported operand kinds/calls and source/resource limits retain their contracts.
The original `gap-void` Number-comparison source and fixed golden are now executed
unchanged. Corpus 67 has 1,799 numeric pairs and 915 rejections; six additional
primitive pairs bring that profile to 115 pairs. See the
[ECMAScript void semantics](https://tc39.es/ecma262/multipage/ecmascript-language-expressions.html#sec-void-operator).

## Nullish assignment

Mutable Number/General identifier cells now admit `??=`. The old value is loaded
once; non-nullish values skip RHS evaluation and the store, including false,
zero, negative zero and NaN. Null and Undefined select the RHS, then store and
return its exact primitive kind and payload. Captured/parameter cells, RHS writes
to the same cell, nested assignments and async suspension keep those snapshots.
An abrupt RHS completion performs no assignment store.

Number-only paths retain scalar storage; General joins keep the current explicit
numeric-conversion policy. Const and property targets and unrepresented operand
kinds remain outside this slice. Function targets use the same-signature contract
in [runtime closures](runtime-closures.md). The existing nullish predicate, typed
regions and stores suffice; no IR schema, emitter or host ABI changed. The original
`unsupported-compound-10` source and Number golden execute unchanged. Corpus 68
has 1,807 numeric pairs and 914 rejections; the primitive profile has 121 pairs.
