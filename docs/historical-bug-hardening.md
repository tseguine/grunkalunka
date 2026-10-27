# Historical browser and embedder bugs as regression-hardening inputs

**Status: proposed test-design technique, not an assigned roadmap milestone or a
standing bug-mining task.** The maintainer explicitly selects priorities and
workstream ownership under [AGENTS.md](../AGENTS.md). Recording this idea does
not direct agents to leave their current work, start bulk issue mining, or adopt
an external product's entire API as a compatibility target.

## Idea

Search *public* resolved and open browser/embedding issue trackers for
reproducible failures that could also affect an **already implemented**
Grunkalunka contract. Translate the underlying failure scenario into a small,
independently justified regression. This lets our implementation learn from
other engines' historical mistakes without copying their architecture.

Prefer **resolved bugs with a minimal reproducer, an explained fix and an
upstream regression test**: those usually offer better evidence than an
unconfirmed open report. Open reports may still identify useful edge cases,
but are hypotheses until their expected behavior is established separately.

### Candidate sources and overlapping product surfaces

| Public source | Candidate failure surfaces | Relationship to engine comparison |
| --- | --- | --- |
| [Chromium issues](https://issues.chromium.org/) | Blink layout, DOM and input; V8/JavaScript semantics; scheduling, memory and isolation | Chromium/Blink/V8 reference family |
| [Mozilla Bugzilla](https://bugzilla.mozilla.org/) | Gecko layout/DOM; SpiderMonkey execution; event ordering, lifecycle and security boundaries | Independent Gecko/SpiderMonkey family |
| [WebKit Bugzilla](https://bugs.webkit.org/) | WebCore CSS/layout/DOM; JavaScriptCore; text, event and process lifetime | Independent WebKit/JavaScriptCore family |
| [Electron issues](https://github.com/electron/electron/issues) | Embedded application windows, navigation, main/renderer IPC, preload/capability boundaries, lifecycle and Node integration | Chromium-based; useful **embedder** reports, **not another Blink vote** |
| [CEF issues](https://github.com/chromiumembedded/cef/issues) | Native Chromium embedding, off-screen rendering, input translation, window lifetimes, message loops, resource callbacks and GPU surfaces | Chromium-based; not another independent engine |
| [Ultralight issues](https://github.com/ultralight-ux/Ultralight/issues), [AppCore issues](https://github.com/ultralight-ux/AppCore/issues) | Lightweight/off-screen embedding, view/event lifecycle, host resource resolution, painting and platform input/windowing | WebKit-derived with JavaScriptCore; distinct integration choices, **not an independent WebKit-family vote** |

The three embedding products are included because Grunkalunka aims to support
both embeddable application environments and standalone browsing. Reports about
their *host/renderer interface* may matter even when standard browser-layout
oracles do not expose the defect. Do not conflate a shell/embedding bug with a
JavaScript or CSS specification failure.

## Triage procedure for a candidate issue

1. **Pin provenance.** Record the upstream URL and issue ID; reported product,
   affected/fixed versions or commit (if known); reproducer; expected and
   observed behavior; and report status. An issue may be stale, disputed or
   specific to a configuration.
2. **Reduce to an invariant.** Identify what failed: e.g. visibility changes
   must invalidate hit targets, a captured value must survive suspension, a
   stale capability must not regain authority, or an input must not overflow
   a layout bound. Do not rely on the upstream patch as our design.
3. **Establish the expectation independently.** Check the relevant specification,
   our admitted contract/ADR and any authoritative upstream fix or regression.
   Cross-engine agreement is evidence, not a standards proof. Some reports
   describe implementation quirks or deliberate embedding limitations.
4. **Check relevance against current `master`.** Inspect the actual source,
   tests and relevant issue/PR before proposing work. Classify the case as:
   **covered by an existing test**, **relevant and untested**, **excluded by a
   demonstrated invariant**, **not applicable**, or **future/unsupported
   feature**. Do not call a feature supported because its rejected test passed.
5. **If a relevant missing case is in an already delegated goal**, propose the
   smallest test with a meaningful independent oracle. Where practical, show
   fault sensitivity with a deliberately broken source or fixture; preserve
   original input, expected observations and exact executed revision. Keep
   reference-browser tests distinct from our own implementation tests.
6. **Record only actionable evidence.** Link the upstream issue and our
   fixture, PR or bounded steering finding. Note what actually executed in
   Grunkalunka and what remains conjectural. Do not create a giant imported
   bug database, automatic compatibility percentage or repetitive issue churn.

A potential bug in a **currently owned** implementation belongs with that
workstream's existing PR/issue or a narrowly evidenced `steering:` issue.
An issue for an **unassigned future feature** is only a candidate for
maintainer prioritization, not permission to begin implementing it.

## Examples of worthwhile candidate regressions

- **Headless renderer:** repeated `display:none`/show and reflow should not
  leave stale geometry, painted pixels or hit targets. Test only whichever
  stages are actually implemented; a borrowed semantic tree is not a DOM.
- **Compiler/runtime:** callbacks, captured cells, returned values and async
  reaction ordering should retain the admitted JavaScript evaluation
  semantics across suspension and rebinding.
- **Capabilities/resources:** revocation, reused handle indices and failure
  cleanup must never silently select another authority grant.
- **Embedding (later):** closing a view during input, cancellation or a pending
  host callback should respect explicit ownership and lifecycle boundaries.

These are *search themes*, not claims that Grunkalunka currently has those
bugs, and not instructions to build the still-unsupported features.

## Reuse, safety and reporting boundaries

Use public reports only. Prefer **original, minimal reproductions** and cite
the source; inspect the license and provenance of any copied upstream tests,
code or assets before importing them. A published issue does not give carte
blanche to redistribute third-party material or proprietary Ultralight code.
Sensitive security reports warrant focused boundary tests, not wholesale
replication of weaponized exploits in CI.

Keep resource limits and actual run evidence visible. A pass in Chromium,
Firefox, WebKit, Electron, CEF or Ultralight does not imply Grunkalunka passes.
Conversely, an issue excluded by our architecture needs a concrete invariant
and evidence, not a claim that Rust makes the entire bug family impossible.

No crawler, recurring sweep, new CI matrix, service connection, new product
compatibility promise or workstream assignment is created by this proposal.
