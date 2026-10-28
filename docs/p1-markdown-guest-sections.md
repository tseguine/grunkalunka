# Markdown guest cross-host Wasm section identity (P1)

## Observed motivation

The exact-head canonical CI for [PR #98](https://github.com/tseguine/grunkalunka/pull/98)
executed the real Markdown/WIT guest tests successfully on Linux x64, Windows
x64, and native Apple Silicon macOS ARM64. All three hosts reported Rust
**1.99.1 (58cf8f9c3, 2026-10-13)** and used the common
`wasm32-unknown-unknown` guest target, but the **guest files differed**:

| Host | Observed guest SHA-256 |
| --- | --- |
| Linux x64 | `8ea7c9707116d1cf756069a792721eae4d152dd62eca0b1a24cded78b24550df` |
| Windows x64 | `1b8d39fbe009e69aa311ebb0f868c0f0ee8e105f376546c8284e7a1e071e2144` |
| Apple Silicon macOS ARM64 | `c34f0cee5bb7d8f2a9f126bb0abfe07331b567ccf4de997d2372b08eef12a678` |

These are **same-workflow checkout observations**, not retained artifacts.
The three native Grunkalunka compilers separately emitted byte-identical
614-byte GCD core Wasm in that same PR; they use a different compilation
pipeline. Neither result implies a platform-specific guest behavior defect
or compiler nonconformance.

## Bounded section-difference experiment

The [optional native-host workflow](../.github/workflows/p1-markdown-guest-sections.yml)
builds the **existing unchanged Rust `outline_guest`** independently on
Linux x64, Windows x64 and Apple Silicon ARM64, using locked Cargo
dependencies and the same `wasm32-unknown-unknown` target. It runs the
same source scanner and synthetic negative tests on all three hosts.
It does not change Rust sources or compiler flags to force equal output.

The [section fingerprint tool](../tools/bench/wasm-section-fingerprint.mjs)
walks the bounded core-Wasm envelope and records SHA-256 and byte length for
each raw section payload. It distinguishes named **custom** sections from
standard Wasm sections such as code and data; it does not interpret
instruction semantics or claim that different bytes imply different behavior.

To reproduce after building the Rust target:

```sh
node --test tools/bench/tests/wasm-section-fingerprint.test.mjs
node tools/bench/wasm-section-fingerprint.mjs inspect \
  target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm > native.json
node tools/bench/wasm-section-fingerprint.mjs compare linux.json native.json
```

The hosted Linux job transfers only its bounded **JSON fingerprint receipt**
through ephemeral GitHub job outputs, not a guest binary or release artifact.
The receiving hosts compile independently and compare their own section
digests to the Linux receipt. Changed standard sections and changed custom
sections are reported separately, along with actual native Rust/Node host
identities, installed Rust version, Git revision, and a small set of
source/lockfile checkout-versus-Git hashes. A Windows checkout can have
different line-ending bytes from the Git blob; this is **observed separately**
and is not silently treated as proof of the cause of an emitted difference.

The workflow should fail on invalid section structure, mismatched Git
revision or tracked inputs, a different observed Rust release, truncated
transfer, or an invalid receipt. **A differing compiled guest is the expected
diagnostic subject, not a failing performance or portability gate.**
The result is visible in job logs. It does not upload or retain benchmark
data, modify the canonical Linux/Windows/macOS CI, run an application
deployment, or assign new compiler-stream work.

## Limits

Raw section hashes locate *where* output bytes differ, not why they differ.
Matching standard sections does not prove semantic equivalence; different
sections do not demonstrate a functional regression. The inspected
source-file subset and `Cargo.lock` are not a complete compiler
input/flag/environment attestation. Cached dependencies, Rust standard
libraries, host triples, file paths and code-generation nondeterminism remain
possible confounders. The existing capability-backed Markdown tests remain
separate correctness evidence; this experiment does not replace them.

P1 remains open for broader application portability, attributable memory,
measured cost boundaries and independent build reproducibility.

## Defined-function body localization (follow-up experiment)

Section-level digests previously established that both the executable code and
data sections differ across the native hosts. A different code-section hash,
however, cannot tell whether one or hundreds of individual function bodies
differ.

The [same P1 optional workflow](../.github/workflows/p1-markdown-guest-sections.yml)
now also records a **bounded, ordered vector of complete SHA-256 digests** for
each size-prefixed function body in the standard Wasm code section, plus body
byte lengths. The scanner decodes only the code-section vector and its
lengths; it **does not** parse Wasm instructions, infer source function names,
associate local bodies with imports, or claim that ordinal equality establishes
semantic equivalence. Invalid count/length boundaries and incomplete vectors
fail closed. To avoid inflated reports, at most 4,096 defined functions are
admitted, with a 256 KiB receipt limit.

Comparisons report both hosts' defined-function counts. If the counts match,
they report the number of differing ordinal bodies, an upper bound on the
combined sizes of those bodies, and the first 24 differing **defined-function
indices**, while preserving the raw section-level hashes. If the counts differ,
they explicitly mark positional body comparison as unavailable. Full hashes
remain in the bounded input receipts; only aggregate differences are printed
in the comparison report.

This further localizes *where* native builds diverge, not *why* they diverge.
Even one changed body can shift function references and affect other sections;
identical body bytes do not establish identical executable behavior. An
unexplained mismatch remains a diagnostic observation, not a failing
reproducibility or performance gate. The prior native Markdown capability
checks remain separate functional evidence.

## Order-independent exact body overlap

The hosted body-index comparison found **1,313/1,897** differing Windows
defined-function bodies and **1,374/1,897** differing Apple Silicon bodies
relative to Linux on the exact #110 test revision. Positional differences
alone cannot tell whether function ordering accounts for them.

The [same bounded inspector](../tools/bench/wasm-section-fingerprint.mjs) now
also takes the **multiset intersection** of the raw body length and full
SHA-256 digest pairs, without using function ordinals. Repeated identical
bodies are counted with multiplicity, not collapsed into a set.

The report provides matching body occurrences regardless of position,
unmatched occurrences on each host, the count matching at the same ordinal
(where both hosts have the same number of functions), and the additional
matches that appear only when ordinal is disregarded. If the function counts
differ, only the multiset overlap is compared and positional fields remain
explicitly unavailable. This works on the existing per-body fingerprint
receipts, without transferring or persisting generated Wasm.

If almost every body matches as a multiset but many differ positionally,
reordering is a plausible explanation. If exact multiset overlap is much
smaller, **permutation alone cannot account for the changed bytes**.
Neither conclusion identifies JavaScript/Rust source functions, follows
function call references, normalizes function-index immediates, or proves
semantic equivalence. Different bodies can still implement equivalent
behavior; matching bodies can reference different function indices.

The diagnostic workflow asserts only the internal arithmetic consistency
of the overlap counts, not any required percentage of matching code, so
host-dependent differences remain observations rather than CI failures.
