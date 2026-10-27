# Shadow boundary dispatch and retargeting oracle

`tests/browser-oracles/shadow-events.html` is an original, self-contained
light-DOM/shadow-DOM fixture. It builds eight **detached** `outer → host → shadow
root → inner` trees: open/closed roots, composed/scoped events, and bubbling or
nonbubbling synthetic dispatch. An independent verifier in
`tools/browser-oracles/shadow-events.mjs` supplies exact expectations that the
browser never receives. No browser majority vote supplies the expected results.

The selected cases exercise three different DOM event distinctions:

- **Boundary:** Non-composed dispatch ends at the shadow root; composed dispatch
  crosses the boundary to host and outer listeners. The target is exposed as
  `inner` inside the root and retargeted to `host` outside it.
- **Closed-root visibility:** For a composed event, a closed shadow root hides
  its `inner`/`shadow` path entries from listeners outside, while its own
  listeners see the complete path. Open roots expose those entries everywhere.
- **Nonbubbling composed events:** Outer/host capture still runs. Host listeners
  observe `AT_TARGET` phase on both capture and bubble registrations; the shadow
  root's bubble listener and the outer bubble listener do not run.

Each case checks every listener's identity, target, currentTarget, phase, event
flags and `composedPath()`, as well as the dispatch result and post-dispatch
reset state. In these detached non-cancelable tests `dispatchEvent` returns true.
After scoped dispatch, the event's `target` is cleared; after composed dispatch,
its retargeted `target` remains `host`. `currentTarget`, phase and composed path
are reset in both cases.

Source-derived expectations follow the WHATWG DOM Standard's
[dispatch](https://dom.spec.whatwg.org/#concept-event-dispatch),
[retarget](https://dom.spec.whatwg.org/#retarget),
[composed path](https://dom.spec.whatwg.org/#dom-event-composedpath), and
[listener invocation](https://dom.spec.whatwg.org/#concept-event-listener-inner-invoke)
algorithms; the page records observations but imports no expected-value table.

The separate runner saves immutable exact HTML snapshots and raw JSON receipts
before checking each browser. Fresh-instance repeats must be stable. Three
guarded source mutations each require **complete alternate independently
specified traces** and rejection of the baseline at the designated case:
`skip-composed` and `skip-bubble` target `open-composed-bubble` while
`open-closed-root` targets `closed-composed-bubble` and exposes otherwise-hidden
path entries. A changed hash, unrelated malformed observation, or missing engine
is not accepted as semantic sensitivity.

Use `node --test tests/browser-oracles/shadow-events.test.mjs` for offline
contract tests. Actual managed Chromium/Firefox/WebKit smoke must separately
exercise source provenance, full traces, repeat and mutations through the
existing optional production file-origin collector. The separate joined layout
report remains a diagnostic for its original layout fixture, **not** shadow-DOM
conformance proof. Host/browser versions and source digests remain in receipts.

These eight synchronous synthetic scenarios are a narrow reference target, not
full shadow-DOM conformance, slots, relatedTarget, trusted user input, default
actions, async event-loop sequencing, WPT execution, or native runtime support.
