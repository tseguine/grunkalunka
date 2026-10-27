# Cascade and live DOM-mutation oracle

`tests/browser-oracles/cascade-mutations.html` is an original self-contained
fixture, not an imported WPT case. An empty block avoids font-dependent content.
Its eight eager snapshots record computed styles, border rectangles, class names
and inline property values/priorities. The browser receives no expected manifest.
The verifier in `tools/browser-oracles/cascade.mjs` supplies independent values:

| Phase | Content width (CSS px) | Reason |
| --- | ---: | --- |
| initial | 40 | Later equal-specificity important rule wins |
| normal-inline | 40 | Important stylesheet rule beats normal inline width 100 |
| important-inline | 64 | Important inline width beats important style rules |
| remove-inline | 40 | Removing inline width reveals the stylesheet result |
| selected-class | 52 | More-specific important rule beats later `.box` rules |
| remove-selected | 40 | Removing the class restores the less-specific rule |
| remove-box | 80 | Only the normal ID width remains applicable |
| restore-box | 40 | Adding the class restores important stylesheet declarations |

The content-box block has padding 2 and border 1 on each side: border width is
content width + 6, height is 16, and x/y are zero. Rules derive from
[CSS cascade sorting](https://www.w3.org/TR/css-cascade-5/#cascade-sort),
[CSSOM property updates](https://www.w3.org/TR/cssom-1/#dom-cssstyledeclaration-setproperty),
[DOM classList](https://dom.spec.whatwg.org/#dom-element-classlist), and
[CSS box sizing](https://www.w3.org/TR/css-sizing-3/#box-sizing).
Queries observe each synchronous mutation; they make no paint-timing claim.

Run `node --test tests/browser-oracles/cascade-mutations.test.mjs` offline.
The existing real-engine smoke also runs this fixture through its production
collector twice in fresh browser instances. It then captures three separate
source mutations: reverse rule order, remove the two `.box` important flags,
and skip adding `selected`. Each must yield its independently expected changed
width and fail the baseline at the relevant computed-style phase. A mere source
hash mismatch or unrelated malformed receipt cannot satisfy that check.

Evidence lives under the smoke output's `cascade/` directory: original and
mutated source snapshots, raw receipts and a completion summary. Existing
artifact retention and the 240-second whole-smoke watchdog apply unchanged.
Diagnostic JSON remains hidden. The baseline/repeat/diagnostic-isolation checks
are not replaced. No compiler, runtime, renderer implementation or root dependency
is involved. Eight selected transitions and three sensitivity probes do not
establish full cascade, CSSOM, DOM, or browser conformance; no majority vote is used.
