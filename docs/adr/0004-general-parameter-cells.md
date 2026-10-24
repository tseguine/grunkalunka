# ADR 0004: General source parameter cells with a Number ABI

Status: accepted for the 0.0.1 implementation.

## Context

A source parameter initially holds its incoming Number but can receive another
primitive kind through ordinary assignment. Physical parameter slots currently
implement a Number calling convention shared by callers, host signatures and the
IR verifier. A source binding and a physical signature slot need not be the same
storage cell. Incoming arguments remain Number-only in this increment.

## Decision

Extend the original SymbolId storage plan to include parameter plain writes.
Select a General source cell only when the parameter cannot remain Number under
its writes and conservative alias dependencies. Retain the original physical
Number parameter slot and load/pack its incoming value into a General root local
before evaluating the body. Redirect the original source SymbolId to that cell
after initialization. Parameters that remain Number get no extra operations or
cells, preserving their existing specialized artifacts.

These cells are generated parameter storage, not additional source declarations;
they do not consume the 128-source-local budget. The physical parameter count,
Number argument contract, 1000-parameter limit and 16 KiB source limit remain.
The normal General local initialization, visibility, owner and mutability rules
verify each cell. Captured aliases retain snapshots rather than follow writes.

The plan still computes the greatest Number candidate fixed point. Collect the
positive classifier's symbol dependencies once, record reverse edges, seed
non-Number initializers/writes and remove candidates through a worklist. An
unseeded cycle remains a Number candidate, subject to ordinary source validation;
a non-Number seed invalidates every dependent candidate. This avoids repeatedly
scanning the growing parameter set. Shape classification does not admit source
operands, publish lexical bindings or infer runtime kinds from type annotations.

## Alternatives

- **Change the parameter ABI to General:** directly represents dynamic arguments,
  but changes host/internal signatures and caller obligations. General arguments
  require a separate accepted contract; source mutation does not require that change.
- **Make physical parameter slots General without an ABI change:** superficially
  small, but violates the verifier and emitter's signature-layout invariants.
- **Allocate General cells for all parameters:** simplifies selection, but expands
  every existing function and loses Number-only specialization. Numeric reads and
  returns would require a broader migration.
- **Keep rejecting non-Number parameter writes:** preserves the old representation
  with no extra storage, but leaves ordinary primitive mutation unsupported.
- **Retain whole-plan fixed-point rescans:** familiar and adequate for 128 source
  locals, but parameter dependencies increase the repeated analysis surface. Reverse
  invalidation preserves the same conservative result with one propagation pass.

## Consequences and checks

A changed source parameter may need a paired cell plus entry load/packing work.
Conservative aliases may also use General layout. No flow-sensitive narrowing is
claimed. Default/rest/destructured parameters, General call arguments, compound,
update/logical writes and additional source kinds remain separate requirements.

Verification must cover entry and helper parameters, kind-changing writes,
parameter-to-parameter and parameter-to-local aliases, original-value snapshots,
shadowed names, loops and calls, signed zero/NaN, source-local and parameter limits,
long dependency chains, unchanged previous artifacts and explicit rejection accounting.

A large flat dependency-chain witness ends its discarded comma expression with a
scalar zero to isolate storage planning. A discarded sequence whose final value
is General remains a separately recorded lowering gap; successful parameter writes
do not imply that expression-routing support.
