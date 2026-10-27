# Source-checked three-engine reports

After collection, compare the original self-contained HTML source and three
captures in Chromium, Firefox, WebKit order:

```sh
node tests/browser-oracles/cross-engine.mjs snapshot.html chromium.json firefox.json webkit.json > report.json
node --test tests/browser-oracles/cross-engine.test.mjs
```

Use `-` for an unexecuted engine; a missing file is also reported as missing.
Malformed JSON or receipts are invalid, not absent. The CLI emits the other
available observations even when an engine is missing or invalid. Source/usage
errors go to stderr. Inputs are trusted local files, not a hosted upload service.

The CLI independently hashes the supplied source bytes. Every capture must name
that hash, its expected engine/family, the collector and versions, the host OS and
architecture, and requested viewport/DPR consistent with its observations.
Matching fixture IDs without matching source receipts are insufficient. All
three pair slots are always present; unavailable pairs are `not-compared`.
The two usable engines can still provide a pair result when the third is absent.

Before comparing, each pair must agree on recorded Playwright/Node version,
host OS/architecture, fixture ID, observation identities, viewport and scale.
Engine versions, user agents, capture timestamps and URLs may differ and remain
in the raw inputs. Comparisons are exact: this report does not expose tolerance
flags, normalize CSS strings or vote for a winning engine. Field-level differences
retain both values. `ready` means eligible for these checks, not standards-correct.

Exit 0 means all three engines and pairs were comparable and their reported fields
matched; exit 1 means complete comparable evidence with observed differences.
Exit 2 means incomplete, invalid or incompatible evidence. The report preserves
per-engine errors and all pair results even when the overall status takes the
precedence invalid, incomplete, incompatible, different, equal. A difference is
not automatically a conformance failure; callers must distinguish these outcomes
rather than treating every nonzero exit as a broken browser.

**Limits:** a matching receipt is a consistency check, not authenticated proof of
what a browser loaded. The hash covers the HTML file, not external assets. Use
self-contained fixtures until asset inventories exist. Recorded OS/architecture
labels do not establish identical fonts, OS images, browser patches or rendering
configuration. Missing observations in all three captures cannot be discovered
without an independent required-observation manifest; the baseline smoke's verifier
provides that separate check for its fixture. Run that verifier before reporting
its artifacts. This generic report makes no new normative claims and explicitly
records `conformance: not-assessed` even when every engine agrees.

No browser installation is needed to replay saved evidence. The existing focused
oracle workflow discovers the new tests; compiler and runtime checks are unchanged.
