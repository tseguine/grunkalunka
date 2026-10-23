# Dependency license screening

Grunkalunka's own source stays MIT. Avoid GPL/LGPL dependencies;
an explicit permissive alternative may be chosen for a dual-licensed dependency.

Run `node scripts/licenses.mjs` from the repository root. It reads the locked,
offline Cargo metadata graphs for the workspace and retained frontend research,
checks exact reviewed license expressions, and
writes per-package name/version/source/declared/selected inventories to root
`target/license-inventory.json` and the frontend spike's own ignored target
directory. It includes all packages returned
by unfiltered Cargo metadata (including other-target/build packages), rather than
only crates compiled on the current Windows host. Missing/unreviewed expressions
fail the check. The selected expression must contain no GPL/LGPL license.

Observed 2026-10-23: 192 workspace packages and 83 frontend-research packages;
193 distinct package versions across both graphs, including the four maintained
workspace crates at 0.0.1 and the historical frontend package. The workspace migration retained
all 174 external numeric-implementation package versions, sources, checksums and
resolved feature sets.

R1a subsequently enables Wasmtime's `component-model` feature, adding its binding
generator and component support dependencies while leaving defaults disabled.
`encoding_rs 0.8.42` declares `(Apache-2.0 OR MIT) AND BSD-3-Clause`: select MIT
for the code and retain the BSD-3-Clause terms for WHATWG encoding data. Its
`COPYRIGHT`, `LICENSE-MIT` and `LICENSE-WHATWG` files confirm that combination.

Direct packages: Oxc 0.155.0 crates declare MIT; Wasmtime 50.0.0 declares
`Apache-2.0 WITH LLVM-exception`; wat 1.262.0 declares
`Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` (MIT selected).

The exact reviewed alternatives are recorded in `scripts/licenses.mjs`:

| Declared family | Selected terms |
| --- | --- |
| MIT alone, MIT/Apache alternatives, legacy slash alternatives | MIT |
| Apache-2.0 WITH LLVM-exception alone | Apache-2.0 WITH LLVM-exception |
| Apache-2.0 WITH LLVM-exception OR BSL-1.0 | Apache-2.0 WITH LLVM-exception |
| (MIT OR Apache-2.0) AND Unicode-3.0 | MIT AND Unicode-3.0 |
| (Apache-2.0 OR MIT) AND BSD-3-Clause | MIT AND BSD-3-Clause |
| BSD-2-Clause OR MIT OR Apache-2.0 | MIT |
| Unlicense OR MIT | MIT |
| Unicode-3.0 or Zlib alone | The declared license |
| Apache-2.0 OR GPL-2.0-only | Apache-2.0 |

The last entry is `self_cell 1.3.0`, used by Oxc. Its README explicitly offers
Apache-2.0 **OR** GPL-2.0; its bundled `LICENSE-APACHE` and `LICENSE-GPLv2` confirm
the alternatives. We choose Apache-2.0, not GPL. Legacy slash forms in bytecount,
fnv, rustc-demangle and version_check have bundled MIT and Apache license files;
the MIT alternative is selected. Dragonbox includes its Apache/LLVM and Boost
license files. The separately attributed TypeScript test excerpts now live under
`tests/compatibility/typescript/` with their Apache-2.0 license and adaptation notice.
Four original Test262 numeric test files and their BSD-3-Clause license are retained
under `tests/compatibility/test262/`, with Sputnik author notices and mapped
adaptations. The Benchmarks Game Mandelbrot source and derived scalar reference/
adaptation snippets under `tests/compatibility/benchmarks-game/` retain their
BSD-3-Clause license and original author attribution. All six external suite pins
and root-license screening scopes are recorded in `tests/compatibility/suites.json`; registration does not clear every
third-party workload for copying.
These source fixtures are not Cargo packages and are not counted by the metadata
screen; their pinned provenance and selected-file notices are reviewed separately.

This is dependency-metadata screening for the current development build, not a complete legal
or redistribution audit. The downloaded crates retain their license/notice files.
Before distributing the compiler/Wasmtime host binary, collect the required notices
and review bundled/native material and applicable terms; metadata alone cannot
establish that completeness. Tiny emitted numeric modules contain no bundled
Wasmtime runtime or external library imports. No binary package is published here.
