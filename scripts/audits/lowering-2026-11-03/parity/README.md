# Fixed lowering parity corpus

This supplement preserves the exact inputs and independent expected output hashes
for the historical 7,487-case, 34-matrix compiler comparison. It is a manual audit
tool, not a new CI job or canonical compatibility runner.

There are **6,033 admitted cases and 1,454 rejected cases**. The historical run
compared 13,554 output files: 13,520 WAT, Wasm, or diagnostic files and 34 metadata
summaries. Artifact hashes cover complete bytes, including exact first diagnostic
text and spans. Generated artifacts are intentionally omitted from this bundle.

## What is fixed

Each case in `cases/` retains its complete UTF-8 source, compiler profile, language,
frozen actual outcome, original discovery expectation, and source/output hashes and
lengths. JSONL files are chunked below 120 KiB. `catalog.json` fixes their ordering,
counts, chunk hashes, and historical summary hashes. Source inputs are self-contained;
materialization does not consult current repository fixtures or regenerate expected
compiler outputs.

`probe.rs` is the unchanged historical Rust harness. Its SHA-256 is
`13f3c789bbd405fb550bdb79886449c32dff4c7ebb944e97977d68f223c9b0d7`.
It executes each public compiler API on an explicit 2 MiB worker thread and writes
full WAT/Wasm or diagnostic bytes. It supports numeric, scalar-value, async, managed
heap, and UI-import compiler profiles. These are compiler artifact comparisons;
the separate runtime receipt supplies execution evidence.

The historical baselines were frozen before the relevant conversions at several
source snapshots. They were not all first produced by one compiler commit.
`provenance.json` and `historical/` retain the available freeze/comparison receipts,
probe identities, source and published commit mappings, and any provenance limits.
The final source snapshot is `4bef43c35814d7cb747254ad4d4fbac0dc9715dd`, tree
`9ea19ddd1ff1dbd0d39c46216d13a22747130735`; the published equivalent is
`67400f91882124e021f4bf30157930bb91acba31` (PR #466), with the identical tree.

The maintained 5,796-case corpus has its original `generate-corpus.mjs` preserved.
Its repository inputs are pinned to published baseline commit
`926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c`; repository paths and hashes are recorded
in `provenance.json`. The other 1,691 cases remain explicit fixed source snapshots,
including hand-written cases and generated boundary ladders. A saved generator is
not claimed where none was retained. The fixed source bytes suffice to reconstruct
every tested input.

## Validation of this package

`validation/portable-final-replay.json` records an independent replay through this
portable runner using the immutable final-source probe: all 7,487 cases and all
13,554 output-file checks passed. The historical baseline hash check, complete
source/manifest roundtrip, pinned-generator reconstruction, and deliberate
corruption controls are retained alongside it. The controls reject changed
diagnostics, actual outcomes, profiles, extra or missing files, CRLF metadata, and
`observe` metadata without the explicit historical option.

`validation/build-helper-receipt.json` records a new target-directory build of the
unchanged probe from the exact final source tree. Its 16-case smoke covers admitted
and rejected cases, and `validation/paired-smoke.json` covers pair-mode byte
comparison using independent predecessor and final probes. These checks validate
the packaging and tools; the archived production receipts retain their own scope.

`SHA256SUMS` fixes every delivered file except the checksum inventory itself. Case
chunk hashes and the immutable source/output hashes are also checked by the runner.

## Commands

Python 3.10 or newer is required. Run commands from any directory; bundled paths
are resolved relative to the scripts. Output directories must be new. The runner
never updates the frozen expectations.

```sh
python /path/to/parity/run_parity.py inventory
python /path/to/parity/run_parity.py materialize /tmp/lowering-inputs
```

Build the unchanged harness in an isolated checkout of the published final commit.
Use a new, exclusive Cargo target directory. This temporarily adds an untracked
example to the checkout and removes it afterward. Dependencies must already be
available for `--locked --offline` Cargo execution.

```sh
git worktree add --detach /tmp/lowering-final 67400f91882124e021f4bf30157930bb91acba31
python /path/to/parity/build_probe.py \
  --repo /tmp/lowering-final \
  --expect-commit 67400f91882124e021f4bf30157930bb91acba31 \
  --target-dir /tmp/lowering-final-target \
  --output /tmp/lowering-final-probe
python /path/to/parity/run_parity.py check \
  --probe /tmp/lowering-final-probe/parity-probe \
  --output /tmp/lowering-final-replay
```

`check` validates newly generated files against the independent frozen hashes. It
does not need any historical binary or generated WAT/Wasm file. To compare two
supplied compiler probes, use the following command. Both must also match the
independent expectations; selecting a baseline that predates a case's admission
can correctly fail. Consult provenance when selecting baseline revisions.

```sh
python /path/to/parity/run_parity.py compare \
  --baseline-probe /path/to/baseline-probe \
  --candidate-probe /path/to/candidate-probe \
  --output /tmp/lowering-pair-replay
```

Add `--only materialization expression-resumption` to limit any command to named
matrices. Probes run serially. The default per-matrix timeout is 900 seconds and is
adjustable with `--timeout`; a failure retains partial outputs and a failing receipt.
Do not run these probes concurrently with cost measurements.

For already-generated outputs arranged as `ROOT/MATRIX/*`, use
`verify-existing ROOT`. `--historical-layout` instead uses the recorded historical
baseline directory names and permits the narrowly defined metadata normalization
below. Verification checks the exact file set, every artifact hash and length, and
every summary row's identity, language, profile, and actual status.

## Discovery metadata normalization

The original `verify_next.py` is archived unchanged. Six early manifests still
said `expected=observe`, while their actual outcomes and artifact bytes were
frozen. Three later runtime matrices had frozen summary expectations of `observe`
but manifests with explicit expectations. The historical verifier normalized only
that summary expectation column, after verifying the actual outcome against the
frozen manifest; no artifact or diagnostic bytes were normalized.

The portable materializer writes the explicitly frozen actual outcome as each
manifest expectation, so new runs have strict admission checks. Historical-output
verification may replace `observe` only when the case record proves that it occurred
in the original metadata and the independently fixed actual outcome, identity,
profile, and language all match. Every other difference fails. This is a metadata
conversion, not a relaxation of output comparison.

## Reconstructing maintained fixture provenance

The original generator can be run against an isolated checkout of the pinned
baseline. It produces sources and manifests only and does not invoke a compiler:

```sh
git worktree add --detach /tmp/lowering-fixture-baseline 926a4aa8bcf7c6082a3d09d795a7e8cd7bfea91c
node /path/to/parity/generate-corpus.mjs /tmp/lowering-fixture-baseline /tmp/regenerated-lowering-corpus
```

This command is useful for source provenance. It is unnecessary for normal parity
replay, which uses the complete fixed source snapshots in this supplement.

## Limits

Successful replay establishes the recorded admission, exact compiler artifacts,
and first diagnostics for these inputs and compiler profiles. It does not prove
all-language equivalence, absence of untested bugs, or runtime semantics by itself.
The recorded historical receipts remain historical observations; portable replay
produces a new receipt with the actual probe hashes and source build metadata.
