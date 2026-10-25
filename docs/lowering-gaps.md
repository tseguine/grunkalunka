# Known lowering gaps

The exact `gap-async-await` source now executes in the separate
[numeric async profile](async.md), including its Promise job boundary. It remains
a numeric-profile rejection; general Promise values and exception handlers are
still known requirements. Numeric structured control flow can now suspend.

This page and the 919 rejection fixtures below describe the **numeric profile**.
The separate [primitive-return profile](compiler.md#primitive-runtime-boundary)
supports undefined/null/Boolean/Number returns. Thirteen exact original numeric rejection
sources now execute in that profile; its CLI report records both outcomes. These
counts are profile-specific, not a total number of missing language features.
The value report has four selected rejected pairs, not an exhaustive inventory.

The numeric profile admits one exported two-number function with numeric locals and immutable Boolean predicate locals,
Number/Boolean expression statements, empty statements, assignments, branches, while/for/do loops,
TypeScript expression wrappers, unresolved `Infinity`/`NaN` reads, unlabelled loop break/continue and returns.
Source inspection has not established an admitted wrong-result bug. The gaps below are currently rejected language
features or valid program shapes outside that contract.

Selected primitive-Boolean operands of unary `+`, `-` and `~` explicitly produce
Numbers. In the numeric profile this does not admit general Boolean storage, Boolean ABI returns or other
primitive coercions; none of the 38 curated gaps is promoted by unary plus/minus.

The executable frontier contains 34 curated reference cases in
`tests/compatibility/lowering-gaps.mjs`. Each has an independent expected numeric
result in JavaScript and a required current compiler diagnostic. The source is
retained in the compatibility run alongside its hash and the separate oracle source.

| Area | Exact fixture IDs |
| --- | --- |
| Operators and coercive equality | `gap-power`, `gap-loose-equality-coercion`, `gap-nullish`, `gap-typeof`, `gap-void`, `gap-delete`, `gap-property-in`, `gap-instanceof` |
| General JavaScript values | `gap-string-coercion`, `gap-object-property`, `gap-array-value`, `gap-bigint` |
| Additional control flow | `gap-switch`, `gap-label` |
| Nested declarations and nested recursion | `gap-nested-function`, `gap-recursion` |
| Module loading | `gap-module-import` |
| Bindings and parameter forms | `gap-var`, `gap-var-hoisting`, `gap-uninitialized-let`, `gap-local-destructure`, `gap-destructuring-write`, `gap-defaults`, `gap-rest` |
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
covers every current negative, beyond these 36 numeric reference opt-ins. Its 56
groups classify 31 invalid sources, 144 resource restrictions and 746 open requirements
across all 921 records. Each pins the exact source, extension and strict Module mode;
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
equality operators also admit represented undefined/null/Boolean/Number operands.
`gap-loose-equality-coercion` retains its exact string operand, independent result
and oracle, and remains rejected. Other unrepresented kinds remain unsupported.

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
predicates and represented primitive truthiness, preserving short-circuit effects. Nullish
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
or change the reference result to match a lowering bug. The 36 remaining examples are a
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

Generic declaration type parameters now erase for otherwise admitted numeric entry
and helper functions. Three original ordinary TS exclusions promote; explicit generic
calls, namespaces, enums and ambient functions remain separate gaps. Constraints and
defaults remain unvalidated metadata, not runtime proofs. See [the contract](compiler.md#generic-declaration-metadata).

## Primitive strict-equality promotions

`gap-null` and `gap-undefined` now execute their exact original sources in both
profiles, retaining their fixed `(4,2) -> 4` references and explicit JS/TS source
files. Six ordinary rejection observations also transfer to admitted strict-equality
programs. Corpus 51 has 1484 pairs, 987 rejections and 36 curated references.
The inventory keeps every remaining original source hash and classification;
56 groups still describe 812 open requirements, 144 resource restrictions and
31 invalid sources. Counts describe fixtures rather than distinct missing features.

Strict equality handles the four represented primitive kinds, including mixed-kind
inequality and Number NaN/signed-zero behavior. Loose equality, General storage,
logical operands and coercion remain open. Earlier counts above record their
respective implementation steps rather than the current frontier.

## Immutable primitive binding increment

Corpus 52 admits `const-boolean-conditional-initializer` with its exact original
source and adds nine paired storage/use programs. Totals are 1494 pairs, 986 numeric
rejections and 36 curated references, with 811 open, 144 resource and 31 invalid
observations in 56 groups. The Value profile also executes the former `general-local`
rejection with its original JS/TS text; its selected rejection count is now seven.

General const storage and aliases do not imply coercion. Two original numeric
rejections now reach the explicit General-binding numeric-use guard rather than
failing at initialization; their source hashes remain unchanged. Mutable General
cells, numeric coercions, arguments and helper results stay open.

## General truthiness increment

Corpus 53 adds sixteen paired programs for represented primitive conditions,
negation, predicate short-circuiting, loops, aliases and erased TS assertions.
The numeric frontier remains 986 rejection fixtures and 36 curated references;
there are 1510 admitted pairs. No existing numeric negative is counted as promoted.
All 56 inventory groups remain, with 811 open, 144 resource and 31 invalid records.
The separate Value profile transfers the original `general-condition` sources to
execution, reaching 21 pairs and six selected rejections.

ToBoolean covers the four represented primitive kinds. Nullish coalescing,
numeric coercions, General parameter writes and heap kinds
remain open. A successful condition does not imply admission in numeric operands.

## General logical selection increment

Corpus 54 transfers four rejection observations of three exact original sources:
`const-boolean-discarded-logical`, `const-boolean-logical-initializer`,
`discarded-predicate-binding-reject-root-logical` and
`strict-boolean-equality-reject-root-logical`. Duplicate-source observations keep
both IDs. Sixteen additional pairs cover primitive selected kinds and effects.
Current totals are 1530 pairs, 982 numeric rejections and 36 curated references;
56 groups retain 807 open, 144 resource and 31 invalid observations.

`predicate-logical-numeric-local` retains its original source/hash and rejection;
its diagnostic now reaches the unsupported numeric read of a General binding.
The separate Value profile executes four more exact remaining numeric rejection
sources, reaching 25 pairs, six selected rejections and eight recorded numeric
origins. That profile-specific support does not remove those eight numeric records.
General-valued &&/|| now preserve the selected primitive kind; ??, numeric coercion,
General parameter writes, compound/update operations and General logical assignments remain open.

## Mutable local storage and plain-write increment

Corpus 55 promotes eight source-bound rejection observations concerning mutable
Boolean initializers, aliases, discarded reads, strict equality and loop-header
initialization. All original IDs and sources remain in admitted fixtures. Sixteen
new paired programs cover later Number-to-null writes, alias snapshots and
assignment results. Totals are 1554 pairs, 974 numeric rejections and 36 curated
references; 56 groups retain 799 open, 144 resource and 31 invalid observations.

`unary-predicate-binding-reject-mutable` and
`arithmetic-predicate-binding-reject-mutable` keep their original source hashes and
rejections; diagnostics now reach unsupported numeric reads of General cells.
The Value profile adds four mutable storage cases, reaching 29 pairs, six selected
rejections and eight exact remaining numeric origins. General parameter writes,
compound/update/logical assignment, uninitialized let, numeric coercion and ?? remain
separate requirements. Conservative storage planning does not imply flow-sensitive
narrowing or that a captured alias follows later writes.

## Primitive ToNumber arithmetic increment

Corpus 56 transfers nineteen exact original observations for nullish/Boolean
unary conversion, General operands and conditional/logical/sequence arithmetic.
All original IDs, sources and hashes remain in admitted fixtures. Sixteen additional
pairs cover cell writes, snapshots, effects, aliases, loops and TypeScript wrappers.
Totals are 1589 pairs and 955 numeric rejections: 780 open, 144 resource and 31
invalid observations in 56 groups. The 36 curated numeric references are unchanged.

Conversion covers the four represented primitive kinds at unary +/minus and binary
+, -, *, /, % operands. Relational/loose comparisons, other conversion positions,
General parameter writes, compound/update/logical assignments, implicit lexical
initialization, ?? and additional runtime kinds remain open. The frozen Boolean
bitwise/shift repair is not resumed by this increment.

The Value profile has 34 paired result programs, four selected rejection pairs,
1904 comparisons and eight exact numeric origins. Its former `coercion` sources
execute; the former unconditional-recursion `general-number-call` sources instead
compile and exhaust fuel as separately recorded controls. Their two termination
observations do not contribute result comparisons or language coverage claims.

## General source parameter increment

Corpus 57 adds sixteen numeric pairs: 1605 pairs, 955 rejection observations,
36 curated references and the same 780 open/144 resource/31 invalid classifications
in 56 groups. No numeric rejection transfers in this step. Seven diagnostics
advance past parameter writes to Number-only reads/returns, with original source
hashes and records unchanged. The Value profile executes their four distinct JS
sources and adds four parameter cases, reaching 42 pairs, four selected rejected
pairs, twelve numeric origins and 2352 result comparisons. Duplicate observations
are not counted as distinct language features.

General parameter writes use local source cells initialized from Number ABI
parameters. General call arguments, defaults/rest/destructuring, compound/update/
logical writes and discarded sequences ending in General remain open. A large
flat dependency-chain witness uses a trailing scalar zero; it demonstrates storage
planning and Wasm validation, not support for that discarded-General sequence or
execution within the normal fuel allowance.

## Discarded General primitive increment

Corpus 58 transfers `discarded-null`, `discarded-undefined` and the original
root-conditional/root-sequence Predicate-binding discard observations to execution.
All four IDs and exact original JS sources remain in admitted fixtures,
paired with TypeScript companions.
Sixteen other pairs cover effects, loops, snapshots and type wrappers. Totals are
1625 pairs, 951 numeric rejections and 36 curated references: 776 open, 144 resource
and 31 invalid observations across 56 groups. No retained diagnostic advances.

General-valued discarded sequences and conditional/call results now work under
the existing source and signature contracts, resolving the gap observed during
parameter-cell work. The exact formerly failing 1000-parameter chain compiles and
validates without trailing scalar zero; its execution is not counted. Value
reporting reaches 46 pairs/four selected rejections/twelve numeric origins and
2576 comparisons, with prior controls and termination-only observations unchanged.
Other operators, write families, General arguments, implicit lexical initialization
and unrepresented kinds remain separate requirements.

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

Initialized mutable General locals and selected source parameters now support
prefix/postfix `++`/`--` over the four represented primitive kinds. Postfix returns
the converted old Number; both forms store the new Number. See
[the compiler contract](compiler.md#primitive-numeric-updates). Corpus 60 adds
20 pairs (1680 total); the existing 932 rejection records and all 36 curated
references remain unchanged. Value reporting reaches 54 pairs/3024 comparisons.
This expands behavior without claiming an existing inventory gap was closed.

## Implicit let initialization

Bare let identifiers now initialize intrinsic Undefined at declaration execution;
source order, shadowed names, TDZ and loop reinitialization retain their semantics.
See [the compiler contract](compiler.md#implicit-let-initialization). Corpus 61
has 1700 pairs and 928 rejection records. Four exact original JS/TS negatives are
now admitted. The curated `gap-uninitialized-let` retains its exact source and
numeric rejection, advancing to the General-cell return restriction; the same
original executes in the Value profile as a thirteenth numeric origin. Value
reporting reaches 59 pairs/3304 comparisons. Definite assertions, var, destructuring
and numeric General-cell returns remain separate requirements.

## Primitive arithmetic compound assignments

The five arithmetic compound operators now capture the old left value before RHS
effects, apply represented primitive numeric conversion and store/return a Number.
Number and General mutable identifier targets are supported. See
[the compiler contract](compiler.md#primitive-arithmetic-compound-assignments).
Corpus 62 has 1721 pairs/923 rejections: five exact observations promoted and no
retained diagnostic changes. All 36 curated references remain. Value reporting
reaches 63 pairs/3528 comparisons with existing numeric origins and controls intact.
Other operator families, targets and unrepresented kinds remain separate work.

## Primitive relational comparisons

The four relational operators now admit represented Undefined/Null/Boolean/Number
operands with numeric conversion, NaN-false results and left-to-right snapshots.
See [the contract](compiler.md#primitive-relational-comparisons). Corpus 63 has
1739 pairs/921 rejections (two exact promotions), with all 36 curated references
and remaining diagnostics unchanged. Value reporting reaches 67 pairs/3752
comparisons. String, BigInt, Symbol, object and observable conversion semantics
remain separate requirements; equality semantics are unchanged.

## General logical assignment

`&&=`/`||=` now select, store and return represented primitive values with exact
old-value capture and skipped RHS effects. Number-initialized locals/parameters
participate through existing General storage planning. See
[the contract](compiler.md#general-logical-assignment). Corpus 64 adds twenty
pairs (1759 total) while retaining 921 numeric rejections and 36 curated references.
Eight exact diagnostics advance to the separate numeric General-result restriction;
none is a numeric promotion. Two original Boolean-RHS sources execute unchanged
in the Value profile, which reaches 73 pairs/4088 comparisons and fifteen numeric
origins. `??`, `??=`, const assignment admission, other targets and unrepresented
kinds remain separate requirements.


Local non-escaping concise arrows now admit the exact original `gap-call` and
`gap-closure-capture` programs. Those examples are retained in `local-arrows.mjs`,
including source hashes. Escaping and recursive closures, later-initialized
captures and async arrows remain known work despite these two promotions. Numeric
block-arrow bodies now have scoped early returns, statements and loops, tested in
both synchronous and async callers; this does not add escaping function values.
See [the bounded contract](compiler.md#local-arrow-calls-and-shared-captures).
