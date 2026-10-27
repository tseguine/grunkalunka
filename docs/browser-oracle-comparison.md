# Browser observation comparison (experimental)

The browser oracle is deliberately separate from the Grunkalunka compiler,
private IR, language semantics and renderer implementation.

## Validate and compare two captures

Use the identical HTML fixture, viewport CSS pixel dimensions, device pixel
ratio, and source revision in each browser. Save each page's JSON observation
payload without rewriting it.

```sh
node tests/browser-oracles/compare.mjs blink.json gecko.json
node tests/browser-oracles/compare.mjs blink.json webkit.json --rect-tolerance=0.25 --style-px-tolerance=0.1
node --test tests/browser-oracles/compare.test.mjs
```

The comparator accepts two version-1 browser-oracle capture JSON files.
Its output is a machine-readable JSON report containing both unchanged
captures, detected discrepancies, environment incompatibilities, and the
explicit CSS-pixel tolerances used in the comparison. Standard output is
reserved for the JSON report; parse and usage errors go to standard error.

Exit codes:

| Exit | Meaning |
| --- | --- |
| 0 | Structurally comparable captures; no observed differences at requested tolerances |
| 1 | Comparable captures; one or more observed differences |
| 2 | Invalid inputs or incompatible fixture/viewport/device scale, or CLI failure |

A report with `status: equal` is **differential agreement**, never proof of
conformance to an HTML/CSS standard. Browser-engine consensus can be wrong.

The comparator deliberately does *not* compare browser identity, timestamp,
or document URL for equality, since those are expected to differ between
independent engines. These values remain in the original captures. It does
require identical fixture ID, observed element IDs in order, viewport width
and height, and device pixel ratio. The fixture ID is caller-supplied and
does not independently prove identical HTML/CSS bytes; source hashing is a
follow-up.

All computed-style values are compared exactly as strings unless both match a
numeric `px` length and a nonnegative `--style-px-tolerance` is supplied.
Bounding rectangles are compared as numbers in CSS px with the optional
`--rect-tolerance`. The defaults are both **zero**. Missing computed
properties, malformed values and unobserved data cannot silently become
zero-valued properties. Comparisons are per field; unmatched observed element
IDs are reported as incompatible, not compared as if they were the same DOM
node.

This is not a rendering engine runner. The initial captures come from
opening the source HTML fixture in a real browser. A later collector should
automate pinned Blink/Gecko/WebKit versions and record engine, operating
system, fonts and provenance. Browser collectors must not copy the reference
engine's DOM/layout architecture into Grunkalunka.
