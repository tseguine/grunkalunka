# Bounded Oxc frontend-cost probe (P1)

This independent measurement extends the existing disposable
[Oxc lexical-semantics experiment](../spikes/001-frontend-semantics/README.md).
It observes two narrowly defined phases without changing the production
Grunkalunka compiler or its admitted source semantics.

Run from the repository root, after fetching the spike's locked Cargo
dependencies:

```sh
cargo run --manifest-path spikes/001-frontend-semantics/Cargo.toml \
  --locked --offline -- --frontend-cost \
  --samples 7 --iterations 100 --warmup 20
```

The output is one JSON object with raw timings for four existing fixtures:
`bindings.js`, `bindings.ts`, `coercion.js`, and `coercion.ts`.
Each report includes original source-byte counts, expected symbol/reference
counts and separately retained input-order measurements with min/median/max
in **milliseconds per bounded batch**. The pinned Oxc version is 0.155.0.

## Precisely measured work

- **Parsing with allocation:** creates a fresh Oxc Arena/Allocator, parses the
  complete fixed source in JS/TS module mode, and observes the number of top-level
  statements and diagnostics. AST allocation is included. Reading the input file,
  type checking and semantic analysis are excluded.
- **Semantic construction on a reused AST:** repeatedly invokes
  `SemanticBuilder::new_compiler().with_build_nodes(true).build()` on an AST
  parsed once outside the timed interval. Symbol/reference enumeration and
  diagnostic collection are included in this phase. The existing typed source
  annotations are parsed but **not** type-checked.

Both workloads are checked first against independently recorded fixture
semantics (5/4 symbols and 4/5 references, respectively); every batch's
aggregate statement/symbol/reference counts and diagnostic totals are verified
**after** its timed interval. Sample ordering alternates parsing-first and
semantic-first to reduce but not eliminate ordering bias. Fixed warmups are
separate from retained samples. Timed phases are not summed into a supposed
pipeline cost: they have different allocation, AST ownership and cache behavior.

Arguments are unique `--option value` pairs. Samples are 2..12 (default 7),
iterations per batch are 1..500 (default 100), and warmup iterations per phase
are 0..100 (default 20). Invalid or duplicate controls fail. All source
fixtures are fixed and under the spike's existing bounded fixture sizes.
Only raw timing observations are emitted; there are **no speed thresholds,
machine-independent ratios or rankings**.

The normal canonical workspace check already executes this spike without
arguments. That path continues its original lexical/diagnostic checks and now
runs one tiny two-sample frontend-cost smoke across all four fixtures. This
exercises actual Oxc parsing, semantic results, raw report structure and invalid
configuration rejection, but does **not** persist timings or gate on speed.

## Interpretation and remaining P1 work

This is **Oxc-only frontend phase evidence**, not isolated Grunkalunka source
frontend overhead. It excludes Grunkalunka's source-admission checks,
signature inference, lowering, IR verification, Wasm emission, toolchain startup,
and compiler process transport. Its semantic phase reuses an AST rather than
repeating full compiler compilation. Allocator costs, diagnostics, witness
enumeration, cache state and OS scheduling affect the results.

Before any comparable measurements, preserve exact Git commit, Rust/Oxc
versions, build profile and optimization flags, CPU/OS/architecture, fixture
hashes, repetitions, raw samples and cache/thermal conditions alongside the
receipt. This tool records only Oxc version and observed OS/architecture;
it does not attest the compiler binary or capture all provenance automatically.
No release, deployment, persistent artifact or dependency is added. The existing
compiler, browser workstream and A1 pilot remain unrelated.
