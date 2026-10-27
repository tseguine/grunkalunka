# Capability-backed Markdown outline experiment

`node scripts/check-document-outline.mjs` builds the Rust `outline_guest` and
embeds its exact core-Wasm bytes in a private WIT component. The component imports
only an in-memory document capability. Its adapter obtains authorized text, feeds
the actual Markdown guest, and returns `result<list<heading>, inspect-error>`.
No parsing is reimplemented in the host or handwritten adapter.

Each heading carries level, original UTF-8 heading range, section end, optional
parent index and owned text. Duplicate/empty headings and skipped levels retain
the library's behavior. The native driver exercises absent, denied and permitted
profiles against the same component artifact. Ordinary denied, unknown-grant,
input-too-large and outline-too-large results leave the instance reusable. Read
counters distinguish denial from backing access, including after earlier success.
Fuel exhaustion remains a trap; that instance is discarded, then a fresh one runs.

The private adapter has one fixed 64 KiB transport memory. The imported read may
allocate one string of at most 4,096 UTF-8 bytes. Results contain at most 64 records
and 4,096 label bytes; records and labels occupy disjoint fixed transport regions.
The Rust guest has its own allocator and memory. Result limits are checked after
parsing and do not constitute a hard total-memory limit. Byte-at-a-time copying is
an experimental composition technique, not a performance recommendation.

On normal return, the adapter releases the canonical guest borrow. Wasmtime lifts
the list and strings into owned host values; a post-return callback then clears the
transport memory, resets guest input/results and increments a checked counter.
This happens for ordinary errors too. Retained host outputs are compared after
subsequent calls and destruction of the original store. Reset is not secure erasure
of Rust allocator memory, and cleanup/reuse after a trap is not promised.

The check compares eight real component projections with direct native-library
results, covering Unicode, combining marks, BOM/NUL, CRLF, empty/duplicate headings,
container headings, literal code and exact limits. It also exercises repeated
calls, denied-to-permitted transitions, oversized input/results, correction,
separate instances and fuel exhaustion. NUL-to-U+FFFD projection also tests exact
4,096-byte labels and overflow from a source below the input quota. The existing
CI hosts run the command.
Builds and driver invocations have outer watchdogs; no timing ratio is asserted.

This advances issue #27 but does not establish a general vault or filesystem API,
source-language document imports, or completion of A1. The host's private grant
representation is not cross-store authentication. The guest cannot choose paths;
texts and grants are selected by the trusted host. Existing resource experiments
cover broader handle ownership rules. No WASI, network, persistence, release or
deployment is added. See [text transport](document-transport.md),
[core outline guest](markdown-outline-guest.md), and the
[canonical ABI](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md).

The expanding-label golden follows [CommonMark 0.31.2, insecure characters](https://spec.commonmark.org/0.31.2/#insecure-characters).
