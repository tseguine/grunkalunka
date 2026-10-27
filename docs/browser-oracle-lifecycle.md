# Synthetic DOM listener-lifecycle oracle

`tests/browser-oracles/listener-lifecycle.html` is an original, standalone
file-origin fixture. Four scenarios use fresh detached targets and two synthetic,
cancelable, bubbling events per target. The single nested case dispatches another
synthetic event synchronously. No trusted user input, shadow tree, default action,
layout, animation or external network is involved.

The independent exact verifier in `tools/browser-oracles/listener-lifecycle.mjs`
checks **both** per-scenario dispatches, listener order, nested/outer identity,
Event flags, dispatch return values, post-dispatch cleanup and complete provenance:

| Case | Normative distinction |
| --- | --- |
| Remove during dispatch | A removed later listener is skipped even in the current invocation |
| Add during dispatch | A new listener is not in the current invocation's cloned list; it appears next dispatch |
| Nested `once` | Removal precedes invocation; the nested dispatch does not re-invoke the once listener |
| Passive cancel | `preventDefault()` in a passive listener cannot cancel the event |

These derive from the WHATWG [listener-invocation algorithm](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#concept-event-listener-inner-invoke),
[add/remove algorithms](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#concept-event-listener),
and [cancellation algorithm](https://dom.spec.whatwg.org/commit-snapshots/3c53f93a3e1fb11ef5bd38029b7a9c97baaf0177/#concept-event-canceled-flag).
No engine majority vote supplies the expected trace. Source mutations separately
omit removal, omit addition, disable `once` and disable `passive`. Each must pass
its **entire independent alternate expectation**, with its own fixture hash, and
fail the baseline at the intended scenario/dispatch. Synthetic receipts in unit
tests prove verifier sensitivity but are not browser execution evidence.

Run offline verifier checks with
`node --test tests/browser-oracles/listener-lifecycle.test.mjs`.
The managed Chromium, Firefox and WebKit smoke exercises the fixture with fresh
instances, retains source HTML and raw JSON receipts under `listeners/`, and fails
if an engine does not satisfy the checked expectations. Source hash, browser
version, host, Node/Playwright versions, viewport and file URL remain recorded.
The separate layout cross-engine report is **not** a listener verification report.
Agreement between managed engines is not WPT coverage or broad DOM conformance;
these are four narrowly selected reference observations, not a DOM implementation.
