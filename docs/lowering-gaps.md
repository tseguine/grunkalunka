# Known lowering gaps

The current compiler admits one exported two-number function with numeric locals and immutable Boolean predicate locals,
Number/Boolean expression statements, empty statements, assignments, branches, while/for/do loops,
TypeScript expression wrappers, unresolved `Infinity`/`NaN` reads, unlabelled loop break/continue and returns.
Source inspection has not established an admitted wrong-result bug. The gaps below are currently rejected language
features or valid program shapes outside that contract.

Selected primitive-Boolean operands of unary `+`, `-` and `~` explicitly produce
Numbers. This does not admit general Boolean storage, Boolean ABI returns or other
primitive coercions; none of the 38 curated gaps is promoted by unary plus/minus.

The executable frontier contains 38 curated reference cases in
`tests/compatibility/lowering-gaps.mjs`. Each has an independent expected numeric
result in JavaScript and a required current compiler diagnostic. The source is
retained in the compatibility run alongside its hash and the separate oracle source.

| Area | Exact fixture IDs |
| --- | --- |
| Operators and coercive equality | `gap-power`, `gap-loose-equality-coercion`, `gap-nullish`, `gap-typeof`, `gap-void`, `gap-delete`, `gap-property-in`, `gap-instanceof` |
| General JavaScript values | `gap-string-coercion`, `gap-object-property`, `gap-array-value`, `gap-bigint`, `gap-null`, `gap-undefined` |
| Additional control flow | `gap-switch`, `gap-label` |
| Function values, nested declarations and nested recursion | `gap-call`, `gap-nested-function`, `gap-recursion` |
| Module loading | `gap-module-import` |
| Bindings and parameter forms | `gap-var`, `gap-var-hoisting`, `gap-uninitialized-let`, `gap-local-destructure`, `gap-destructuring-write`, `gap-defaults`, `gap-rest` |
| Closures | `gap-closure-capture` |
| Exceptions and completion | `gap-catch`, `gap-finally-return` |
| Classes and instances | `gap-class-instance` |
| Generators and async | `gap-generator`, `gap-async-await` |
| PoC entry and source restrictions | `gap-export-name`, `gap-one-parameter`, `gap-extra-export` |
| Conservative return analysis | `gap-while-guaranteed-return`, `gap-unreachable-after-return` |

This separates missing operators/value semantics from functions/modules and later
closure, exception and async work. Export naming/arity and
structural return rules are deliberate PoC admission restrictions, rather than
invalid JavaScript. Source-size, depth, statement/local counts, input-batch and execution-fuel
limits remain separate [resource bounds](compiler.md#experimental-contract).

The [source-bound rejection inventory](../tests/compatibility/rejection-inventory.json)
covers every current negative, beyond these 38 numeric reference opt-ins. Its 56
groups classify 29 invalid sources, 141 resource restrictions and 809 open requirements
across all 979 records. Each pins the exact source, extension and strict Module mode;
group evidence, descriptive dependencies, harness needs and observable completion
criteria keep future work visible. Source invalidity is assessed against the named
language rule, not inferred from the compiler's rejection. Valid const writes, TDZ
and unresolved references can throw only when evaluated; unreachable branches may
avoid those errors. PoC ABI/entry/return restrictions remain open requirements.

Input and completed-report validation reject missing or stale inventory metadata.
The existing fixture sources, expected diagnostics and curated independent oracles
are preserved. A successful rejection test still records compiled lowering as
`not-executed`; no additional negative source is executed in Node. Classifying this
finite inventory is coverage bookkeeping, not closure of its open requirements or
proof of complete ECMAScript/TypeScript conformance.

The former `gap-as`, `gap-angle-assertion`, `gap-non-null` and `gap-satisfies`
references are now admitted as `ts-erasure-as`, `ts-erasure-angle`,
`ts-erasure-non-null` and `ts-erasure-satisfies`, with additional nested, effect,
predicate and boundary cases in
[`typescript-erasure.mjs`](../tests/compatibility/typescript-erasure.mjs).
These expression wrappers erase without adding type checking;
Numeric assignment/update targets now also accept those four retained wrappers,
with original symbol identity and per-retained-node depth accounting. Property,
destructuring, const, TDZ, unresolved and function-binding writes remain rejected.

The former `gap-for` reference is now `for-promoted-reference` in
[`numeric-for.mjs`](../tests/compatibility/numeric-for.mjs), with additional scope,
ordering, omission and source-budget checks. `for-in`/`for-of`,
labels remain unsupported.

The former `gap-do` reference is now `do-promoted-reference` in
[`numeric-do.mjs`](../tests/compatibility/numeric-do.mjs), with at-least-once,
condition-order, scope, return-coverage and source-budget checks.

The former numeric `gap-loose-inequality` reference is now
`inequality-promoted-reference` in
[`numeric-equality.mjs`](../tests/compatibility/numeric-equality.mjs). Both loose
equality operators admit numeric operands only. `gap-loose-equality-coercion`
retains its source, independent result and oracle, and now records rejection of
the string operand. Coercive Boolean equality remains outside admission; selected
same-kind strict Boolean equality is covered below.

The former `gap-infinity-global` and `gap-nan-global` references are now
`intrinsic-infinity-promoted-reference` and `intrinsic-nan-promoted-reference` in
[`intrinsic-numbers.mjs`](../tests/compatibility/intrinsic-numbers.mjs). Only
unresolved reads of those exact names become numeric constants. Resolved shadows
retain lexical binding and TDZ rules; global writes, other globals and property
access remain outside admission.

## What the tests establish

Only these explicit references opt into Node execution. All other rejection
fixtures remain unexecuted in Node, including malformed, effectful and potentially
nonterminating inputs. References run in disposable child processes with a five
second deadline. This is a timeout boundary for curated repository tests, not a
sandbox for arbitrary programs. No network, external services or package installs
are used. The module example imports only a literal data URL.

Arguments are restricted to finite primitive numbers without negative zero; default
parameter cases use an empty argument list. Results use binary64 bit transport to
preserve signed zero and infinities; NaNs compare by class. Async references are
awaited to observe the final numeric result, without claiming job-order conformance.
The TS-only cases use explicitly paired JS erasures; no TypeScript checker or
upstream compiler is run to validate those erasures.

Each completed rejection fixture with a `loweringGap` record separates:

- `oracle`: observed reference result, expected bits, arguments, entry and source hash.
- The ordinary admission result: the required diagnostic and absence of a Wasm artifact.
- `lowering`: `executed: false`, `outcome: "not-executed"`.

A passing admission check does not mean the missing feature works. A reference
whose expected result, execution, diagnostic or artifact absence does not match
fails the run. The report requires one gap observation per selected gap fixture.

Some examples necessarily depend on other missing features. Those dependencies
are explicit in each record's `prerequisites`: recursion also needs functions and
calls, generators need iterator objects, and a module import is first blocked by
the current single-top-level-function rule. Such a rejection identifies the first
blocking frontier; it does not independently exercise every deeper missing stage.

Compound arithmetic assignments, prefix/postfix increments/decrements and
parenthesized expressions, discarded numeric statements and numeric comma expressions have graduated into the admitted differential
corpus, including ordered mutation and returned-value regressions.

Numeric `&&` and `||` now short-circuit and preserve their selected operand.
Outside the explicit inline primitive bridges, Boolean operands (including comparison
results) remain outside numeric value admission. In predicate positions logical operators may combine admitted
predicates and numeric truthiness, preserving short-circuit effects. Nullish
coalescing remains outside admission in both contexts.

Boolean literals and conditional arms are admitted in predicate positions.
Mutable Boolean locals/Boolean returns and Boolean-valued arms in numeric ternaries
remain outside admission. Equality admits the selected same-kind strict operands
described below; other Boolean equality operands remain outside admission.
Logical negation is admitted in predicate positions, including nested negation and
negated numeric comparisons. That feature itself adds no storage or numeric
coercion; immutable Boolean predicate locals are covered by the const policy below.

## Next steps

Promote an implemented case deliberately into the admitted differential corpus,
then test its supported domain and boundary behavior against both Wasm engines.
Do not merely remove the diagnostic check, count an expected rejection as support,
or change the reference result to match a lowering bug. The 38 examples are a
concrete starting frontier, not an exhaustive ECMAScript conformance inventory.
Test262 and the other [external suites](testing.md) provide broader future coverage.

Type checking/type queries, a general JS-value ABI and the bridge between compiled
code and WIT capabilities remain separate roadmap work. These tests do not select
a value representation, GC strategy, final entry ABI or capability-profile defaults.

The former `gap-bit-not` reference is now `bit-not-promoted-reference` in
[`numeric-bit-not.mjs`](../tests/compatibility/numeric-bit-not.mjs). Numeric unary
`~` truncates and wraps to signed 32-bit bits before complementing and returns a
Number. Selected inline Boolean operands and initialized immutable Predicate bindings
are admitted through the documented [unary bridges](compiler.md#unary-reads-of-immutable-predicate-bindings).
String and BigInt operands remain outside admission.

The former `gap-bit-and`, `gap-bit-or` and `gap-bit-xor` references and ordinary
`bitwise` rejection are promoted in
[`numeric-bitwise-binary.mjs`](../tests/compatibility/numeric-bitwise-binary.mjs).
Both operands must be Numbers and execute eagerly in source order; each truncates
and wraps before the operation returns a signed numeric result. Mixed-type
coercion remains outside admission.

The former `gap-shift-left`, `gap-shift-right` and `gap-shift-unsigned` references
and `bitwise-shift-remains` rejection are promoted in
[`numeric-shifts.mjs`](../tests/compatibility/numeric-shifts.mjs). Number operands
use five masked count bits; `<<`/`>>` yield signed Numbers and `>>>` yields an
unsigned Number. Nonnumeric operands remain rejected.

The ordinary `unsupported-compound-2` through `unsupported-compound-7`,
`bitwise-compound-remains` and `shift-compound-remains-0` through
`shift-compound-remains-2` rejections are promoted in
[`numeric-bitwise-compounds.mjs`](../tests/compatibility/numeric-bitwise-compounds.mjs).
These six mappings reuse target capture, RHS evaluation, store and result snapshots.
The curated gap count is unchanged. At that step, ordinary IDs 0, 1, 8, 9 and 10 retained
their original `%=`/`**=`/`&&=`/`||=`/`??=` programs; unsupported write targets and
nonnumeric RHS values remain outside admission.

The former `gap-break` reference is now `break-promoted-reference` in
[`numeric-break.mjs`](../tests/compatibility/numeric-break.mjs). Unlabelled break
exits the nearest while/for/do loop, skipping the for step or do condition. Mixed
break/return paths do not prove a numeric return. Switch, labels and
try/finally remain outside admission.

The former `gap-continue`, `for-continue`, `do-continue` and
`break-continue-remains` references are now finite admitted cases in
[`numeric-continue.mjs`](../tests/compatibility/numeric-continue.mjs). Continue
reaches the nearest loop's while test, for update then test, or do test. The old
potentially infinite `while-continue` rejection is retired without executing its
source; finite terminal-test and nested-target cases replace its coverage.

The former module/function directive references are now paired full-source cases
in [`directives.mjs`](../tests/compatibility/directives.mjs). Only Oxc-recognized
prologues erase, and each source directive still counts toward the statement limit.
Non-prologue strings remain rejected; no general string ABI is added.

The former `gap-hashbang`, ordinary hashbang and directive/hashbang rejections
are now paired full-source cases in [`hashbangs.mjs`](../tests/compatibility/hashbangs.mjs).
Only Oxc-recognized initial hashbang comments are admitted, with no source statement
charge or IR. Original UTF-8 source bytes still count toward the size limit.
BOM-prefixed, misplaced and repeated hashbangs retain original parser errors.

The former `string-annotation`, `return-annotation`, `local-type` and
`declarators-later-annotation` rejections are paired full-source programs in
[`type-annotations.mjs`](../tests/compatibility/type-annotations.mjs). Annotations
remain on the original AST as unvalidated metadata; they cannot admit unsupported
runtime values. That annotation-only unit retired no reference gap.

The former `gap-type-alias`, `annotation-alias-remains` and
`annotation-interface-remains` cases are paired full-source programs in
[`type-declarations.mjs`](../tests/compatibility/type-declarations.mjs). Explicit
aliases/interfaces and their direct named exports, plus default interface exports,
retain the original AST and emit no runtime code. Each charges one statement;
type members create no runtime cells or expression work. Declaration-level
`import type`/`export type` module items now also erase. Runtime imports/exports,
inline type specifiers, ambient values, enums, namespaces and augmentations remain
separate gaps. The four ordinary TS module-item promotions transfer no curated
reference; the type-module group remains open. See [the bounded contract](compiler.md#type-only-module-items).

Direct top-level numeric helper calls, including self/mutual recursion, are covered
in [`numeric-calls.mjs`](../tests/compatibility/numeric-calls.mjs). They promote no
existing rejection. `gap-call` still needs an arrow function value/capture;
`gap-nested-function` and `gap-recursion` still contain nested declarations.
That call-only unit retained all 41 reference sources, oracles and diagnostics.

Number `%` and `%=` now promote the exact `gap-remainder`, ordinary `remainder`,
three unary/bitwise/shift remainder compositions, eight `ts-rc` adapter cases and
three compound-assignment cases in [`numeric-remainder.mjs`](../tests/compatibility/numeric-remainder.mjs)
and the TypeScript adapter. All 16 original IDs/source bytes remain traceable; the
curated remainder reference retains its original oracle, arguments and expected
result. The empty remainder classification group and its 16 records are removed,
leaving 40 curated references and 666 classified negatives. The two enumerated
operator diagnostics truthfully add `%`/`%=`; other retained diagnostic text and
source positions remain unchanged. Coercion, BigInt and unsupported write targets
remain separate requirements.

The subsequent pristine numeric Math.abs intrinsic promotes only `gap-math-call`
and the identical ordinary `call` source. Its original source/oracle and provenance
are retained in `numeric-math-abs.mjs`; the other 39 references are unchanged.
Mutable/external realm builtins and module imports remain open requirements.

Fourteen ordinary wrapped-target rejections are now admitted with exact original TS
sources and hand-paired erased JS oracles in
[`numeric-ts-write-targets.mjs`](../tests/compatibility/numeric-ts-write-targets.mjs).
No curated reference is promoted. The remaining 685 negatives retain exact source,
mode and diagnostics; 39 explicit gap oracles remain. The wrapped-target group is
removed without weakening inventory or report validation.

The pristine sqrt/floor/ceil/trunc family promotes only the original ordinary
`math-abs-other-method` floor program, preserving its ID/source/mode. No curated
reference is removed: all 39 remain. Function-call and mutable-intrinsic groups
still describe unsupported callees, coercion, mutation and realm ownership.

Only numeric `~` now bridges known Boolean-producing operand shapes to Number:
literals, logical negation and admitted numeric comparisons/equalities through
runtime-transparent wrappers. The four original `bit-not-boolean`,
`bit-not-comparison`, `bit-not-negation` and `bit-not-asserted-boolean` cases retain
IDs/source bytes in the admitted corpus. The Boolean-values group remains open
with 82 records at that step; no curated reference is promoted. Other Boolean value positions,
root Boolean logical/conditional operands and general coercion remain separate work.

Discarded Number/Boolean evaluation promotes exactly 13 original ordinary fixtures,
retaining their source bytes, IDs and strict Module dialect. It covers statement,
comma-prefix and expression-form for-header positions, including mixed selected
results. No curated reference or inventory group is removed. Mutable Boolean storage,
coercive arguments/results and unsupported source in skipped branches remain open.

## Boolean const frontier

Immutable Boolean predicate locals promote exactly `gap-boolean-value`,
`boolean-literal-local`, `not-local` and `equality-numeric-local`. The former curated
reference retains its original oracle, values/L4 metadata, inputs `[4,2]` and +0
result in the admitted fixtures and direct contract test. The frontier now has 38
references. Mutable Boolean storage and other Number uses remain open;
the separate discarded-leaf boundary is described below. [The bounded contract](compiler.md#immutable-boolean-predicate-locals)
spells out the initializer and predicate-position restrictions.

## Primitive arithmetic frontier

The explicit inline Boolean bridges now cover binary `+`, `-`, `*`, `/` and `%`.
Six original ordinary sources graduate; no curated reference changes. Other bare Boolean
binding positions, other operator families and general coercion remain separate
work. [The source contract](compiler.md#inline-boolean-arithmetic-operands) preserves
primitive conversion, snapshots, skipped-branch admission and original depth.

## Unary Predicate binding frontier

Unary `+`, `-` and `~` now accept initialized immutable Predicate bindings using
semantic identity and original wrapper depth. The three original ordinary
`const-boolean-complement`, `const-boolean-plus` and
`boolean-numeric-unary-reject-identifier` IDs retain their source bytes; no curated
reference changes. The later arithmetic step explicitly shares this eligibility
only with five operand callers. Mutable storage, direct Boolean results/arguments,
root Boolean joins and other bare-binding binary operands remain separate requirements.

## Arithmetic Predicate binding frontier

Initialized immutable Predicate bindings now enter binary `+`, `-`, `*`, `/` and
`%` through the same bounded semantic eligibility used by unary operators. Four
ordinary IDs retain original source identities, including three duplicate-source
observations. All 38 curated references remain. The Boolean group stays open for
other operator/value positions, mutable storage and general coercion; discarded
identifier leaves are separately covered below. This is not
closure of that group or the separately deferred frontend-liveness work.

## Discarded Predicate binding frontier

Five ordinary sources now admit immutable Predicate identifier reads in discarded
contexts, retaining IDs and three distinct original JS sources. The Boolean group
remains open; none of the 38 curated references transfers. Root join classification,
mutable bindings, Boolean results/arguments and assignment-value policy are unchanged.
The exact native-liveness incident on deferred bitwise/shift work remains deferred;
this independent selector is not a repair or a new attempt budget.

## Strict Boolean equality frontier

Exactly `boolean-equality-operands` and `not-equality-operand` graduate unchanged
into the paired strict-Boolean corpus. The existing bounded eligibility method must
prove both operands of `===` or `!==`; this includes initialized immutable
Predicate bindings and selected inline shapes, preserving original depth/effects.
No curated reference moves. The other 903 rejection observations remain the baseline.
The Boolean-values assessment explicitly records this extension; loose or mixed-kind
equality, Boolean relational operands, general storage and ABI values remain open.

## Numeric logical-assignment frontier

Exactly `unsupported-compound-8` (`&&=`) and `unsupported-compound-9` (`||=`) move
unchanged into paired Number support. No curated reference moves. Initialized
mutable Number identifiers capture their target once, evaluate/store only the
selected Number RHS and return the selected or old snapshot. The logical-assignment
group remains open: `??=`, const assignment admission and general value storage
are separate requirements. Const early-skip sources remain valid JavaScript outside
this profile. Retained `unsupported-compound-1`/`10` source/mode/span/classification
stay exact; only their operator-whitelist diagnostic text advances.
