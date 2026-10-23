# TypeScript-derived arithmetic cases

Expressions extracted from Microsoft TypeScript, tag `v5.9.3`, immutable commit
`c63de15a992d37f0d6cec03ac7631872838602cb`.
Copyright Microsoft Corporation and TypeScript contributors. The extracted material
retains Apache-2.0 terms in [LICENSE.txt](LICENSE.txt); Grunkalunka's MIT license
does not relicense it.

[Source fixture](https://github.com/microsoft/TypeScript/blob/c63de15a992d37f0d6cec03ac7631872838602cb/tests/cases/conformance/expressions/binaryOperators/arithmeticOperator/arithmeticOperatorWithAnyAndNumber.ts)
and [emitted JS baseline](https://github.com/microsoft/TypeScript/blob/c63de15a992d37f0d6cec03ac7631872838602cb/tests/baselines/reference/arithmeticOperatorWithAnyAndNumber.js).

## Selection and modifications

`expressions.txt` preserves 32 selected right-hand-side expressions, in source
order. `manifest.json` maps each to its original one-based source line and variable
name through the fixture ID. It records full-source and emitted-baseline hashes
captured from the pinned revision, plus the locally checked excerpt hash.

The adapter changes declarations into returned expressions inside the existing
two-parameter `run(a,b)` ABI. Both parameters receive initialized primitive numbers;
the upstream `a: any` domain is deliberately narrowed. The original type-checking
purpose, uninitialized globals and entire-program behavior are not reproduced.
All 32 selected multiplication/division/remainder/subtraction expressions now run
in JS, Node-Wasm and Wasmtime. The eight formerly rejected remainder expressions
retain their original IDs, source lines and exact original TS module text, and
now have paired unannotated JS numeric oracles. No selected adapter rejection
remains. The other 48 bitwise/shift expressions are still unselected and
unexecuted; they do not contribute passing tests. This deliberate promotion does
not change the pinned source/excerpt/license or execute the full upstream suite.

At capture time, the selected declarations were also checked against the emitted-JS
section of the upstream baseline. That baseline describes compiler output, not
runtime results. The execution oracle remains the adapted JavaScript in Node.
Offline replay verifies excerpt integrity and selection IDs/lines/counts/operator
expectations; it cannot reverify full upstream files from these excerpts alone.
No TypeScript compiler, type baselines or upstream test runner executes here.

Selected-file headers and ancestor directories were inspected for license notices.
The root `ThirdPartyNoticeText.txt` describes DefinitelyTyped, Unicode, DOM,
background synchronization and WebGL material; those materials are not included in
these arithmetic excerpts. Recheck applicable notices for future selections.

Run the normal `node scripts/check.mjs`. Reports carry this manifest and immutable
source/baseline URLs in the additive `externalSources` field of report schema 1.
