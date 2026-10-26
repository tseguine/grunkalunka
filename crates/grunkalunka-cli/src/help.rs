pub const TEXT: &str = "Grunkalunka: compile and run the supported JS/TS profiles

Usage:
  grunkalunka compile SOURCE.{js,ts} NEW-OUTPUT.wasm
  grunkalunka wat SOURCE.{js,ts}
  grunkalunka eval OUTPUT.wasm < input-bits.txt

Primitive-return profile (undefined, null, Boolean or Number):
  grunkalunka compile-value SOURCE.{js,ts} NEW-OUTPUT.wasm
  grunkalunka wat-value SOURCE.{js,ts}
  grunkalunka eval-value OUTPUT.wasm < input-bits.txt

Options (standalone):
  -h, --help       Show this help and exit
  -V, --version    Show the package version and exit

Both profiles take two numeric inputs. Evaluation reads at most 4096 lines,
each containing two 16-digit hexadecimal binary64 bit patterns, not decimals.
Numeric output is one 16-digit pattern per line. Primitive output is undefined,
null, boolean:0, boolean:1, or number: followed by a 16-digit pattern.

Compilation refuses existing output files. WAT is printed to stdout.
This is a supported subset, not full JavaScript/TypeScript or Node compatibility.
See docs/compiler.md for admission limits and runtime contracts.
";
