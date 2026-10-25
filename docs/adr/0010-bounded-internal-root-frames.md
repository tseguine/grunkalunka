# ADR 0010: Bounded internal root frames

Date: 2026-10-25

Status: accepted

## Context

Owned JSON and raw String roots survive until explicit release. Generated functions
also need temporary String roots, which must be reclaimed without releasing host
roots or aliases still used by callers. The source experiments show that retaining
all local cells exhausts the 64-root quota; caller frames and IR-derived liveness
can distinguish the roots that must survive.

The earlier frame experiment trusts call ordering and can panic on malformed
protocol calls. A maintained boundary needs checked tokens and explicit limits.
This unit defines the guest ownership protocol; compiler admission follows separately.

## Decision

Use a stack of at most 128 frames, each with a set of protected internal root IDs.
Frame IDs are nonzero monotonically increasing u32 values and are never reused. A
mutation naming a frame must name the current top token. Stale, missing and
out-of-order tokens return InvalidFrame without changing frames, roots or input.
Frame ID exhaustion is terminal for entering new frames, but existing frames can
still be completed or aborted. It is separate from root ID exhaustion.

Internal String construction copies the initialized raw UTF16 input. Before an
internal allocation, collect internal roots absent from all active frames. External
roots are always retained. Clearing a frame removes its pins but does not collect;
the caller can safely clear then repin all live values before allocating again.
Pinning accepts only live internal roots and changes only the current frame.

Leaving validates the token and optional returned internal root before mutation.
The returned root is protected by the parent frame, or becomes externally owned
when the outer frame leaves. Other unprotected internal roots are collected.
Returning an already external root is rejected rather than silently transferring
another owner's release obligation. Aborting removes every frame and every
internal root, preserving external roots and the input. It is idempotent.

Every mutating protocol operation clears old output, including errors. Errors
otherwise preserve state. Existing input bounds, root IDs, 64-root quota, external
constructors and status 0-9 meanings remain unchanged. Explicit release removes a
root from internal ownership and all pin sets, preventing stale pin accumulation.

New statuses are 10 FrameLimit, 11 InvalidFrame, 12 NotInternal, 13 FrameExhausted.
Native methods and matching Wasm exports follow the same checked semantics.

| Export | Signature | Result |
| --- | --- | --- |
| gk_heap_enter | () -> i32 | Nonzero frame token, or zero with status |
| gk_heap_clear | (frame:i32) -> i32 | Status; clear current pins without collection |
| gk_heap_pin | (frame:i32,root:i32) -> i32 | Status; protect a live internal root |
| gk_heap_string | (frame:i32) -> i32 | Nonzero internal String root, or zero with status |
| gk_heap_leave | (frame:i32,returned:i32) -> i32 | Status; zero returned ID means no result |
| gk_heap_abort | () -> i32 | Success status; remove temporary ownership |

Root IDs and frame tokens are separate domains. Native leave accepts an optional
root; Some(0) is an invalid root, while the Wasm ABI reserves zero for no result.
Frame validation precedes root validation. Internal construction checks terminal
root-ID exhaustion before collection so failure cannot discard an existing value.
The checked factory then applies the shared live-root quota.

## Scope and non-goals

Only raw String construction is available internally in this unit. There is no
source admission, object graph tracing, String operator dispatch, exception mapping,
General helper ABI, parameter-object layout or compiler liveness implementation.
The 128-frame limit is an engineering bound for this new profile; existing numeric
and primitive source limits do not change.

Abort is cleanup after a normally returned guest error and subsequent generated
trap. It is not recovery from arbitrary engine interruption inside Rust guest code.
Such interruption can strand the guest state lock; a future heap host must discard
the instance and invalidate its handles instead of trying to reuse it.

## Alternatives considered

Keeping every allocation until function exit loses bounded repeated-allocation
programs. Reference counting would require updates at every alias and value copy.
Tracing all source cells retains dead storage; the compiler must eventually supply
live cells and value snapshots at safepoints. This guest accepts explicit pin sets
without deciding the compiler's analysis algorithm.

Unchecked stack depth is simpler but permits malformed calls to panic and cannot
identify stale callers. Reusable frame indices risk accepting an old token for a
new frame. Monotonic IDs and checked top identity avoid that ambiguity.

## Consequences

At most 128 sets each contain at most 64 live root IDs. Release removes stale pins,
so retained metadata remains bounded. Collection examines bounded roots and frames;
this is not an exact fuel or allocator guarantee. The host remains a cooperating,
synchronous owner of guest memory, with no reentry or callbacks.

Compiler and host integration must enforce this ownership protocol and separately
bound analysis, initialization, execution, memory and result copies. The numeric
and four-kind primitive profiles remain separate.

## Verification and evidence

Native TDD and actual Wasm checks cover token order, nested transfer, outer ownership,
collection, aliases, external roots, quotas, stale tokens, failures and exhaustion.
The existing JSON/String oracle and composed-guest checks must remain green.
Full workspace checks and complete diff review precede integration.
