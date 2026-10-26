# ADR 0016: Share function selection with the typed evaluator

Date: 2026-10-26

Status: accepted bounded refactor; extension and broader migration evidence pending.

## Context

At `9465d19`, ordinary Number/Predicate/General evaluation and function-value
evaluation have separate worklists and child-region stacks. Both implement
conditional selection and comma-prefix sequencing. Function conditions and
discarded prefixes cross into the ordinary evaluator, carrying aggregate region
context manually. The IR already supports a typed Closure join, so a second
selection algorithm is not required by the runtime representation.

## Decision

Add a Function(signature) request/result kind to the existing evaluator. Use one
selection scheduler, pending-work loop and child-region continuation machinery
for conditional and comma expressions. Retain function signature queries and
constructor/call leaves in `function_values.rs`; they are not Number conversion
or General boxing. A function assignment stores the RHS snapshot and retains it
as its result. Branches still yield a verified logical Closure signature.

Keep the existing active function-value/callback/result limits and aggregate
region guard. The common cursor accounts for its own pending child regions;
crossings into function-result calls retain the prior explicit context transfer.
The constructor leaf's existing checking context is preserved. This refactor
does not claim all source classification or function-body checking is iterative.

## Alternatives and boundaries

- Keeping two worklists preserves current behavior but repeats region ownership
  and selection changes for future value kinds.
- A generic callback-driven traversal framework would add a new abstraction
  where the existing typed worklist already supplies the required control.
- A new CFG/SSA pipeline would expand the experiment beyond this concrete
  duplication; no current source behavior requires it here.

No new source expression, function-valued logical result, IR operation, backend
layout, calling convention, dependency or TypeScript type trust is introduced.
Fact queries remain effect-free; evaluating a constructor or call retains its
effects. General numeric coercion and primitive/function transport boundaries
remain separate. [ADR 0014](0014-backend-value-layout.md) owns the physical step;
neither step by itself establishes the broader architectural hypothesis.

## Verification and consequences

The full documented check passed after rebasing onto the README badge and again
on the subsequent CI-concurrency revision `57dc490`: 485 native tests plus one
doctest, 76 Node contracts, 4,466,748 numeric comparisons, primitive/JSON/heap
checks, formatting and strict Clippy. All 3,562 numeric variants retained exact
source/WAT/Wasm bytes, all 675 primitive artifacts matched, and all 919 original
rejected sources/hashes/diagnostics matched. Another 26 JS/TS examples retained
exact WAT/Wasm, including callbacks, fresh factories, throws/catches and async
jobs. Existing oracles execute these source paths in Node-Wasm and Wasmtime,
including the existing small-stack and aggregate-region boundaries.

Swapping consequent and alternate in the shared selection scheduler deliberately
broke the existing function-selection oracle (Number two instead of four).
Restoring the original file made it pass again; restored strict workspace Clippy
and formatting passed. Function-valued logical results still failed admission in
that refactor (`7c711e9`); the follow-on extension below is a separate change.

A release compiler-only comparison covers the combined layout/cursor changes
against `f278074`, using identical helper source and 13 fixed programs. Seven
alternating pairs use the same old/new iteration count per source, calibrated
to about 200 ms from the old median. Median compile-time ratios are 0.956–1.104;
kernel peak-working-set ratios are 0.985–1.016. All pass the unchanged 1.20
thresholds. Helper ready-time observations also pass the predeclared bound
`new <= max(old * 1.5, old + 1 ms)`; that includes process launch/source loading,
not Wasm instantiation. Helper executables are 2,942,976/2,935,296 bytes.

An earlier 128-iteration screen flagged 1.312 on a tiny primitive join, but its
batches lasted only about five milliseconds with overlapping samples. The longer
screen reports 1.048 for that case without any production change. Retain the
original failed observation; this is bounded, variable workstation evidence,
not a speedup claim or an isolated attribution to this refactor. Generated Wasm
identity protects the compared output footprint; new features still need their
own execution/cost evidence before broad migration.

Upstream `7a96a16` then added isolated checker research and proposed ADR 0015.
This unpublished note was renumbered to 0016; production and canonical-test blobs
were unchanged. The canonical check, artifact/diagnostic parity and calibrated
release-cost screen were rerun successfully on that base. The checker experiment
was not executed or adopted here, and no research dependencies were fetched.

The source change removes a separate function Work/Regions implementation and
shares actual evaluation control, rather than adapting an old IR to a new shell.
It does not make arbitrary new value kinds automatic or prove lower compiler
cost. The preselected function-valued logical extension is a separate unit below
that tests whether this boundary makes feature work more local. See the
[course-correction evidence](../ir-course-correction.md) for the remaining gates.

## Follow-on feature

The next bounded change implements same-signature function-valued `&&` and `||`.
It extends the effect-free signature query, routes logical selection through the
shared scheduler, and supplies function truthiness to the existing logical
continuations. Callback-origin queries follow both operands conservatively so
higher-order relays retain their checking types. Storage planning also needs the
same traversal in its closure hint; without it, a loop assigning a numeric call
result to an accumulator rejects the new selection as General storage. That
hint does not replace actual signature/call validation. Actual operand evaluation
remains ordered and conditional. There is no new IR operation, layout, emitter rule,
runtime ABI or dependency.

Maintained JS/TS oracles cover selected code/captures, once-only effects, snapshots,
returned/higher-order functions, primitive result tags, erased type claims and
async fulfillment/rejection job counts. Mixed signatures and primitive joins,
host function transport and function-valued coalescing remain rejected. A 2 MiB
stack check admits bounded chains and rejects over-depth chains. Inverting the
new function truthiness branch makes the selection oracle return four instead
of two; restoring exact source bytes makes the full closure/async suites pass.
This is evidence for incremental extension of the existing structured IR, not
approval of a broader architecture migration.

A following feature also admits same-signature function-valued `??`: represented
functions are non-nullish, so the existing function selection can retain the left
snapshot and skip RHS effects/jobs. Signature, callback-origin and storage queries
now traverse coalescing operands. Mixed-kind joins and primitive coalescing remain
separate; a constructor-free selector's default numeric-formal check still rejects
`x??y`. This does not infer function kinds from TypeScript claims or introduce new
IR, backend, runtime or dependency machinery.

The function-only slice passes the full check (504 native tests, two doctests,
76 Node contracts, 4,466,748 numeric comparisons and the primitive/JSON suites).
A frozen pre-change CLI reproduces identical WAT/Wasm for 3,562 numeric and 146
primitive variants, plus all 919 original rejection sources and diagnostics.
Sixteen JS/TS numeric/primitive/async coalescing variants also have identical
WAT/Wasm to their already supported function-valued `||` equivalents. This
supports unchanged execution cost for those artifacts, without a new compiler
or engine timing claim. A coalescing-branch mutation fails the new source oracle;
restored code passes the complete closure and async suites.
