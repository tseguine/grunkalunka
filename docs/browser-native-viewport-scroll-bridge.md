# Source-checked native viewport scrolling versus real browsers

The existing [native block observation bridge](browser-native-block-bridge.md)
now includes a **page-level viewport scroll** witness alongside its original
geometry, hide/restore, hit-target, percentage-height and border-box channels.
It compares the implemented `hit_test_scrolled` API with real DOM
`window.scrollTo`, `getBoundingClientRect` and
`document.elementFromPoint`, without borrowing browser calculations to
construct the Rust input.

## Selected fixture and expected evidence

The independent HTML source appends a 650px-tall spacer and a 80×30px block
after the earlier fixed-height groups. It reads the actual
`document.scrollingElement.scrollHeight`, then samples one viewport point
at (10, 590) before scrolling, after `window.scrollTo(0, 350)`, and after
restoring scroll to zero. The exact browser scroll offsets, target border
rectangles and normalized `elementFromPoint` target IDs are retained in
the original source-hashed `scrollViewport` receipt.

The native Rust example reconstructs the earlier groups' reviewed occupied
heights as separate fixed block inputs, then lays out the spacer and target.
It independently asserts a 958px document scroll height, the target's
**document-space** border rectangle (0, 928, 80, 30) and the following
viewport observations:

| Phase | scrollY | Target rectangle y | Hit at (10, 590) |
| --- | ---: | ---: | --- |
| Before | 0 | 928 | None |
| Scrolled | 350 | 578 | `scroll-target` |
| Restored | 0 | 928 | None |

Only the final node is caller-targetable in this selected Rust hit-test
policy. The comparator requires complete `scrollViewport` data in both
sources, exact finite rectangle fields, nonnegative integral scroll offsets
and a known target/null result. It compares offsets, document extent, target
rectangles and hit IDs separately; missing or malformed observations fail
closed. The existing comparator continues to check the original fixture
SHA-256 and viewport/DPR provenance.

## What this proves, and what it does not

The existing `browser-native-bridge.yml` independently executes the same
native example and fresh Chromium, Firefox and WebKit observations. It does
not add another Actions workflow, browser matrix or conditional bypass.
Ordinary offline regression tests prove missing/malformed receipts and
controlled target/geometry mismatches are detected, but are not substitutes
for real-engine execution.

This is one document-level scroll offset and one selected hit point under
normal block flow. It does not implement native scroll events, scrollbars,
nested overflow containers, smooth scrolling, layout/visual viewport
distinctions, compositing, overscroll, scroll anchoring or general browser
scroll conformance. Reproduce it using the commands in the
[native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser).
