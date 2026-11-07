# Returning switch dispatch

The compiler admits `switch` when every non-final arm returns or throws
on every structural path. The final arm may complete normally and resume the
enclosing suffix, including through an empty body or a conditional early return.
Discriminants and case expressions require positive
source proofs for the four represented scalar primitive kinds or a known String.
This is a first control-flow slice; fallthrough between arms, switch-local `break`,
arms that continue an enclosing loop and broader reference
discriminants remain guarded. Type annotations provide no source-kind proof.

The discriminant evaluates once. Case expressions evaluate in source order until
strict equality selects the first matching arm. Default may appear anywhere in
the source, but its body runs only after every case expression misses. A later
case test does not run after a match. Numeric NaN never matches; signed zeros
match; primitive kinds stay distinct; Strings compare exact UTF-16 contents.
Saved discriminants survive case-test mutation, throws, await and actual GC.

A final default may likewise complete normally. A non-final default still must
return or throw, because otherwise it would fall into the following arm.
A default is optional. When all tests miss and no default exists, normal
completion reaches the enclosing sequence's suffix. A selected return or throw
bypasses that suffix. Empty switches still evaluate the discriminant once.
An absent default cannot prove a complete function return by itself: the usual
structural return requirement still applies to the unmatched path.

Typed frames in the existing heap-owned source-control cursor lower each test
and arm in turn. The completed arms fold iteratively into existing Block/If and
strict managed-equality IR; no new runtime operation or public ABI is added.
Each arm publishes locals only inside its branch. Shared-case TDZ reads remain
unsupported rather than reading an outer binding with the same name. The
effect-free completion query recognizes the same terminal-arm shape for checked
helper return inference. Source statement/depth, generated IR and guest execution
budgets retain their separate contracts. Boundary checks include 254 case arms,
just-over-limit statements and nested dispatch on a 2 MiB native stack.

`managed-logical` compares source results against native JavaScript, Node/Wasm
and Wasmtime, including the exact former `gap-switch` source and its original
fixed golden. Primitive-return, async reaction-order and forced-GC suites cover
the other existing interfaces. Canonical fixtures retain the promoted source hash
and independent native oracle. The remaining switch rejection group stays open.
