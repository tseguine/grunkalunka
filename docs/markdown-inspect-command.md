# Native Markdown inspection command

The `grunkalunka-markdown-inspect` binary is a small consumer of the standalone
[Markdown library](markdown.md). Build the workspace, then pass one explicit file
path or `-` for standard input:

```sh
cargo run -p grunkalunka-markdown --bin grunkalunka-markdown-inspect --locked --offline -- README.md
target/debug/grunkalunka-markdown-inspect README.md
target/debug/grunkalunka-markdown-inspect - < README.md
```

Add `.exe` to the executable path on Windows. `--help` prints usage; a literal file
named `--help` or `-` can be selected with a path such as `./--help`. Relative paths
use the caller's working directory. There is no directory traversal, glob expansion,
network lookup, renderer, Wasm execution or application/vault permission model.
Only the requested file or input stream is read, and source files are never changed.

Output inventories headings, links, images and code blocks in separate sections,
with source order within each section. Heading/link/image rows retain the original
half-open UTF-8 byte ranges supplied by the library. Labels, destinations, titles
and code-block info use Rust debug-string escaping so embedded control characters
do not create raw terminal controls or extra report lines. This is a human-readable
inspection format, not JSON or a stable machine-readable schema; structured
consumers should call the library APIs.

Code-block bodies are not printed or executed. Their `text-bytes` field measures
the parser-projected body's UTF-8 length, not the original source range's length.
Reference definitions resolve only inside the source. Destinations, including
unsafe schemes, remain inert data. Existing CommonMark defaults and disabled
extensions are unchanged; raw HTML is not parsed into link/image records.

Input is strict UTF-8, limited to 1,048,576 bytes inclusive. The reader takes at most
one byte beyond that limit to detect excess input before parsing or writing a
report. BOM and line endings are passed unchanged to the library. The limit is a
local command policy, not a new library limit or total CPU/memory/deadline guarantee.
Blocking input streams can still wait for data. Invalid arguments, read/UTF-8/size
errors and output failures exit nonzero; validation failures emit no partial report.

`cargo test -p grunkalunka-markdown --locked --offline` exercises the actual binary
with stdin, Unicode file paths, metadata/ranges, escaped controls, help/errors and
exact byte-limit boundaries. The existing canonical cross-platform checks include
these tests and build the executable without publishing or uploading it.
