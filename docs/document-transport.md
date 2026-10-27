# Recoverable document-text transport

`cargo test -p grunkalunka-runtime --test document_transport --locked --offline`
exercises a private WIT resource whose read method returns
`result<string, read-error>`. This is the transport prerequisite of issue #27,
not a Markdown-processing guest, vault API or source-language import feature.
The normal workspace checks include the tests; no new dependencies are required.

A host explicitly links the document interface. It validates its policy and one
private grant representation before copying a document. Denied, unknown-grant and
too-large requests return ordinary enum values inside the WIT result. They are
not host traps. The same instance can receive a denial, gain permission, read a
document, reject an oversized revision and then read a corrected revision.
Attempt/success counters distinguish rejected requests from returned text.

The guest adapter forwards the canonical result; it does not parse document text.
The fixture owns one fixed 64 KiB memory with no growth. Its intentionally narrow
allocator accepts only one UTF-8 result allocation per call, at most 4,096 bytes,
and rejects other allocation requests. This is sufficient for these calls, not a
general-purpose allocator. A normal post-return callback clears the result area
and text storage, releases the allocation slot and increments an observed counter.
The adapter explicitly drops the canonical guest borrow on normal return,
including ordinary error returns. Host-owned documents are not transferred.

Tests observe post-return cleanup for both success and error values. Owned host
strings retain their bytes after cleanup and subsequent reads. Empty strings,
Unicode, BOM, combining marks, NUL and mixed line endings are transported without
normalization. The inclusive text limit counts UTF-8 bytes, not characters; a
4,096-byte string succeeds and a 4,097-byte revision returns `too-large` without
silently truncating or replacing it with an older document.

Injected unexpected host failures remain traps, not fabricated document results.
Those instances are discarded; cleanup or reuse after a trap is not promised.
Stores have fuel before instantiation, but this is not a hostile-module sandbox
or a bound on compilation, all host memory or blocking host operations. The
private representation check is not cross-store grant authentication; that
separate question is covered by the existing resource-policy experiments.

No filesystem, network or WASI is linked. The host texts are supplied in memory.
This deliberately establishes result/string ownership and recoverable policy
failure before adding a real document-processing guest or general allocation.
See [capabilities](capabilities.md) and the [WIT result type](https://component-model.bytecodealliance.org/design/wit.html#results).
