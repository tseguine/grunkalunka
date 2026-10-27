# Multi-document authority with the Markdown consumer

`node scripts/check-document-outline.mjs` also exercises the real outline component
against the private multi-document grant policy. This is an integration check, not
a new public grant service or source-language import.

The ordinary component artifact and compiled Markdown guest are unchanged. The
host initially contains two documents and issues only the first grant. A denied
profile returns a structured denial; a raw backing index cannot select the
ungranted document. Read counters remain empty in both cases. After authorization,
the returned outline retains Unicode labels, hierarchy and original byte ranges.
A relative path in the document remains inert text rather than a lookup request.

The host then grants the second document, revokes the first token and reissues a
new grant. Copied old tokens stay revoked; the new token returns the original
outline, while the second document remains independently readable. Profile denial
after earlier success still prevents reads. A token from another owner cannot
select or revoke a colliding representation, and is rejected before component
entry. The legitimate token in that owner continues to work.

For a negative control, a fresh instance receives only the first canonical borrow
while its host also owns a live second grant. Only the adapter's imported-call
operand is changed to substitute the second host representation. The exact same
Markdown guest bytes remain embedded. This must trap before host dispatch, not
return the second outline or an ordinary policy error. The trapping instance is
discarded; no post-trap cleanup or reuse is asserted.

Every ordinary component result checks post-return cleanup. Previously returned
host-owned lists remain usable after other calls, revocation and store destruction.
The existing native/component parity, quotas, malformed-transport and fuel checks
remain separate evidence; these authority checks do not replace them.

The host and its wrapper are trusted. These finite in-memory fixtures are not
filesystem confinement, a serialized grant format or an arbitrary-component
security certification. They establish the bounded document-authority integration
in #27 only when their exact revision has passed the canonical CI checks.
