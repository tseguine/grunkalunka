# Cross-engine reporting in CI

The browser smoke workflow's `cross-engine` job joins the Chromium, Firefox and
WebKit artifacts from the **same workflow run**. Downloads remain in separate
engine-named directories. The source comes from the same tested checkout; the
reporter hashes it and requires matching capture receipts before comparison.
No external repository, user-supplied URL, privileged token or latest-run lookup
is involved. The added job needs Node only, not Rust or another browser install.

`report.json` in artifact `browser-cross-engine` retains all
three engine slots, raw inputs, pair differences, coverage and provenance errors.
The original per-engine artifacts remain available independently for seven days.
A failed engine does not suppress reporting of the remaining evidence: the join
is attempted unless cancelled, and missing captures are explicitly incomplete.
Malformed, missing, incompatible or unreadable evidence fails the report job.

Unanimous equality is **not** a pass condition for this diagnostic report. Exit 1
from the reporter means fully comparable observations differ and is retained
without failing this job. Exit 0 means the reported fields agree. Both still state
`conformance: not-assessed`. All other exits, including execution timeouts, fail.
The existing per-engine spec-derived baseline, repeatability and mutation checks
remain unchanged and still fail the workflow independently. No tolerance is widened.

The report step has a 30-second watchdog inside a five-minute job. A genuine
browser difference, incomplete evidence and a normative assertion failure remain
separate outcomes; none is silently relabeled as another. See
[browser-oracle-cross-engine.md](browser-oracle-cross-engine.md) for the detailed
receipt contract and the limits of source hashes and environment metadata.

## Node toolchain consistency

All browser producers and the report job select the same exact Node patch
version from workflow-level `BROWSER_NODE_VERSION`, currently `24.21.0`, and
assert the selected runtime before doing work. Update that one pin deliberately;
do not use a floating major or choose versions independently in matrix jobs.
The initial hosted join correctly rejected captures from Node `24.21.0` and
`24.22.1` in the same run when the workflow requested `24`. The environment
compatibility check remains strict; changing metadata or ignoring the mismatch
is not a fix. The pin concerns the job toolchain, not actions' internal runtimes,
and does not freeze OS images, fonts or transitive npm dependencies.
