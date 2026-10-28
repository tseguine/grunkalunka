# ADR 0017: Use upstream issue reports as non-normative engineering evidence

Date: 2026-10-28

Status: proposed

## Context

Grunkalunka aims to provide portable, capability-oriented JS/TS application
environments, including browser, Node-compatible and embedded/desktop use.
Implementation experience from mature projects can reveal failure modes,
interoperability expectations, difficult design tradeoffs and poor UX before
we encounter them ourselves.

The existing [historical bug-hardening proposal](../historical-bug-hardening.md)
identifies Chromium, Mozilla/Firefox, WebKit, Electron, CEF and Ultralight/
AppCore reports as sources of potential regressions. That is only part of the
value of public issue trackers. Reports closed as WONTFIX, invalid or duplicate
can contain independently useful reproductions, workarounds, use cases, UX
complaints and objections to the maintainers' chosen behavior. A duplicate
classification may be appropriate for an upstream team's workflow without
making the reports' evidence interchangeable.

Other projects also operate under their own backwards-compatibility promises,
architectures, organizational priorities and users' expectations. Their fixes,
rejections and emphatic explanations are not normative Grunkalunka decisions.
Equally, literal specification compliance may fail to interoperate with widely
deployed de facto behavior. The difference between a written specification,
independently observed engine behavior and actual application reliance matters.

The project already maintains separate source/runtime compatibility evidence,
browser reference oracles, capabilities and resource bounds. This proposal
introduces **a method of interpreting upstream evidence**, not an additional
source-language admission, test gate or authority to change a workstream.
Inspection baseline: `master` `0c8bc279f0ed3ec94fa44183e51a958713d6a2ac`.
Reconcile future source and decisions before implementing this proposal.

## Decision

**If accepted, curate public upstream issue reports as evidence, independently
analyzed against Grunkalunka's contracts and goals.** Do not treat an upstream
status, maintainer opinion, standards document, engine consensus or issue count
as an automatic instruction to copy, reject or implement behavior. Record the
reasoning that connects evidence to our own conclusions.

### Preserve source reports separately from synthesized findings

Use source-controlled Markdown with a small, machine-readable metadata header
and human-readable analysis. The proposed initial organization is:

```text
research/upstream-evidence/
  README.md
  reports/
    chromium/<upstream-id>.md
    mozilla/<upstream-id>.md
    webkit/<upstream-id>.md
    electron/<upstream-id>.md
    cef/<upstream-id>.md
    ultralight/<upstream-id>.md
  findings/
    <stable-failure-mode-or-design-pressure>.md
```

The exact tree and metadata schema can be refined in a bounded pilot. Keep
**one distinct record per upstream report selected for retention**, even when
upstream marks it duplicate or resolves it into another ticket. A lightweight
record can suffice for an otherwise redundant "same here"; preserve substantial
new evidence rather than collapsing it into the upstream canonical issue.
Do not bulk-mirror the entire tracker merely to populate an index.

A *report record* must retain, where available:

- Stable source/project and upstream ID/URL; observation date; reported product,
  versions, platform and relevant configuration.
- The upstream status and resolution **as observed**, including duplicate
  target, stated WONTFIX rationale, relevant discussions, reproductions,
  fixes, workarounds and upstream tests. A claim made by a reporter or
  maintainer is attributed to its author, not silently adopted as fact.
- Its **novel contributions** relative to connected reports: additional
  reproduction conditions, independent environment, affected workflow,
  accessibility/UX impact, proposed alternative, objection or counterexample.
- An evidence assessment: confirmed behavior, independent reproduction,
  plausible but unverified report, disputed expectation or user preference.
  Preserve uncertainty; do not infer prevalence or severity from issue counts.
- An explicit **relevance rationale** identifying the Grunkalunka subsystem,
  supported behavior, selected constraints and non-applicable implementation
  details. Link the inspected commit/path when source has actually been checked.
- Local disposition and verification evidence, including local test/PR/steering
  references where applicable, plus concrete conditions for reassessment.

A *finding record* independently synthesizes one or more reports into a
**technical failure mode**, **interoperability requirement** or **design/UX
pressure**. Reports can contribute to multiple findings and findings can draw
from multiple upstream sources. Every claim should link its contributing report;
this is many-to-many evidence, not a one-ticket-to-one-test mapping. Link the
same finding to multiple local tests if necessary. Generate indexes from the
Markdown records; avoid a separately edited database of conclusions.

### Separate orthogonal classifications

Do not let `FIXED`, `WONTFIX`, `INVALID`, `DUPLICATE` or `CLOSED`
stand in for our assessment. Track at least these independent questions:

| Dimension | Examples of findings |
| --- | --- |
| Upstream administrative outcome | Open, fixed, duplicate of another report, WONTFIX, invalid, unconfirmed; preserve the actual resolution rationale |
| Evidence quality | Confirmed reproduction, corroborated observation, disputed expectation, unverified assertion; state source and environment |
| Problem type | Behavioral defect, security invariant, performance/resource failure, UX friction, design alternative, interoperability quirk |
| Local applicability | Already covered, relevant but untested, future/unsupported, architecture-excluded with proof, not applicable, needs investigation |
| Local action | Regression candidate, design consideration, investigate, defer, no action; include owner only after maintainer assignment |

A WONTFIX decision may be weak evidence of a genuine UX problem without
demonstrating its prevalence. A duplicate may supply the only accessible
reproduction for another platform. A report closed as invalid may still
document user confusion, but it must not be labeled a confirmed defect for
that reason alone. Maintain both the upstream conclusion **and** our possible
disagreement, with supporting evidence and uncertainty.

### Evaluate compatibility from independent kinds of evidence

For a relevant behavior, analyze separately:

1. **Formal contract:** which specification/version applies to an explicitly
   selected compatibility surface, and what does it require or leave open?
2. **Observed de facto behavior:** which independent engine families, versions
   and configurations actually exhibit the behavior? Chromium, Electron and
   CEF are not three independent Blink votes; Ultralight is WebKit-derived.
3. **Application reliance:** what real applications, workflows or users
   demonstrably depend on the behavior? Engine consensus is not proof of use.
4. **Product impact:** what UX, accessibility, performance and maintenance
   effects follow from each alternative?
5. **Security and authority:** could emulation violate explicit grants,
   isolation, resource limits or the selected threat model? Can compatibility
   be obtained through a safer internal mechanism or bounded profile?

Prefer standards conformance wherever it serves the selected contract and is
reasonably compatible. But **do not assume literal conformance always yields
the most interoperable result**: a widely relied-upon de facto convention may
justify a documented exception or a deliberately permissive input boundary.
Conversely, copying a widespread quirk solely because engines agree may
perpetuate an avoidable flaw. Compatibility is about observable dependence,
not reproducing another project's internal implementation.

A formal standard is normative **within a compatibility commitment we actually
make**, not an automatic mandate for every separate embedding API or host
capability. Resolve genuine conflicts as explicit product/architecture choices,
with target versions, competing behaviors, consequences and evidence. Do not
quietly weaken a promised JS/TS or web-platform contract, or represent a
deliberate incompatibility as conformance.

### Treat security and memory safety as distinct assessments

Tag security-related evidence independently: authority escalation, origin or
context confusion, stale/forged resource handles, reentrancy and lifecycle,
malformed input, algorithmic denial of service, memory unsafety, supply chain
and host-service exposure. An upstream C/C++ use-after-free may suggest a
relevant ownership or callback-order invariant even if Rust prevents that exact
memory error. Do not claim an entire bug family is impossible merely because
our implementation is written in Rust.

Preserve Grunkalunka's explicit capability and resource boundaries. Before
adopting a compatibility quirk that crosses an authority boundary, require
a specific security analysis and a maintainer-owned decision; do not silently
grant ambient access to match another runtime. Reproduce useful observable
behavior **without** copying vulnerable mechanisms where possible.

The long-term direction is to **prefer safe Rust and progressively minimize
unsafe Rust, FFI/native code and the trusted dependency surface where
practical**. This is not an immediate ban on dependencies required by the
current frontend, Wasmtime/JIT, platform integration or other experiments.
Evaluate dependency exposure, unsafe interfaces, provenance, isolation and
replacement cost explicitly; count neither an `unsafe` keyword nor a
safe-Rust dependency as a complete security verdict. This ADR does not
approve or remove any dependency, select a sandbox or certify memory safety.

### Review and action boundaries

Select a bounded set of reports for an already implemented or explicitly
assigned feature. For each, preserve provenance, reduce useful observations to
invariants or design pressures, inspect the current code/test contracts and
establish expected behavior independently. Prefer an original minimal
reproduction and a fault-sensitive local regression where one is justified.
Existing tests should be linked as coverage; a rejected source program is not
implemented compatibility evidence.

`architecture-excluded` requires a concrete invariant and code/test evidence,
not intuition. `future/unsupported` includes a revisit trigger tied to feature
admission. Finding a relevant gap in another agent's active work goes through
that agent's existing PR/issue or a bounded advisory `steering:` issue under
[AGENTS.md](../../AGENTS.md). **No upstream report automatically becomes a
GitHub issue, priority, workstream assignment or implementation approval.**

Store links, concise paraphrases and independently authored tests by default.
Public visibility does not authorize redistribution of whole issue threads,
copyrighted test code, proprietary artifacts, personal data or weaponized
security reproductions. Review the exact license and provenance before copying
material. Public-only discovery must not probe embargoed security reports.

## Scope and non-goals

This proposal governs *evidence interpretation, traceability and review* for
upstream projects relevant to Grunkalunka, initially browser engines and
embedders. It can later cover external runtimes, tools or libraries when a
maintainer explicitly broadens the selected review.

It does **not** establish the final browser/Node/desktop compatibility target,
admit language features, choose a renderer, modify WIT/capability policies,
change dependency/license rules, replace accepted ADRs, implement a crawler,
import a bulk issue corpus, create recurring jobs, change CI, or choose the next
A1 milestone. It does not turn independent browser oracles into native
Grunkalunka execution evidence. Existing source budgets, test gates, ABI and
resource limits are unchanged.

## Alternatives considered

- **Copy upstream fixes and regression tests directly.**
  - Advantage: Concrete known scenarios and possibly fast coverage.
  - Disadvantages: Architecture mismatch, licensing obligations, accidental
    copying of upstream assumptions and missing independent expectations.
  - Disposition: Reject as the default; consider licensed, pinned adaptations
    only after independent relevance and test design.
- **Treat upstream triage and maintainer judgment as authoritative.**
  - Advantage: Simple decisions and reduced analysis effort.
  - Disadvantages: Imports their constraints and discards useful WONTFIX,
    invalid and duplicate reports.
  - Disposition: Reject.
- **Follow written specifications mechanically, ignoring implementation use.**
  - Advantage: Clear reference targets.
  - Disadvantages: Can regress real interoperability where widely deployed
    behavior has diverged or a contract leaves latitude.
  - Disposition: Reject as a universal rule; retain specifications as a
    necessary reference for the explicitly selected conformance surface.
- **Mirror entire public trackers or create a GitHub issue for each report.**
  - Advantage: Searchable large dataset and easy automated ingestion.
  - Disadvantages: Volume, stale/low-signal records, copyright/privacy risk,
    duplicate effort and unreviewable prioritization.
  - Disposition: Defer bulk ingestion; retain curated Markdown records and
    actionable shared findings.
- **Keep only confirmed, fixed bugs with executable regressions.**
  - Advantage: Stronger correctness evidence and immediate test value.
  - Disadvantages: Loses UX pressure, disputed tradeoffs, portability variants
    and reports about features we have not implemented yet.
  - Disposition: Reject as the sole evidence category; require precise labels
    for weaker evidence.

## Consequences

The repository can connect a design or test decision to original observations
and distinguish corroboration from repetition. Separate reports may reveal new
environment-specific or user-facing information even when upstream groups them
as duplicates. Findings may justify testing, better API contracts or a documented
choice to diverge, without forcing an implementation.

Costs include human review, stale links/statuses, imperfect issue metadata,
reporting bias, licensing review and keeping findings synchronized with current
code. Do not infer a compatibility percentage, project-wide defect rate or UX
severity distribution from this selected corpus. Keep a record's last upstream
observation and last local code review separate; update only when evidence or
relevant product contracts change, not on every unrelated commit.

The proposed methodology by itself guarantees no compatibility, performance
or security property. Its value must be demonstrated by independently useful
findings and traceable decisions, not the number of reports ingested.

## Verification and evidence

**Observed while drafting:** the current `master` root [AGENTS.md](../../AGENTS.md),
[historical bug-hardening proposal](../historical-bug-hardening.md),
[roadmap](../roadmap.md), [dependency/license policy](../dependencies.md),
[ADR template](TEMPLATE.md) and existing ADR numbering/statuses were reviewed.
The known upstream issue source links in the historical proposal are candidate
discovery points, **not an ingested or audited corpus**.

**Not executed:** no external tracker report was individually classified, no
upstream issue or test was copied, no bug regression was added, no CI command or
security/dependency scan was run as part of this documentation change. Prior
project tests are not verification of this new process.

**Evidence required before accepting and operationalizing:** choose a small,
maintainer-approved set of distinct fixed, WONTFIX and duplicate reports.
Demonstrate that the records preserve distinct contributions and uncertainty;
that one many-to-many finding maps them correctly; that applicability can be
reviewed against a pinned Grunkalunka commit; and that any chosen executable
regression has an independent expected result and actual run evidence. Check
metadata/links and copying terms, and measure useful findings versus review
cost. A proposed machine-readable schema or index validator should be separately
reviewed before becoming an enforced CI or repository contract.

## Explicit boundary statement

This ADR **proposes a knowledge/evidence boundary**, not a new runtime or
browser contract. Upstream maintainer decisions, specifications and de facto
behavior enter as separately attributed evidence; Grunkalunka's compatibility,
UX and security decisions remain independently justified.

The maintainer must still choose which investigations to undertake, whether
to accept this ADR, which compatibility contracts to commit to, and which
technical changes or tests to assign. No new roadmap priority is inferred.

## Related work and follow-up

- [Historical issue-driven hardening](../historical-bug-hardening.md) is a
  narrower proposed *regression-discovery* application of this broader method.
- [Browser conformance evidence](../roadmap.md#browser-conformance-evidence-and-feature-roadmap)
  and [browser oracles](../browser-oracles.md) distinguish references from
  Grunkalunka implementation.
- [ADR 0012](0012-guest-runtime-and-host-capabilities.md) proposes the
  semantic/host-authority boundary; [capability evidence](../capabilities.md)
  supplies bounded existing controls.
- [Dependency policy](../dependencies.md) addresses current licenses; a
  separate reviewed milestone would be needed for dependency safety/unsafe
  minimization, with measured constraints.
- [AGENTS.md](../../AGENTS.md) governs review authority, advisory steering
  and maintainer-owned roadmap assignments.

If authorized later, pilot report templates and one generated index in a
separate bounded PR. A separate ADR or scoped test decision should establish
any consequential compatibility exception or new security policy.

## Summary

Preserve upstream issue reports, including duplicates and rejected changes,
as distinct evidence. Synthesize failure modes, interoperability needs and
UX/design pressures separately. Formal specifications, observed de facto
behavior and application reliance inform—but do not automatically determine—
Grunkalunka's decisions. Security boundaries and the long-term safe-Rust goal
receive independent assessment. No automated ingestion or implementation work
is assigned by this proposed ADR.
