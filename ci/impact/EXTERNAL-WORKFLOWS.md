# Independent browser workflow impact audit (advisory)

The canonical CI check catalog enumerates only the commands in
`scripts/check.mjs`. Browser-specific tests run in three **independent,
path-filtered workflows** that are not represented by those commands:
`browser-smoke.yml` (Chromium, Firefox and WebKit plus cross-engine report),
`browser-oracles.yml` (offline oracle tests), and `browser-collector.yml`
(offline collector contracts). The new browser workflow descriptors under
`ci/impact/workflows/` record the actual PR/push trigger paths and expected
source commands. The read-only planner validates both trigger sections and
reports which workflows **should be triggered by path filters**, not which
hosted jobs actually ran or passed.

The audit is deliberately narrow. It only understands the literal YAML
path-list forms presently used by these three workflows; unfamiliar YAML,
negative filters, absent or changed source commands, missing engine matrix,
or an outdated descriptor produces a **fail-closed full plan**. The
browser workflows themselves are unchanged and continue to trigger
independently. A full canonical CI result cannot substitute for an
untriggered browser workflow.

## Tool-only browser edit coverage

The offline `tests/browser-oracles/*.test.mjs` suite imports helpers from
`tools/browser-oracles/`, so it must rerun when those helpers change. The
workflow originally covered test files but not tool-only edits, as recorded
in [steering issue #104](https://github.com/tseguine/grunkalunka/issues/104).
The browser-oracles workflow now declares `tools/browser-oracles/**` for
**both** PR and push triggers, alongside its existing test, documentation,
and workflow paths. The source-level regression tests protect these
trigger relationships. No browser tool or oracle semantics were changed.

With the new filter, the audit is expected to select the offline oracle,
collector, and three-engine smoke workflows for a tool-only browser change.
This describes workflow **trigger intent**, not confirmed execution or a
replacement for the real browser results.

This audit is a first step toward complete external CI attribution, not
a general YAML evaluator or a map of every optional workflow (P1, type
query, etc.). An eventually authoritative selective gate would also have
to verify the actual GitHub run conclusions on the tested merge revision,
account for transitive test/fixture inputs, and retain all applicable
browser-engine checks. No current PR skips canonical CI.
