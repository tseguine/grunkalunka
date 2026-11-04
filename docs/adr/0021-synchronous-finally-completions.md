# ADR 0021: Direct synchronous finally with saved completions

Date: 2026-11-04

Status: proposed

This records the bounded implementation in the accompanying compiler/runtime
feature slice. It does not assert acceptance of a broader exception ABI.

## Context

Local synchronous primitive catches now execute through the direct typed-region
emitter. The existing `TryFinally` IR already separates the protected body,
finalizer, saved return/reason and return continuation, but its source lowering
previously forced raw async returns and its direct emitter rejected cleanup.
Ordinary handlers need cleanup around returns and loop exits without requiring
a Promise activation. Source-dependent traversals must remain iterative.

## Decision

Keep one emitted synchronous finalizer body per `TryFinally` node. A private
Wasm `i32` records the pending normal, return, throw, break or continue kind and
is reset on every dynamic entry. Existing typed value locals save the pending
return and reason before branching to cleanup.

Protected returns enter cleanup. Finalizer returns replace the saved return
and branch past the rest of cleanup. Both then execute the node's return
continuation, preserving the existing IR contract. Finalizer throw/break/continue
use their enclosing destinations directly. Normally completed cleanup dispatches
the saved kind through those destinations. Inner catches and loops temporarily
install their own routes, and nested ordinary calls keep their own return route.

Source lowering keeps the ordinary function's private result type and an
identity return continuation. Async lowering retains raw `AsyncValue` captures
and its existing delayed adoption continuation. Neither a capture type nor
finally syntax alone implies async execution: routing scans the actual nested
operations. Finally does not grant a new synchronous throw contract.

Ordinary Wasm functions declare their own completion locals. The async advance
function also declares owner-qualified locals when its atomic settlement bodies
contain ordinary helper cleanup. Such bodies cannot suspend, so these locals
need no persistent frame lane or spill protocol.

## Scope and non-goals

This covers direct synchronous cleanup in existing numeric, primitive-return,
managed private-call and A1 compositions. Public return encodings and capability
interfaces stay as defined by their existing profiles. Source, local, statement,
nesting and execution-fuel limits continue to apply.

Explicit synchronous throws still require a same-function lexical catch and a
represented primitive reason. A finalizer that could suppress an otherwise
uncaught throw does not independently admit it. Cross-call propagation,
managed exception values, catchable intrinsic Error production and labels need
their own implementation. Host fuel, trap and provider failures remain terminal
and do not run source cleanup.

## Alternatives considered

- **Reuse async splitting for every finally.** This already represents suspended
  cleanup, but introduces continuation states and async result handling for
  ordinary functions. It remains appropriate for actual async finalizers.
- **Duplicate cleanup at every abrupt exit.** This can avoid a discriminator,
  but nested cleanup expands emitted code and duplicates source-owned work.
  A shared cleanup body gives a bounded representation per IR node.
- **Change the public ABI or use Wasm exception transport.** Cross-call errors
  may need a broader completion design, but that decision is unnecessary for
  existing locally closed primitive throws and ordinary loop/return cleanup.

## Consequences

The direct emitter needs explicit typed return, throw and loop destinations,
plus two return branch destinations around cleanup. It emits one finalizer body
and one private discriminator per node. Return/reason snapshots use the same
layouts as the surrounding function, including traced private values. There is
no new runtime import, public tag or Promise queue for synchronous cleanup.

The verifier must keep capture visibility, continuation typing and function
completion ownership separate. An ordinary `AsyncValue` private result is not
evidence of an async function; entering finally must not create raw throw
authority solely from that representation.

## Verification and evidence

The ordinary compiler/runtime suites exercise completion preservation and
override, nested loops/catches, helper boundaries, typed private returns and
atomic settlement composition. Differential tests compare original JavaScript
with Node-Wasm and Wasmtime; A1 tests also inspect the actual Rust/WIT provider's
calls and state. Direct IR checks cover captures, continuations, one emitted
cleanup body and deep iterative traversal on a 2 MiB native stack.

The exact original `gap-finally-return` and `continue-finally-remains` fixtures
retain their source hashes and Number oracles when promoted. The latter has an
unselected loop, so separate taken-loop cases supply runtime cleanup evidence.
Local receipts and hosted results belong to the implementation PR; test presence
alone does not establish a passed gate or general ECMAScript conformance.

## Explicit boundary statement

This introduces direct synchronous cleanup for existing typed completions.
It does not turn terminal host failures into source errors or supply an escaping
exception ABI. Future cross-call and intrinsic-error work must define its value
flow, coercion and boundary transport separately.

## Related work and follow-up

See [synchronous errors](../synchronous-errors.md), [async finally](../async-finally.md),
[ADR 0014](0014-backend-value-layout.md), and
[ADR 0020](0020-source-lowering-control-kernel.md). Array `push` overflow and
nullable `Object.keys` remain consumers of a future coherent intrinsic Error
contract; cleanup support alone does not satisfy that contract.
