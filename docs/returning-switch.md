# Returning switch dispatch

The compiler admits `switch` when every non-final arm returns, throws or breaks
on every structural path. The final arm may complete normally and resume the
enclosing suffix, including through an empty body or a conditional early return.
Discriminants and case expressions require positive
source proofs for the four represented scalar primitive kinds, a known String,
or references (objects, arrays and known functions), including mixtures of those
represented kinds. Empty or unknown origin sets do not establish admission.
This control-flow slice admits unlabelled switch-local `break`. Fallthrough
between arms, arms that continue an enclosing loop and operands without positive
represented-kind proofs remain guarded. Type annotations provide no source-kind proof.

The discriminant evaluates once. Case expressions evaluate in source order until
strict equality selects the first matching arm. Default may appear anywhere in
the source, but its body runs only after every case expression misses. A later
case test does not run after a match. Numeric NaN never matches; signed zeros
match; primitive kinds stay distinct; Strings compare exact UTF-16 contents.
References compare allocation identity, including fresh closures from the same
body and non-suspending async closures used as values. Switch observations
materialize function identity just like strict equality expressions; they do not
call the function. Mixed operands retain their language tags: a String never
matches a Number and a reference never matches null or a primitive. No coercion
or user callback runs during comparison. Saved discriminants survive case-test
mutation, throws, await and actual GC.

A final default may likewise complete normally. A non-final default still must
return, throw or break, because otherwise it would fall into the following arm.
A default is optional. When all tests miss and no default exists, normal
completion reaches the enclosing sequence's suffix. A selected return or throw
bypasses that suffix; break reaches it. Empty switches still evaluate the
discriminant once.
An absent default cannot prove a complete function return by itself: the usual
structural return requirement still applies to the unmatched path.

Typed frames in the existing heap-owned source-control cursor lower each test
and arm in turn. The completed arms fold iteratively into existing If and
strict managed-equality IR, inside a single-iteration DoWhile with a constant
false condition. Its nearest-break target consumes switch-local breaks; nested
source loops and nested switches keep their own targets. The lexical target is
retained even when a finalizer overrides a break with return or throw. Cleanup
runs before the break reaches the suffix, including awaited cleanup. No new
runtime operation or public ABI is added.
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
