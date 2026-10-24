# ADR 0002: Tagged primitive values at the runtime boundary

Date: 2026-10-24

Status: Accepted for the primitive runtime ABI and explicit compiler return profile; heap layout remains separate.

## Context

JavaScript value kinds must remain distinguishable when transported through typed
Wasm. The existing numeric compiler and runtime expose `(f64,f64)->f64`; that
specialized interface is useful and does not represent arbitrary JavaScript values.
We need a maintained typed host boundary for general-value lowering to target.

## Decision

Use an explicit tagged sum. The first runtime cases are `Undefined`, `Null`,
`Boolean(bool)` and `Number(f64)`. Strings, symbols, BigInts and objects are not
implemented by this unit. Proven numeric computations retain their numeric interface.

Add `ValueInstance` with two numeric arguments and a `(i32,f64)` result. Its wire
contract is:

| Tag | Value | Payload |
| --- | --- | --- |
| 0 | Undefined | positive zero |
| 1 | Null | positive zero |
| 2 | Boolean | positive zero for false, exactly one for true |
| 3 | Number | binary64 value, including signed zero, infinities and NaNs |

Reject unknown tags and noncanonical non-Number payloads; never silently reinterpret
an invalid result as another kind. Number NaNs remain Number values; this boundary
performs no NaN-box encoding or payload canonicalization. Tests compare NaNs by class
and non-NaN numbers by bits.

Both execution profiles use the same import-free instantiation and renewed fuel
budget. Keep stores independent. Add no ambient imports, dependencies, guest heap
allocation or collector. The compiler exposes `compile_value` for this ABI; the CLI
selects it explicitly with `compile-value`, `wat-value` and `eval-value`. The original
commands retain the numeric profile. Runtime-only fixtures establish transport
and validation; separate compiled-source tests establish bounded source support.

## Alternatives

| Option | Advantages | Disadvantages and disposition |
| --- | --- | --- |
| Explicit tag and binary64 payload | Simple typed host signature, disjoint value kinds, direct Number transport, no reserved NaN bit patterns or guest heap needed. | Two physical result values. Does not settle mixed helper parameters, heap references or general high-arity calls. Selected for this boundary. |
| One-word NaN-box encoding | Can reduce physical local/argument width for primitives. | Requires reserved patterns, complete Number ingress canonicalization and decoding; smaller local counts do not prove smaller or faster Wasm. Defer physical encoding optimization. |
| WasmGC structures/references | Supplies managed reference lifetime and a path to heap-backed values. | Requires runtime feature/profile decisions and Number boxing or another mixed-value layout. The current Wasmtime feature selection omits GC; the installed Node20.10 test runtime rejects GC struct types without its experimental flag. Defer heap choice, rather than require a collector for immediate primitives. |
| Keep only numeric results | Preserves the existing narrow interface with no additional code. | Cannot distinguish undefined, null, false and numeric zero. Retain as a specialized profile, not the general-value boundary. |

## Consequences and verification

The primitive API is independently usable by import-free Wasm producers. The compiler
return profile lowers real JS/TS through the verified IR and explicit evaluator:
`General` snapshots, scalar-to-General `Pack`, nullish constants and General-valued
conditional joins. Number/Predicate computations and source cells stay specialized.
Function signatures are supplied explicitly by the selected profile: entry results
are General and helper results Number. Annotation syntax supplies neither inference
nor an alternative runtime signature. Calls and returns are checked against that
profile before emission. General temporaries use paired physical locals; Number and
Predicate temporaries keep one local. Typed pair loads/stores preserve stack order.

Source tests compare original JS with Wasmtime and Node-Wasm, including mixed returns,
Number edge cases, side effects and direct calls. General source cells, helper formals,
coercions and implicit returns remain separate units; no heap choice follows from
this result ABI. The numeric profile keeps its original artifacts and diagnostics.

Verify four-kind identity, numeric edge cases, malformed tags/payloads, signature
separation, unused import denial, repeated-call fuel renewal and recovery after fuel
exhaustion. Keep every existing numeric source artifact and diagnostic unchanged.

Flattening General parameters into pairs could exceed embedding parameter limits;
this decision covers only the fixed two-Number-input result boundary. It does not
reduce the existing 1000-source-parameter policy or claim a general helper ABI.
