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
