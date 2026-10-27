# Caller-supplied Markdown in a core-Wasm guest

`node scripts/check-markdown-outline.mjs` builds `outline_guest` for native and
`wasm32-unknown-unknown` and compares owned outline records for supplied text.
It runs in the canonical checks. This is the parsing prerequisite of issue #27,
not yet a WIT component or an authorized document reader.

The example calls the unchanged `grunkalunka_markdown::outline` library. Its
private scalar adapter avoids accepting caller pointers or exporting Rust layouts.
`outline_begin` clears input/results; `outline_push` accepts one byte; and
`outline_finish` validates UTF-8 and computes an outline. Successful finish returns
the heading count. `outline_field(index, field)` selects level, heading start/end,
section end, parent index, or projected-text byte length for fields 0 through 5.
Section start equals heading start. Parent -1 means no parent. The text-byte getter
returns individual UTF-8 bytes. Invalid queries return -1. These coordinates and
parent indices retain the library's source-snapshot and navigation-only contract.

Input is capped at 4,096 bytes, results at 64 headings and 4,096 total label bytes.
Finish errors are -1 for input overflow, -2 for invalid UTF-8, -3 for call-order or
byte-value violations, and -4 for result overflow. Failures discard results and
remain sticky until begin: ignoring an overflowing append cannot turn a source
prefix into a successful document. Finish is single-use; appending afterward
invalidates the result. Ordinary failures recover after reset. Neither limits nor
errors change the general library API. Parsing/allocation occurs before the
result-size check; these limits are not a hard total-memory bound.

Each Wasm instance owns its buffers. The native oracle calls the library directly,
not the exported scalar getters, and emits numeric metadata plus hex-encoded text.
Tests compare exact records, including Unicode/CRLF, empty labels, duplicate
headings, container headings, literal code and exact limits. Fixed coordinate
expectations additionally check the source layout. Repeated calls, sticky errors,
copy ownership and independent instances are exercised. A corrupted expected
record must fail comparison. The Node worker has a 30-second watchdog and is
terminated before settling; builds/native invocations have separate watchdogs.

No imports, WASI, filesystem access or resource resolution exist in the core guest.
Native stdin is only a comparison driver. Byte-at-a-time transport is an explicit
experiment boundary, not a performance design or stable platform ABI. A later WIT
adapter must supply document authority, typed list/results and post-return cleanup.
No artifact is published, uploaded or deployed.
