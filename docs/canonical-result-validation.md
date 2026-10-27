# Malformed canonical result checks

`cargo test -p grunkalunka-runtime --test canonical_result_validation --locked --offline`
exercises the pinned component runtime's validation of a private
`result<string, read-error>` boundary. The ordinary workspace checks include it.
This complements the valid document transport and capability-backed outline
experiments without depending on the Markdown parser or its adapter.

A small component owns fixed memory and returns a deliberately selected result
header. Valid UTF-8 is lifted into an owned String before post-return clears the
memory; an ordinary denied result is distinct from a transport failure. Positive
controls check both return values and an observable cleanup counter.

Negative cases cover invalid/truncated UTF-8, invalid result and error tags,
strings outside or crossing memory bounds, oversized string lengths, and
out-of-bounds or unaligned result headers. They must fail the outer component call,
not produce a successful policy-denial value. Each invalid instance is discarded;
no cleanup guarantee or post-trap reuse is asserted. A fresh valid instance still
runs afterward. Fixtures use fuel-enabled stores and fixed-size memories.

These tests are observations about the selected Wasmtime configuration, not a
new implementation of the canonical ABI, arbitrary-module hardening, or complete
transport/resource conformance. They add no host authority, dependencies or public
API. The format follows the [canonical ABI](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md).
