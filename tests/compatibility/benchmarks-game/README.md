# Mandelbrot scalar orbit

Source: Computer Language Benchmarks Game, Git revision
`40296663ed350d5fe4a6ab5e367bab61cb77c219`,
`public/program/mandelbrot-node-1.html`. The program credits Greg Buchholz's C
program and Isaac Gouy's JavaScript transliteration. `original.js` is the decoded
text of the first HTML `pre` element, including its author header and whitespace;
it is not an upstream standalone file. `manifest.json` distinguishes the captured
raw HTML hash from the extracted JavaScript hash. Only the extracted JavaScript,
license and adapted/reference snippets are checked offline against retained bytes.

Upstream material and the derived `reference.txt` and `adapted.txt` snippets retain
the adjacent BSD-3-Clause terms. The loader and test harness are MIT code. The
reference hoists the original loop counter outside its `for` statement, initializes
one orbit at `cr=a, ci=b`, and returns an instrumented result. The adapted version
uses the currently admitted `while` and numeric branches. Multiplication order,
the inclusive squared-radius check and the 50-iteration limit are preserved.

The result is positive when the original pixel predicate holds and negative
otherwise; its magnitude is the completed iteration count. The initial zero
magnitude ensures at least one iteration, including for NaN inputs. The count is
added instrumentation, not upstream program output. Both versions retain the final
classification after iteration 50. A point escaping on the fiftieth iteration
distinguishes a shortened limit; NaN inputs distinguish `<= 4` from a `> 4` break.

The original full program runs only in a Node oracle test with a small 8-by-8
bitmap. Its output buffer is copied on every write before the program clears it.
All 64 original pixel bits are compared with the sign of both scalar versions.
Ten named orbit cases check exact counts, including interior/exterior, boundary,
late escape and NaN inputs. Those ten pairs also join the shared differential
vectors, so the compiled JS/TS variants execute them in both Wasmtime and Node-Wasm.
The separately retained `for` reference is the JS oracle in that differential run;
its emitted source file and hash are recorded independently of compilation inputs.

This executes an instrumented kernel, not the complete benchmark in Grunkalunka.
Coordinate generation, bitmap packing, typed arrays, bitwise operations, process
arguments and stdout are excluded. No Benchmarks Game timing or score is reported.
The next milestone is correct complete-program output before performance work.
