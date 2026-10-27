# Native and core-Wasm Markdown parity

Run `node scripts/check-markdown-portability.mjs` after fetching the locked Cargo
packages and installing `wasm32-unknown-unknown`. The canonical check runs it too.
The script builds the same fixed-fixture Rust example for the native host and
core Wasm, executes both, and compares the complete projected bytes, not hashes
of the results. No parser implementation or production library API is changed.

Six fixtures cover empty input, CommonMark headings/links, skipped/duplicate
heading levels, image labels inside links, container headings, fenced/indented
code, raw HTML, references, inert unsafe destinations, Unicode, BOM and mixed
CR/LF/CRLF. Each projection includes summaries, outlines, code blocks, prose,
line spans, every source-byte position and its valid inverse conversion.

The native process emits one hex-encoded record per fixture. Wasm exports select
one of the same fixed fixtures and expose its result through bounds-checked byte
access. The private probe protocol accepts no caller pointers or arbitrary source;
it is not a public guest API. Rust's required `unsafe(no_mangle)` annotations name
only these unique probe exports; no unsafe memory operations are introduced.

The checker denies all imports, repeats every projection, checks invalid fixture
and byte offsets, and demonstrates separate instance storage and owned host output
surviving subsequent renders. Native compilation/execution subprocesses have a
three-minute watchdog. Wasm runs in a disposable worker with a 30-second watchdog
and awaited termination. This is not a hostile-input or memory-isolation test.
Debug text is compared
only within one checkout/toolchain, not adopted as a stable serialization format.

Native correctness remains covered by the library's existing tests; agreement
between two builds cannot prove independent CommonMark conformance. This check
establishes the exercised library behavior in Node's Wasm engine on the CI hosts,
not browser UI support, Wasmtime coverage, a WIT component, JS source lowering,
performance, or a complete application. Build outputs stay in Cargo's target
directory and are neither uploaded nor released. `CARGO_TARGET_DIR` is respected.

See [Markdown inspection](markdown.md) and Rust's
[`wasm32-unknown-unknown` target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).
