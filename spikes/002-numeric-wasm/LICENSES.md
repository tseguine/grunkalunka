# Dependency license screening

Grunkalunka's own source stays MIT. The user selected avoiding GPL/LGPL dependencies;
an explicit permissive alternative may be chosen for a dual-licensed dependency.
User alignment on a dependency also covers downloading it and its required packages.

Run `node scripts/licenses.mjs` from the repository root. It reads both locked,
offline Cargo metadata graphs, checks exact reviewed license expressions, and
writes per-package name/version/source/declared/selected inventories to each
spike's ignored `target/license-inventory.json`. It includes all packages returned
by unfiltered Cargo metadata (including other-target/build packages), rather than
only crates compiled on the current Windows host. Missing/unreviewed expressions
fail the check. The selected expression must contain no GPL/LGPL license.

Observed 2026-10-23: 83 packages for spike 001; 175 for spike 002; 176 distinct package
versions across both. Our two experimental packages are included in those counts.

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
| BSD-2-Clause OR MIT OR Apache-2.0 | MIT |
| Unlicense OR MIT | MIT |
| Unicode-3.0 or Zlib alone | The declared license |
| Apache-2.0 OR GPL-2.0-only | Apache-2.0 |

The last entry is `self_cell 1.3.0`, used by Oxc. Its README explicitly offers
Apache-2.0 **OR** GPL-2.0; its bundled `LICENSE-APACHE` and `LICENSE-GPLv2` confirm
the alternatives. We choose Apache-2.0, not GPL. Legacy slash forms in bytecount,
fnv, rustc-demangle and version_check have bundled MIT and Apache license files;
the MIT alternative is selected. Dragonbox includes its Apache/LLVM and Boost
license files. No third-party source was copied into this repository by the PoC.

This is dependency-metadata screening for a local experiment, not a complete legal
or redistribution audit. The downloaded crates retain their license/notice files.
Before distributing the compiler/Wasmtime host binary, collect the required notices
and review bundled/native material and applicable terms; metadata alone cannot
establish that completeness. Tiny emitted numeric modules contain no bundled
Wasmtime runtime or external library imports. No binary package is published here.
