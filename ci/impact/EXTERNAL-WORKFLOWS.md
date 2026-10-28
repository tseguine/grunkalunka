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

## Concrete gap: tool-only browser edits

The offline `tests/browser-oracles/*.test.mjs` suite imports helpers from
`tools/browser-oracles/`, so it may need to rerun when those helpers change.
However, the `browser-oracles.yml` workflow's own PR path filter includes
`tests/browser-oracles/**` but does **not** include
`tools/browser-oracles/**`. The browser smoke and collector workflows *do*
trigger for those tool-only changes. The audit therefore emits
`requiredNotTriggered: ["workflow:browser-oracles"]` for tool-only edits.
That is an explicit *coverage gap*, not a passing test or a suggestion to
weaken the offline test. A future browser-owned workflow update could add
the missing path trigger, with its own tested PR and coordination.

This audit is a first step toward complete external CI attribution, not
a general YAML evaluator or a map of every optional workflow (P1, type
query, etc.). An eventually authoritative selective gate would also have
to verify the actual GitHub run conclusions on the tested merge revision,
account for transitive test/fixture inputs, and retain all applicable
browser-engine checks. No current PR skips canonical CI.
