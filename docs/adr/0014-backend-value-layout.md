# ADR 0014: Share backend value layout across storage and calls

Date: 2026-10-26

Status: accepted for this bounded backend policy; broader IR strategy remains experimental.

## Context

At baseline `f278074`, semantic `Type::width()` is used only by emitters, while
normal locals, coroutine globals, result signatures, captured cells and closure
argument frames each choose physical encodings separately. The duplication makes
an encoding change require coordinated edits even though source semantics do not
change. Moving the width function alone would preserve that problem.

The existing structured IR already has typed snapshots, mutable source cells,
ordered operations, lexical regions and a verifier. Preserve it rather than
inventing a second graph merely to move representation decisions.

## Decision

Use a backend-private value layout mapping from verified logical types to ordered
Wasm lanes and per-lane memory offsets. Reuse that policy for ordinary locals,
async globals/copies, internal results, captured-cell access and closure argument
loads/stores. Remove physical width from the semantic Type API. Keep source type
classification, logical signature checks, initialization and evaluation order in
their existing frontend/verifier layers.

This first mapping retains the existing encodings exactly: f64 Number, i32
Predicate, tag/f64 General and code/environment i32 Closure pairs. A 16-byte
captured-cell allocation and the original eight-byte Number/function closure
argument entries remain distinct storage contracts. They are not inferred solely
from lane count. The original layout refactor did not admit General arguments;
the later typed-IR extension below does so explicitly.

## Alternatives and limits

- Moving only `width`: too small to remove duplicated lane/offset decisions.
- A target-independent representation framework or a second lowered graph:
  unnecessary for these four encodings and one current target; defer until needed.
- Replacing structured regions with a block-argument CFG: may help later analysis,
  but does not itself settle value representation and requires Wasm restructuring.
  Keep the existing async-only graph and test structured evolution first.

No ABI, source feature, budget, dependency, optimizer, garbage collector, host
capability or Promise scheduling change is included. Heap String/JSON syntax
still bypasses this IR; the proposed lifetime/runtime/async ADRs 0011–0013 retain
their own status. Source operations such as ToBoolean and the async adapter's
primitive result protocol still have type-dependent semantic code; those are not
duplicate physical layout policies to erase mechanically.

## Verification and consequences

After rebasing onto CI baseline `79f7731`, the full documented canonical check
passed: 485 native tests plus one doctest, 76 Node contracts, 4,466,748 numeric
comparisons, primitive/JSON/heap checks, formatting and strict Clippy. All 3,562
numeric source variants retained exact source/WAT/Wasm bytes; all 675 primitive
artifacts matched. All 919 rejected sources and diagnostics matched too. Another
26 JS/TS examples retained exact WAT and Wasm across numeric, primitive and async
profiles, including callback frames, fresh factories, throws, catches and jobs.
Existing behavioral oracles execute those features in Node-Wasm and Wasmtime.

Changing the Closure environment field offset from four to zero deliberately
broke an existing higher-order callback test with an out-of-bounds table trap.
Restoring the original file made that test pass again; restored strict workspace
Clippy passed. The shared policy is therefore exercised by behavior, rather than
only tests that repeat its table.

An exploratory dev-compiler screen used four warmups, 128 compilations per sample
and seven alternating old/new sample pairs for 13 sources. Median ratios ranged
from 0.888 to 1.060, within the predeclared 1.20 regression threshold. This is not
a release performance, peak-memory or cross-platform cost result. Exact Wasm
identity preserves generated footprint for the compared sources.

The expected benefit is one physical policy for those storage/call consumers.
This does not prove that frontend feature work is cheaper, arbitrary layouts are
supported, or compilation is faster. Code/environment roles and the host primitive
protocol remain explicit backend consumers of the current field ordering; changing
those roles would still require corresponding operation/adapter changes. The chosen later extension and cost gates
are recorded in the [evidence note](../ir-course-correction.md). Broad migration
requires that additional evidence; successful relocation alone is insufficient.

## General closure parameter follow-up

The typed IR now permits General parameters only on the private closure ABI.
Predicate parameters and General parameters on ordinary helpers or exported entries
remain invalid. Source parameter inference is a separate step: the existence of
this IR representation does not itself admit new JS/TS calls.

The backend's argument-entry policy is eight bytes for Number or code/environment
Closure values and sixteen bytes for General tag/f64 pairs. Ordered prefix offsets
and total frame size use that one policy for both caller stores and callee loads.
Captured cells remain sixteen bytes and frame allocation retains its sixteen-byte
alignment, 1 MiB arena and 1,000 logical parameter limit. Existing Number/function
frames retain their prior encodings. No new host interface or dependency is introduced.

Maintained executable IR tests cover mixed General/Number/General and
General/Closure/General entries, Number and General results, captured parameters,
all four primitive kinds and the 1,000/1,001 boundary. A fixed-eight-byte frame
produced valid Wasm with a wrong result (5 versus native 3); typed offsets fix it.
The fresh modules also execute through the existing Wasmtime evaluator. This is
bounded representation evidence, not frontend admission, a new compilation-cost
measurement or proof of arbitrary physical layouts.

The source lowerer now also uses this established General argument policy for
runtime closures and factories. Input facts are separate from mutable storage
facts: a Number input may still initialize a General local cell after entry.
Known inlined calls specialize their arguments independently of that private ABI.
Callers box Number values only when their callee's logical input is General;
function pairs remain separate. No layout, exported ABI, arena or default fuel
change was needed. Maintained source oracles cover the frontend admission apart
from the earlier IR representation evidence.

## Fulfilled primitive Promise follow-up

The internal `FulfilledPromise` type reuses the sixteen-byte General tag/f64
physical encoding for a nonescaping fulfilled completion snapshot. Explicit IR
construction and projection keep it logically distinct from General; source
aliases, captured cells and suspended storage use the shared layout policy.
Exported Promise parameters and results remain invalid. This permits stored fulfilled primitive
locals without prematurely choosing general Promise object identity, settlement,
ownership or scheduler layouts. A general Promise representation can replace
this virtual encoding when those observable operations are implemented. See
[the contract and evidence](../fulfilled-promise-locals.md).

Known synchronous local arrows can return the same logical type through inline
regions. This preserves the sixteen-byte snapshot without adding a physical
call ABI. Private runtime closure signatures now also permit this distinct logical
result kind through the established tag/f64 physical return pair. Indirect calls,
callback contracts and returned-function chains check the logical kind separately
from that encoding. Private closure signatures now also allow Promise parameters,
using sixteen-byte entries through the same caller-store/callee-load policy as
General parameters. Mixed Promise, General, Number and function entries retain
their logical types and ordered offsets; captured parameter cells retain the
existing sixteen-byte allocation policy. Maintained IR fixtures execute callback
forwarding, captures and 1,000 parameters. Source inference now connects represented
fulfilled Promise actuals to these private inputs through original semantic symbols
and constructors. Promise input facts are separate from General storage facts.
Known fulfilled constructors participate in argument dependency analysis instead
of remaining opaque. Actual argument modes and binders retain
the logical type, without changing the physical frame policy. This admits the
documented homogeneous fulfilled inputs, not arbitrary Promise objects. Exported
Promise parameters/results and ordinary-helper Promise parameters remain invalid.
The public ABI, arena limit and default fuel budget are unchanged.
