# Discarded logical expressions

An unused `left && right`, `left || right` or `left ?? right` keeps the original
short-circuit decision and evaluates only the selected right operand. Its operands
do not need a common result carrier. Represented function and Promise assignments
can therefore appear on the right of a scalar condition without becoming scalar
results themselves.

The existing evaluator computes the left operand once, retains its effects and
uses truthiness or nullishness as required. Functions and Promises are truthy and
non-nullish; a Promise fulfilled with null is still a non-nullish Promise. Mixed
Promise/primitive values keep their existing raw condition checks. Selected right
operands use the ordinary discard path; the branch has no result join.

Await, failure, capture and assignment effects keep their source order. A failing
left skips the right; a skipped right adds no effects, rejection handling or jobs.
Simply discarding a Promise does not adopt, await or handle it.

An internal dispatch mode retains the shared source-depth and aggregate-region
guards before splitting the expression. Existing source validation also checks
skipped branches. Branch construction stays outside the central evaluator frame,
preserving the existing two-MiB constructor probes. There is no new IR operation,
runtime ABI or dependency.

Maintained native-JS/Node-Wasm/Wasmtime tests cover closures, constructor effects,
Promise assignments, mixed nullish tests, awaits, failures, nested operators,
JS/TS erasure and nesting boundaries. Notification fixtures compare native
rejection checkpoints with the guest protocol. This does not make unrelated
unrepresented operands or value-producing cross-kind joins lowerable.
