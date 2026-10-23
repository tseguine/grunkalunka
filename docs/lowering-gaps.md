# Known lowering gaps

The current compiler admits one exported two-number function with numeric locals,
numeric expression statements, assignments, branches, while/for/do loops,
TypeScript expression wrappers and returns.
Source inspection has not established an admitted wrong-result bug. The gaps below are currently rejected language
features or valid program shapes outside that contract.

The executable frontier contains 57 curated reference cases in
`tests/compatibility/lowering-gaps.mjs`. Each has an independent expected numeric
result in JavaScript and a required current compiler diagnostic. The source is
retained in the compatibility run alongside its hash and the separate oracle source.

| Area | Exact fixture IDs |
| --- | --- |
| Operators and coercive equality | `gap-remainder`, `gap-power`, `gap-bit-and`, `gap-bit-or`, `gap-bit-xor`, `gap-shift-left`, `gap-shift-right`, `gap-shift-unsigned`, `gap-loose-equality-coercion`, `gap-nullish`, `gap-bit-not`, `gap-typeof`, `gap-void`, `gap-delete`, `gap-property-in`, `gap-instanceof` |
| General JavaScript values | `gap-boolean-value`, `gap-string-coercion`, `gap-object-property`, `gap-array-value`, `gap-bigint`, `gap-null`, `gap-undefined` |
| Additional control flow | `gap-break`, `gap-continue`, `gap-switch`, `gap-label` |
| Functions, calls and recursion | `gap-call`, `gap-nested-function`, `gap-recursion` |
| Module loading | `gap-module-import` |
| Bindings and parameter forms | `gap-var`, `gap-var-hoisting`, `gap-uninitialized-let`, `gap-multiple-declarators`, `gap-local-destructure`, `gap-destructuring-write`, `gap-defaults`, `gap-rest` |
| Closures | `gap-closure-capture` |
| Exceptions and completion | `gap-catch`, `gap-finally-return` |
| Classes and instances | `gap-class-instance` |
| Generators and async | `gap-generator`, `gap-async-await` |
| TypeScript type-only declarations | `gap-type-alias` |
| Globals and standard library | `gap-infinity-global`, `gap-nan-global`, `gap-math-call` |
| PoC entry and source restrictions | `gap-export-name`, `gap-one-parameter`, `gap-extra-export`, `gap-module-directive`, `gap-function-directive`, `gap-hashbang` |
| Conservative return analysis | `gap-while-guaranteed-return`, `gap-unreachable-after-return` |

This separates missing operators/value semantics from functions/modules and later
closure, exception and async work. Export naming/arity, directives/hashbangs and
structural return rules are deliberate PoC admission restrictions, rather than
invalid JavaScript. Source-size, depth, statement/local counts, input-batch and execution-fuel
limits remain separate [resource bounds](compiler.md#experimental-contract).

The former `gap-as`, `gap-angle-assertion`, `gap-non-null` and `gap-satisfies`
references are now admitted as `ts-erasure-as`, `ts-erasure-angle`,
`ts-erasure-non-null` and `ts-erasure-satisfies`, with additional nested, effect,
predicate and boundary cases in
[`typescript-erasure.mjs`](../tests/compatibility/typescript-erasure.mjs).
These expression wrappers erase without adding type checking; type aliases and
TypeScript-wrapped assignment/update targets remain outside admission.

The former `gap-for` reference is now `for-promoted-reference` in
[`numeric-for.mjs`](../tests/compatibility/numeric-for.mjs), with additional scope,
ordering, omission and source-budget checks. `for-in`/`for-of`,
`break`/`continue` and labels remain unsupported.

The former `gap-do` reference is now `do-promoted-reference` in
[`numeric-do.mjs`](../tests/compatibility/numeric-do.mjs), with at-least-once,
condition-order, scope, return-coverage and source-budget checks.

The former numeric `gap-loose-inequality` reference is now
`inequality-promoted-reference` in
[`numeric-equality.mjs`](../tests/compatibility/numeric-equality.mjs). Both loose
equality operators admit numeric operands only. `gap-loose-equality-coercion`
retains its source, independent result and oracle, and now records rejection of
the string operand. Coercive and Boolean equality remain outside admission.

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

Boolean literals are admitted in predicate positions. Boolean locals/returns,
Boolean-valued ternary arms and Boolean equality operands remain outside admission.
Logical negation is admitted in predicate positions, including nested negation and
negated numeric comparisons; it does not add Boolean locals or numeric coercion.

## Next steps

Promote an implemented case deliberately into the admitted differential corpus,
then test its supported domain and boundary behavior against both Wasm engines.
Do not merely remove the diagnostic check, count an expected rejection as support,
or change the reference result to match a lowering bug. The 57 examples are a
concrete starting frontier, not an exhaustive ECMAScript conformance inventory.
Test262 and the other [external suites](testing.md) provide broader future coverage.

Type checking/type queries, a general JS-value ABI and the bridge between compiled
code and WIT capabilities remain separate roadmap work. These tests do not select
a value representation, GC strategy, final entry ABI or capability-profile defaults.
