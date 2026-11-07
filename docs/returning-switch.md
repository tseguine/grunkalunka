# Returning switch dispatch

The compiler admits source-order fallthrough, including grouped case labels,
ordinary statements, conditional early returns and switch-local break.
Discriminants and case expressions require positive
source proofs for the four represented scalar primitive kinds, a known String,
or references (objects, arrays and known functions), including mixtures of those
represented kinds. Empty or unknown origin sets do not establish admission.
Arms that continue an enclosing loop and operands without positive
represented-kind proofs remain guarded. Type annotations provide no source-kind proof.

The discriminant evaluates once. Case expressions evaluate in source order until
strict equality selects the first matching arm. Default may appear anywhere in
the source. It is selected as the initial entry only after every case test
misses, and can also be reached by fallthrough from an earlier selected case. A later
case test does not run after a match. Numeric NaN never matches; signed zeros
match; primitive kinds stay distinct; Strings compare exact UTF-16 contents.
References compare allocation identity, including fresh closures from the same
body and non-suspending async closures used as values. Switch observations
materialize function identity just like strict equality expressions; they do not
call the function. Mixed operands retain their language tags: a String never
matches a Number and a reference never matches null or a primitive. No coercion
or user callback runs during comparison. Saved discriminants survive case-test
mutation, throws, await and actual GC.

A normally completing arm falls into the next source arm without evaluating its
case test again; default participates in the same body order.
A default is optional. When all tests miss and no default exists, normal
completion reaches the enclosing sequence's suffix. A selected return or throw
bypasses that suffix; break reaches it. Empty switches still evaluate the
discriminant once.
An absent default cannot prove a complete function return by itself: the usual
structural return requirement still applies to the unmatched path.

Typed frames in the existing heap-owned source-control cursor lower each test
and arm in turn. Selection folds case tests iteratively into strict managed
equality/If IR and stores the first matching source index in an ancestor-owned
Number slot. A separate execution pass emits each body once in source order,
entering at that index and allowing normal completion into subsequent arms.
With a default, the last body is structurally unconditional after preceding
conditional bodies: every entry reaches it unless an earlier completion is
abrupt. The effect-free return query folds the same arm suffixes in reverse.
Execution stays inside a single-iteration DoWhile with a constant-false condition. Its nearest-break target consumes switch-local breaks; nested
source loops and nested switches keep their own targets. The lexical target is
retained even when a finalizer overrides a break with return or throw. Cleanup
runs before the break reaches the suffix, including awaited cleanup. No new
runtime operation or public ABI is added.
Each arm publishes locals only inside its branch. Shared-case TDZ reads remain
unsupported rather than reading an outer binding with the same name. The
effect-free completion query recognizes fallthrough return paths for checked
helper return inference. Source statement/depth, generated IR and guest execution
budgets retain their separate contracts. Boundary checks include 254 case arms,
just-over-limit statements and nested dispatch on a 2 MiB native stack.

`managed-logical` compares source results against native JavaScript, Node/Wasm
and Wasmtime, including the exact former `gap-switch` source and its original
fixed golden. Primitive-return, async reaction-order and forced-GC suites cover
the other existing interfaces. Canonical fixtures retain the promoted source hash
and independent native oracle. The remaining switch rejection group stays open.
