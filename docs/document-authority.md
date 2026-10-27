# Multi-document authority experiment

`cargo test -p grunkalunka-runtime --test document_authority --locked --offline`
uses the existing document string-transport component with a private multi-document
host. Ordinary workspace tests discover it; it does not depend on the outline
component integration in #31.

The trusted host supplies an in-memory collection, then issues opaque tokens that
map distinct grant representations to selected document indices. Borrowing does
not use a path or a document index. Tokens retain an owner allocation; the calling
wrapper checks its identity before creating a canonical borrow. The host method
separately checks profile and live-grant membership before copying document text.
Revocation removes a grant, not its document or other grants. Cloned tokens share
revocation; checked monotonic IDs never wrap or get reused, even after exhaustion.

Five tests cover recoverable denial, an ungranted backing index, exact returned
text, interleaved documents, revocation/reissuance, copied tokens, foreign owners
with colliding local representations, and terminal ID exhaustion. Normal component
returns check post-return cleanup, including ordinary policy errors. A retained
host string remains valid after subsequent calls and store destruction.

A separate negative case changes only the existing reader's imported-call operand:
the guest substitutes another live host grant's representation for its sole passed
canonical borrow. It must trap with an unknown handle before host dispatch. The
unaltered reader first demonstrates both documents with their actual grants. The
trapping instance is discarded; post-trap cleanup/reuse is not asserted.

This is a private policy fixture, not a public grant service, filesystem confinement,
or arbitrary-component security certification. The host is trusted; it could bypass
its wrapper. Paths in text are inert data and never resolved. Collections and test
sequences are finite; no aggregate-memory or blocking-I/O budget is established.
This advances #27's authority evidence but does not yet apply this host policy to
the actual Markdown outline consumer. No source-language import or A1 completion
is claimed. See [document transport](document-transport.md) and the separate
[canonical handle and host-identity experiments](capabilities.md).
