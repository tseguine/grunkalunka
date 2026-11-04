# Native outline control for A1 sessions

The private `a1_note_scene` executable accepts an opt-in `A1SCENE2` input for
sessions that need to collapse and reopen an authorized outline. The previous
`A1SCENE1` input keeps its original title, heading targets and output behavior.
Neither protocol is a public WIT view ABI or an authorization token: the caller
must already have the document grant and validated, owned heading records.

## Input

The first line is seven tab-separated fields, followed by the same visible
heading rows as `A1SCENE1`:

| Field | Contract |
| --- | --- |
| Tag | `A1SCENE2` |
| Profile | `granted` or `denied` |
| Source length | UTF-8 byte length, at most 4096 |
| Visible heading count | Number of rows that follow, at most 64 |
| Backing reads | One for a granted document, zero for a denied document |
| Total heading count | Authorized heading count, including hidden rows, at most 64 |
| Expanded | Exactly `0` or `1` |

An expanded scene includes all authorized headings; a collapsed scene includes
none. An empty document uses expanded `0`. A denied scene has zero source length,
visible headings, backing reads, total headings and expansion. Contradictory
state, malformed rows and quota violations fail before emitting a hit or frame.
Collapsed input carries no hidden labels; no input contains the original note
bytes or filesystem path.

## Control and output

A granted, nonempty scene replaces the title with a native `Hide outline` or
`Show outline` control. Both labels occupy the same 304 by 24 box at `(8,8)` in
the 320 by 160 frame, so the same pointer coordinate can collapse and reopen the
outline. Hit bounds include the top and left edges and exclude the bottom and
right edges. Heading row geometry remains unchanged.

For these scenes, `--hit X Y` emits exactly `A1HIT2` followed by a tab and either
`toggle` or `none`, then a newline. Only the control is targetable. Heading rows,
the collapsed panel, a denied document and a document with no headings are
inert. A control hit has its own identity and never pretends to be heading zero.
Empty documents show `No headings.` and denied documents retain their denial
message.

`--report` retains the `A1FRAME1` layout report, including the actual control
label and geometry. `--ppm` emits native RGB pixels using the existing bitmap
text service. A control hit identifies intent only; the application must still
invoke its compiled source handler, validate the service result and publish a
successfully painted next frame before changing session state.

The existing `tests/applications/note-preview.test.mjs` suite exercises both
protocols, control boundaries, inert denied/empty scenes, native pixels and
contradictory-state rejection through the real executable. Its existing
canonical check and A1 impact dependencies cover this addition.
