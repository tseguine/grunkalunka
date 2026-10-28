# Comparing saved portable GCD replay receipts (P1)

The [portable core replay](portable-core-replay.md) produces a JSON receipt on
each receiving host. To check whether two saved reports describe **the same
recorded producer artifact** without confusing different hosts' timings:

```sh
node tools/bench/compare-portable-replays.mjs left-replay.json right-replay.json
```

The comparator is read-only. It does not build, run or download anything.
Input files are capped at 1 MiB each. It checks the schema, recorded SHA-256
and size, source identity, manifest hash, producer metadata, receiver/Node
architecture identity, bounded sample configuration, the eight independent GCD
golden-case count and deterministic checksum, and the explicit Wasmtime
passed/not-run distinction. The two reports must agree on artifact, source,
manifest, producer and declared build label. Receiver OS and architecture may
differ, as may the native Wasmtime executable.

The output deliberately includes no timing samples, ranking, cross-host
ratios, speedup claims or statistical inference. A Node-only observation
remains identified as such; it is not silently promoted to a two-engine test.

**Limit:** receipts are untrusted self-reported data. A consistent pair can
be fabricated, and matching metadata is not signed provenance or execution
attestation. This checks internal consistency and agreement between recorded
observations, not machine identity, artifact build authenticity, or comparable
runtime speed. Preserve the original receipts and CI logs alongside the
comparison if they matter.

## Real cross-host CI integration

The optional [Linux x64 to Linux ARM64 GCD workflow](../.github/workflows/p1-portable-arm64.yml)
exercises this comparator on **two actual replay receipts**. The x64 producer
compiles once, validates its artifact in Node/V8 and native Wasmtime, and
transmits its bounded JSON receipt alongside the original Wasm bytes, manifest
and SHA-256 as ephemeral job outputs. The ARM64 receiver checks the exact byte
transfer and the producer receipt's link to the original manifest, runs those
unchanged Wasm bytes in its native Node/V8 and Wasmtime, then invokes this
comparator on the producer and receiver JSON reports. It must observe the same
recorded producer artifact, the expected x64/ARM64 host identities, eight
independent goldens and both Wasmtime results.

This is a stricter *end-to-end integration of the receipt reader* than the
synthetic comparator unit tests, but not authenticated evidence: GitHub logs and
job outputs contain observations, not signatures or hardware attestation.
It does **not** compare runtime durations across hosts, establish performance
parity, upload benchmark data or retain a separate comparison artifact.
An absent, truncated or secret-filtered job output is an explicit failure.
A passing CI job establishes the check for that exact tested revision only.
