# ADR 000N: <Short, specific title>

Date: YYYY-MM-DD

Status: <proposed | accepted | superseded | rejected>

## Context

Describe the architectural or engineering problem that motivates this decision.

Include:
- the current state or baseline
- the constraint, requirement, or failure mode
- any relevant existing invariants, source limits, ABI contracts, or runtime assumptions
- the reason this decision is needed now
- what is intentionally not being changed yet

Keep this section concrete and evidence-oriented. If relevant, reference existing compiler, runtime, or Wasm boundaries and why they matter.

## Decision

State the decision clearly and precisely.

Include:
- the concrete design choice
- the exact boundary or contract introduced
- preserved invariants
- explicit assumptions
- any scope limitations

This section should read like the final decision statement, not a loose discussion.

## Scope and non-goals

Define the scope explicitly.

Include:
- what this ADR covers
- what it does not cover
- any runtime/ABI/heap/object-model boundaries that remain separate
- any source, memory, execution, or diagnostic constraints intentionally preserved

This section is important for keeping the ADR set disciplined and for avoiding ambiguous “broad” architecture claims.

## Alternatives considered

List the main viable options and explain why they were rejected, deferred, or retained as future work.

A consistent structure works well:

- Option A: <short name>
  - Advantages:
  - Disadvantages:
  - Disposition: <accepted | rejected | deferred>

Focus on tradeoffs and boundary decisions rather than vague preference.

## Consequences

Describe both the expected benefits and the costs.

Include:
- implementation complexity
- compatibility effects
- runtime or ABI consequences
- preserved invariants
- required follow-up work
- any unproven or intentionally limited promises

Be explicit about what this decision guarantees and what it does not.

## Verification and evidence

Describe how the decision was validated.

Include:
- native tests
- reference comparisons or oracles
- build/workspace gates
- invariant checks
- any manual or automated verification performed
- the limits of the evidence

This section should distinguish:
- what was verified
- what was not verified
- what remains subject to future evidence

## Explicit boundary statement

State the design boundary in plain terms.

Answer:
1. What does this ADR introduce?
2. What does it intentionally not do?
3. What future decision remains required?

This matters for the repo’s architecture because many ADRs intentionally narrow scope rather than claiming a full runtime or heap solution.

## Related work and follow-up

List any deferred work, dependencies, or relevant ADRs.

Examples:
- Related ADRs: 0001, 0002, 0005
- Follow-up work: <list>
- Future architecture decisions required: <list>

## Summary

Provide a short final summary in plain language.

This should capture the essential decision in a few sentences and make the ADR readable to someone scanning the directory.

---

Template Notes:
- Prefer titles that are specific and concrete.
- Keep status values standardized: proposed, accepted, superseded, rejected.
- If the ADR is intentionally narrow, say so in the first paragraph.
- If the decision leaves future runtime or heap architecture open, state that explicitly.
- Prefer “known limits” language over broad claims of completeness or general correctness.
