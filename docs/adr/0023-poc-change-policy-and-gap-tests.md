# ADR 0023: PoC change policy and advisory gap tracking

Date: 2026-11-13

Status: accepted

Maintainer decision, stated on 2026-11-13 after a steering review of `master`
[`d8147cd`](https://github.com/tseguine/grunkalunka/commit/d8147cd372c3d629e43f80f140246d148357a67b).
This ADR is short on purpose and covers working rules, not compiler design.

## Context

The project is a three-week-old proof of concept with no users. Its working
documents nevertheless inherited conventions from a conservative, compatibility-
minded workflow: preserve every existing contract, state every boundary, and
test that anything not yet implemented fails to compile with an exact
diagnostic. Three effects were observed in the review:

- Valid source that is merely unimplemented is pinned as a required rejection.
  The [rejection inventory](../../tests/compatibility/rejection-inventory.json)
  is documented in [testing](../testing.md) as 995 negatives, of which only 31
  are invalid source and 144 are resource limits; the other 820 are open
  requirements. Roughly a thousand further Rust assertions expect a compile
  error. General-lowering changes after [ADR 0022](0022-general-js-ts-lowering-first.md)
  broke `master` or needed follow-up repairs because such tests still required
  rejection of now-working input (#971, #977).
- ADR 0022 was accepted, but the work that followed still admitted one source
  shape per PR, because the surrounding documents told agents to preserve
  existing profiles, ABIs, artifacts and diagnostics.
- Documentation grew into a prose copy of the admission rules and a log of
  receipts, which agents must read and keep consistent.

## Decision

1. **No backward compatibility during the PoC.** Internal ABIs, entry points,
   compile profiles, IR operations, diagnostic text, artifact bytes and document
   structure may change or be removed without shims, deprecation or parallel
   old/new paths. Byte-identical artifacts and unchanged diagnostics are not
   acceptance criteria.
2. **Correctness of executed programs is the protected invariant.** An accepted
   program must match native JavaScript, apart from divergences that are
   written down. This is the one thing a change may not regress silently.
3. **Rejection tests fail the build only for permanent rejections:** invalid
   source, explicit resource limits, and unavailable or denied capabilities.
4. **Known gaps are advisory.** Valid source that does not compile or run yet
   is recorded in the coverage census that ADR 0022 calls for. The census is
   reported in CI and never fails it. No new test may assert that valid source
   must be rejected or pin the wording of an "unsupported" diagnostic.
5. **Existing gap rejections are disposable.** When one blocks a change, delete
   it or move its source into the census in that same PR, and add an execution
   check for what now works. No separate justification is required.
6. **Less ceremony.** Routine work needs no ADR, receipt document or boundary
   statement. Documents describe how the system works now; history lives in Git.

## Scope and non-goals

This does not relax execution oracles, native-stack hygiene, capability
isolation, CI honesty rules or the maintainer's authority over priorities in
[AGENTS.md](../../AGENTS.md). It does not select the general value, call or
completion representation that ADR 0022 left open; that remains a maintainer
design decision. It ends when the maintainer declares the PoC phase over.

## Alternatives considered

- **Keep pinning every unimplemented form as a rejection.** Detects accidental
  admission of something that would miscompile, but makes every generalization
  pay for hundreds of obsolete assertions. Rejected; execution oracles are the
  defense against miscompilation.
- **Delete all rejection tests.** Simplest, but loses real checks on invalid
  source and resource limits. Rejected.
- **Keep gap tests but mark them expected-to-fail.** Still couples the build to
  the gap list. Rejected in favor of a non-blocking census.

## Consequences

Generalizing changes become cheaper and may temporarily accept programs with
documented divergences. An accidental admission that miscompiles is caught by
execution tests or not at all, so new general paths need execution coverage.
Follow-up code work, not done by this ADR: split the rejection inventory into
permanent rejections and census entries, make the census job advisory, and
convert or delete the Rust assertions that expect a compile error for valid
source as they are encountered.

## Verification and evidence

Documentation only. The review compiled 95 small hand-written programs at
`d8147cd`: 58 were accepted, and all 55 that could be run through the numeric
entry point matched Node on six input pairs. That sample is not representative
and is offered as a seed for the census, not as a coverage figure.

## Summary

This is a PoC: break internal contracts freely, keep executed programs correct,
test rejection only where rejection is permanent, and track everything else as
advisory gaps.
