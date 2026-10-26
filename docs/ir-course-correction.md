# IR course correction: bounded evidence

Date: 2026-10-26. Source baseline: `f278074beea812fa61151f3b366a6fec0e839611`.
The subsequent `79f7731` adds CI/PR configuration and documentation only.

## Baseline and contracts

Both previously in-flight closure units are included: selected concise factories
return numeric-leaf functions; represented synchronous functions are truthy while
their bodies remain dormant. These are bounded features, not general functions.

`compile` takes two Numbers and returns a Number. `compile_value` preserves
undefined/null/Boolean/Number results with an `(i32, f64)` host pair.
`compile_async` has the same primitive kinds and a start/resume protocol, at most
32 await sites, lexical catches and source throws. Synchronous lifted closures
can survive jobs; arbitrary Promise objects and async closure bodies remain gaps.
`compile_heap_value` admits the separate String-literal/JSON-call expression path,
returning a checked String root. General source objects, arrays and arbitrary JSON
value returns are still absent; the guest JSON operations alone do not close them.

The original annotated Oxc AST and semantic identities survive frontend checking.
Annotations are untrusted metadata. Existing source, expansion, frame/arena and
execution limits remain admission/resource contracts. Unsupported source, invalid
internal IR, Wasm/host resource failure and JavaScript throw are distinct outcomes;
this work does not turn traps into catchable JavaScript exceptions.

Logical Number, Predicate, General and Closure values are snapshots; mutable
bindings are cells. The backend currently uses f64, i32, `(i32,f64)` and `(i32,i32)`
respectively. Captured cells occupy 16-byte arena allocations; closure arguments
have eight-byte entries; calls pass environment and argument-frame pointers.
The private table call returns f64 or two i32 fields. Ordinary activations use
locals; the async entry keeps snapshots/cells in instance globals and dispatches
continuations through `br_table`. This compiled state machine is not a universal
instruction interpreter. Required emitted features include multi-value results,
indirect calls/function tables, mutable globals and saturating float conversions;
heap/capture paths additionally use bounded linear memory.

The baseline `node scripts/check.mjs` passed: 485 native tests plus one doctest,
76 Node contract tests, corpus 65 with 1,781 admitted JS/TS pairs and 4,466,748
numeric comparisons, 919 expected rejections, primitive-profile comparisons,
JSON guest/composition/heap checks, formatting and strict Clippy. A separate audit
matched all 919 original source texts, hashes and exact diagnostics. This is
finite regression evidence, not a percentage of language completeness.

Baseline discovery found a real CLI stack overflow on the admitted 64-nested-for
fixture. It was reproduced on published `087700f`, localized to verification and
fixed in `8a845b5` with an explicit region-verification stack. The composed
64-statement/128-expression case also passes. Parsing, other classification,
emission and destruction still have recursive paths. Raising limits or host
stacks was not used as a repair or evidence for a new IR.

## Concrete coupling and first choice

| Surface | Observation | Bounded response |
| --- | --- | --- |
| `ir::Type::width` | Its callers were already backend-only, but emitters independently chose lanes and memory offsets. | Share one backend layout policy across locals, coroutine globals, cells and closure frames. Moving width alone would not suffice. |
| `lower/evaluate.rs`, `lower/function_values.rs` | Ordinary contexts share a cursor; function values duplicate conditional/sequence traversal and cross its region-budget boundary. | Later test a shared selection cursor; preserve query versus evaluation and source effects. |
| Closure signature IDs | Numeric-leaf IDs equal arity; richer signatures use an interned range. | Keep working logical checks; the encoding alone is not a correctness defect. |
| `heap.rs` | String/JSON syntax checks directly produce WAT outside the typed IR. | Future real-source heap integration; no claim that this layout step fixes it. |
| `emit/asynchronous.rs` | Splitting regions and planning physical globals occur together. | Share value layouts now; retain the working continuation transformation. |

Evolve the current structured IR first. It already represents ordered effects,
snapshots, mutable cells, typed joins, calls, catches and suspension. A block-argument
CFG could make arbitrary joins and dataflow easier but would introduce a Wasm
structuring problem before those capabilities are needed. The existing limited
hybrid already forms an async continuation graph after semantic verification;
retain it and reconsider a broader CFG when a concrete feature demands one.

The first candidate changes backend ownership only. It is not a new IR graph or
a generic layout framework. Its protection is existing behavioral oracles plus
same-source artifact comparisons. On the final CI base `79f7731`, the full check
passed again. All 3,562 numeric variants retained source/WAT/Wasm identity; 675
primitive artifacts matched, as did 919 rejection sources/diagnostics and 26
additional numeric/primitive/async JS/TS examples. A deliberately corrupted
Closure field offset broke the existing callback oracle; restoring it passed.
This supports keeping the bounded backend policy in [ADR 0014](adr/0014-backend-value-layout.md).
It does not establish a better frontend or select a broad migration.

The next useful semantic extension is chosen
in advance: same-signature function-valued `&&`/`||`, including ordered effects and
skipped async jobs. It must exercise a real frontend path, not only an old-IR
adapter. No broad migration decision follows merely from this refactor passing.

## Cost evidence and remaining gate

Existing paired microbenchmarks were reproduced on Windows x64, Ryzen 9 7940HS,
Node 20.10/V8 11.3. Seven alternating batches after warmup showed Wasm/JS median
ratios about 1.15 scalar, 1.10 loop, 5.95 remainder/branch, 0.98 selected closure
and 0.71 captured-cell loop. These include the export boundary; compilation used
the dev frontend, startup samples were single observations and concurrent builds
were active. They establish neither release compiler cost nor Wasmtime speed.
A separately checked counter rewrite reduced the remainder-heavy gap; this
supports a helper-cost hypothesis without attributing it to the IR architecture.

Identical generated Wasm supports preserving execution behavior, footprint and
runtime cost for those exact artifacts; it does not establish compiler cost or a
benefit for new features. Before broad adoption, measure old/new compilation and
execution within each engine, startup/boundary costs and memory with declared
configurations and tolerances; implement the chosen extension and assess whether
its changes stay localized. Do not silently replace that evidence with Node-only
ratios or larger fuel/stack budgets. See [ADR 0014](adr/0014-backend-value-layout.md).

The backend-only candidate also passed a small warmed dev-compiler screen: four
warmups, 128 compilations per sample, seven alternating old/new pairs, 13 sources;
median new/old ratios 0.888–1.060 against a predeclared 1.20 regression threshold.
Process startup is excluded. Release compilation cost, peak memory and the cost
of a new semantic extension remain unmeasured. Runtime benchmarks were not rerun
as evidence of a speedup from byte-identical Wasm.

## Shared selection pilot

The next bounded candidate adds Function(signature) to the existing evaluator
and removes the separate function-value Work/Regions implementation. Conditional
and comma selection use the same scheduler and continuations as other value
kinds; signature facts and constructor/call leaves remain distinct. Full checks
passed on the badge revision and again on CI revision `57dc490`. Numeric variants,
primitive artifacts, 919 rejected sources/diagnostics and the 26 focused examples
retain the same artifact identities as above. A branch-swap mutation broke the
existing function-selection oracle and restored code passed it again.

The combined two steps passed the calibrated release-compiler screen against
`f278074`: 13 sources, seven alternating pairs at a common old/new count targeting
200 ms from the old median; compile-time ratios 0.956–1.104, peak-working-set ratios
0.985–1.016 against unchanged 1.20 thresholds. The original five-millisecond
primitive-join screen flagged 1.312; longer batches report 1.048 without production
changes. Both observations are retained, not a claim of universally unchanged
compilation cost. [ADR 0016](adr/0016-shared-function-selection-cursor.md) records
the method and limits. Function-valued logical results were the separate chosen
extension, implemented below rather than implicitly admitted by the refactor.

The same gates passed after upstream `7a96a16` added checker-hint research and
proposed ADR 0015. The cursor decision is now ADR 0016; this import does not adopt
a checker. Production/test blobs stayed unchanged and release-cost replay passed
the same thresholds. The isolated research suite was not run as part of this work.

## Function-valued logical extension

The chosen extension now handles same-signature function-valued `&&` and `||`
through the real frontend, shared evaluator, existing typed IR and Wasm emitters.
Changes stay in four lowerer files: signature queries, shared selection control,
callback-origin propagation and the storage planner's closure hint. No IR
operation, layout, emitter, runtime ABI or dependency changes. The planner update
was required by a failing numeric-accumulator loop test; sharing the evaluator
does not yet unify all function-fact queries.

Maintained JS/TS tests execute selected code/captures, effects, aliases, callback
relays, returned functions, primitive tags and async jobs in Node-Wasm/Wasmtime
against native JS. Rejection checks retain complete-signature, primitive/function,
coercion, host-transport and coalescing boundaries. A deliberately false function
truthiness branch fails the selection oracle; restored source passes. Existing
small-stack checks and additional logical-chain limits remain in place.

For new-feature execution cost, four explicit-selection/logical source pairs cover
simple `&&`, `||`, a 16-call loop and a higher-order callback. Six correctness
vectors precede timing; 100 warmups, adaptive common batches of at least 50 ms and
seven alternating pairs include the export boundary. Against the old compiler's
conditional **runtime-function** selections, new/old medians are 0.925–1.176 in
Node 20.10/V8 11.3 and 0.874–1.086 in Wasmtime 50 on Windows x64. All pass the
predeclared 1.20 screen. Wasm byte pairs are 581/555, 581/555, 728/706 and 746/720.
Wasmtime uses the existing dev-built host library with fuel renewal and an
optimized measurement helper; no release Wasmtime performance claim follows.
Startup observations distinguish Node instantiation of an already compiled module
from Wasmtime Engine/Module/Instance creation; they are not comparable engine costs.

The first comparison instead used simple aliases for three baselines. Those are
inlined by the old compiler, while logical values currently use runtime closure
calls. Node ratios were 1.48–1.63, outside the same 1.20 screen; the loop already
used a runtime conditional and passed. Preserve this failed observation. Matching
the runtime-function path makes the second comparison useful, but does not remove
the missed specialization or establish universally equal cost. Both comparisons
recorded exact source pairs and samples during review; their configurations,
results and limits are preserved here while disposable runs can be cleaned up.
Rechecking the recorded standalone static-alias accumulator example on `d8419a7`
accepts all six alias/conditional/logical and `a`/zero-initializer variants. The
earlier benchmark diagnostic is not evidence that this recorded source is a current
lowering gap; no speculative storage repair was made from that stale entry.

The result supports keeping the bounded backend layout and shared evaluator, then
extending the current structured IR as concrete features demand. It does not
justify a new CFG, universal value dispatcher or wholesale migration. Function
fact-query duplication and avoidable runtime materialization are concrete future
improvements, rather than prerequisites for further PoC feature work.

The final extension also passes the unchanged 13-source calibrated release-compiler
screen against `f278074`: compile-time ratios 0.951–1.070, kernel peak-working-set
ratios 0.989–1.021, and the same helper-ready-time threshold. This remains bounded
compiler evidence, not a universal compilation or runtime speed claim.

The next small extension admits same-signature function-valued `??` in the same
four lowering files, retaining the left pair and skipping RHS effects/jobs. Its
original rejected source becomes a differential JS/TS positive. At that stage,
generic constructor-free selectors encountered the missing primitive coalescing
operation during numeric-placeholder body checking. No new IR operation, emitter, runtime
ABI, dependency or broad architecture decision is needed for the function-only slice.

The following primitive binary `??` unit adds one typed `GeneralIsNullish` operation
and a backend tag test. Existing scalar Number and function operands are non-nullish;
General selection tests the kind, not truthiness. Predicate contexts select the
actual primitive before converting it to Boolean. The shared selection cursor,
structured regions, async splitting, layouts and ABI remain in place. The original
constructor-free closure selector now passes preliminary numeric checking and
specializes from actual function arguments. No broad IR migration is needed.
