# ADR 0003: Write-aware local storage planning

Status: accepted for the 0.0.1 implementation.

## Context

Source locals can change primitive kind after initialization. An initializer-only
layout cannot represent `let x = 0; x = null;`, and aliases must capture the value
at their own initialization rather than track a later write. Existing Number-only
locals should retain their scalar IR and Wasm. TypeScript annotations do not prove
runtime kinds. Lexical visibility, TDZ, const writes and source budgets remain
separate admission checks.

## Decision

Run a readonly, flow-insensitive planning pass over original Oxc nodes and SymbolIds
before emitting function IR. Collect local initializers and plain `=` right-hand
sides. Begin with local Number candidates and monotonically remove candidates whose
initializer or plain writes cannot produce an admitted Number under the current
candidate set. Iterate to a fixed point so storage requirements propagate through
aliases. Force removed mutable locals to General; ordinary declaration lowering
still publishes each binding only after its initializer.

Reuse one bounded Number-result classifier with two symbol environments: the
planner's candidate facts and the lowerer's already-published cell types. A plain
assignment has its RHS kind. Arithmetic/update result classification is conditional
on actual source admission; the classifier never grants admission or performs a
conversion. Strings, objects and other unrepresented kinds still fail lowering.

Oxc's compiler builder does not retain AstNodes by default. Enable only its node
store for this pass, retaining syntax checks and avoiding linter/CFG features or a
new visitor dependency. This adds a node-index memory cost; the source-size bound,
ordinary local budget and measured representative process observations constrain
verification. Above the existing local limit the planner yields to ordinary
lowering instead of spending extra inference work on an oversized module.

General locals use the existing paired cells. Plain assignment stores a captured
General RHS and returns that snapshot. In a proven Number-result position, capture
a Number RHS, pack it only for the General store, and retain the Number snapshot as
the expression result. Conditions explicitly apply ToBoolean to General results.

## Alternatives

- **Initializer-only typing:** small and preserves scalar cases, but rejects later
  kind changes even when plain General stores exist. It does not satisfy mutable
  local storage support.
- **General storage for every local:** simple kind changes and fewer planning
  rules, but loses existing Number specialization and requires numeric conversions
  throughout previously supported arithmetic. It expands this unit into a broader
  migration and risks admission regressions.
- **Flow-sensitive analysis or SSA kind refinement:** can keep more scalar values,
  including a captured alias before a later kind change or Boolean-only mutable
  storage. It needs additional join,
  loop and write analysis; defer it as a Wasm-quality improvement rather than making
  broad primitive storage depend on it.
- **A second recursive AST visitor:** avoids retaining Oxc's node index but adds
  traversal machinery or a dependency and another recursion concern. Reuse Oxc's
  original node store and measure its cost first.

## Consequences and remaining work

The analysis is conservative: aliases of a General-layout source may use General
storage even when their captured payload is a Number. This does not create a live
alias or coerce a value. Uninitialized/cyclic references remain subject to normal
TDZ checks; candidate facts do not publish bindings.

This unit covers initialized local `let` bindings and plain `=`. Parameters keep
the Number ABI and source-cell boundary. General parameter writes, compound/update
operators and General logical assignments remain explicit follow-up units; they
must fail with source diagnostics rather than internal IR errors until supported.
Numeric ABI returns of General cells remain restricted. No heap, import, global
installation, dependency, source-budget or execution-fuel change is introduced.

Verification must exercise later Number-to-null/Boolean writes, alias snapshots,
assignment results, lexical scope and loop initialization, preserve all existing
admitted artifacts, and enumerate changed rejection observations honestly. Compare
representative process cost with the frozen compiler before making footprint claims.

## Bounded process observations

One Windows debug-CLI sample used six alternating old/new process pairs for each
of four existing-admitted sources. Every emitted WAT file matched exactly.
Observed peak working-set counter medians increased by 90, 118, 128 and 146 KiB for
the small, loop, selected large-let and 128-local alias cases. Elapsed medians were
mixed (old/new: 34.393/37.035, 36.127/29.262, 38.467/37.297 and 38.740/38.162 ms).
These startup-inclusive, 1 ms sampled observations include process/cache noise and
may miss a terminal peak. They cover the combined implementation, do not isolate
the node-index cost, and establish no general performance or portability claim.
Broader tuning remains evidence-driven; no source or execution budget was widened.
