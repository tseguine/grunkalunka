# Known lowering gaps

The current compiler admits one exported two-number function with numeric locals,
numeric expression statements, empty statements, assignments, branches, while/for/do loops,
TypeScript expression wrappers, unresolved `Infinity`/`NaN` reads, unlabelled loop break/continue and returns.
Source inspection has not established an admitted wrong-result bug. The gaps below are currently rejected language
features or valid program shapes outside that contract.

The executable frontier contains 39 curated reference cases in
`tests/compatibility/lowering-gaps.mjs`. Each has an independent expected numeric
result in JavaScript and a required current compiler diagnostic. The source is
retained in the compatibility run alongside its hash and the separate oracle source.

| Area | Exact fixture IDs |
| --- | --- |
| Operators and coercive equality | `gap-power`, `gap-loose-equality-coercion`, `gap-nullish`, `gap-typeof`, `gap-void`, `gap-delete`, `gap-property-in`, `gap-instanceof` |
| General JavaScript values | `gap-boolean-value`, `gap-string-coercion`, `gap-object-property`, `gap-array-value`, `gap-bigint`, `gap-null`, `gap-undefined` |
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
covers every current negative, beyond these 39 numeric reference opt-ins. Its 56
groups classify 26 invalid sources, 88 resource restrictions and 636 open requirements
across all 750 records. Each pins the exact source, extension and strict Module mode;
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
the string operand. Coercive and Boolean equality remain outside admission.

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
In numeric value positions, Boolean operands (including comparison results) remain
outside admission. In predicate positions logical operators may combine admitted
predicates and numeric truthiness, preserving short-circuit effects. Nullish
coalescing remains outside admission in both contexts.

Boolean literals and conditional arms are admitted in predicate positions.
Boolean locals/returns, Boolean-valued arms in numeric ternaries and Boolean
equality operands remain outside admission.
Logical negation is admitted in predicate positions, including nested negation and
negated numeric comparisons; it does not add Boolean locals or numeric coercion.

## Next steps

Promote an implemented case deliberately into the admitted differential corpus,
then test its supported domain and boundary behavior against both Wasm engines.
Do not merely remove the diagnostic check, count an expected rejection as support,
or change the reference result to match a lowering bug. The 39 examples are a
concrete starting frontier, not an exhaustive ECMAScript conformance inventory.
Test262 and the other [external suites](testing.md) provide broader future coverage.

Type checking/type queries, a general JS-value ABI and the bridge between compiled
code and WIT capabilities remain separate roadmap work. These tests do not select
a value representation, GC strategy, final entry ABI or capability-profile defaults.

The former `gap-bit-not` reference is now `bit-not-promoted-reference` in
[`numeric-bit-not.mjs`](../tests/compatibility/numeric-bit-not.mjs). Numeric unary
`~` truncates and wraps to signed 32-bit bits before complementing and returns a
Number. It does not admit Boolean, string or BigInt operands.

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
The curated gap count is unchanged. Remaining ordinary IDs 0, 1, 8, 9 and 10 retain
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
type members create no runtime cells or expression work. Imports, export lists,
ambient values, enums, namespaces and augmentations remain separate gaps.

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
with 82 records; no curated reference is promoted. Other Boolean value positions,
root Boolean logical/conditional operands and general coercion remain separate work.
