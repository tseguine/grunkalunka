# A1 candidate: read-only note preview presentation contract

This is a bounded **presentation-only** preparation for [A1 pilot #54](https://github.com/tseguine/grunkalunka/issues/54)
and [steering #52](https://github.com/tseguine/grunkalunka/issues/52). It is not a
full application, HTML parser, DOM implementation, layout engine, renderer or
filesystem/vault grant. The user-visible first slice is a semantic HTML outline
of one explicitly supplied note, or an access-denied state.

```sh
node --test tests/applications/note-preview.test.mjs
```

The renderNotePreview function accepts a trusted host-supplied profile, original
UTF-8 byte array, typed heading records from the experimental Markdown WIT
interface (level, original start/end and section end, parent, owned label),
a protected backing-read count, and a visible heading count.

- The denied profile requires zero backing reads and no heading records.
  It emits an access-denied message and never prints the source note.
- The granted profile requires exactly one authorized backing read. Heading
  projection inputs are bounded by the existing 4,096-byte source,
  64-heading and 4,096-byte aggregate label quotas, with checked original
  source ranges, parent hierarchy and source order.
- Heading text is escaped for HTML text position, including replacement for
  U+0000. Source offsets remain byte offsets; escaping does not move them.
- A visible count of zero shows a collapsed state; larger values expose only
  the selected number of headings. This is an app presentation parameter,
  **not** authorization or evidence of source-script execution.

The standalone tests use independent expected HTML and malformed inputs.
They do **not** execute Wasm, source compilation, WIT calls, browser DOM,
CSS layout, event dispatch or build/package installation. Those are separate
explicit integration steps tracked in #54. No third-party dependency is added.
The existing scoped document check calls this test suite; browser-conformance
files, compiler/lowering, shared check runner and workflow remain unchanged.
