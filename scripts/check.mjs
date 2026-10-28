// Shared Node/Bun entry point. No package dependencies or shell command parsing.
import { fileURLToPath } from 'node:url';
import { runCheck } from './check-command.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const run = (command, args) => runCheck(command, args, root);

run(process.execPath, ['--test', 'tests/build/check-command.test.mjs']);
run(process.execPath, ['--test', 'tests/build/process.test.mjs', 'tests/build/source.test.mjs']);
run(process.execPath, ['--test', 'tests/build/execution.test.mjs', 'tests/build/primitive-execution.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/suites.test.mjs', 'tests/compatibility/test262.test.mjs',
  'tests/compatibility/benchmarks-game.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/gap-oracle.test.mjs', 'tests/compatibility/program-sources.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/rejection-inventory.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/watchdog-budget.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/numeric-math-abs-kernel.test.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/numeric-math-unary-kernel.test.mjs']);
for (const spike of ['001-frontend-semantics']) {
  const manifest = `spikes/${spike}/Cargo.toml`;
  run('cargo', ['fmt', '--manifest-path', manifest, '--check']);
  run('cargo', ['clippy', '--locked', '--offline', '--manifest-path', manifest, '-j', '4', '--', '-D', 'warnings']);
}
// The type-query research spike is an independent Cargo workspace.
run('cargo', ['fetch', '--locked', '--manifest-path', 'spikes/004-tsrs-type-query/Cargo.toml']);
run('cargo', ['fmt', '--manifest-path', 'spikes/004-tsrs-type-query/Cargo.toml', '--check']);
run('cargo', ['test', '--manifest-path', 'spikes/004-tsrs-type-query/Cargo.toml',
  '--locked', '--offline', '-j', '2', '--', '--test-threads=1']);
run('cargo', ['fmt', '--all', '--check']);
run('cargo', ['clippy', '--workspace', '--all-targets', '--locked', '--offline', '-j', '4', '--', '-D', 'warnings']);
run('cargo', ['test', '--workspace', '--locked', '--offline', '-j', '4']);
run(process.execPath, ['--test', 'tests/build/compile.test.mjs', 'tests/build/primitive-integration.test.mjs']);
run(process.execPath, ['--test', 'tests/build/example-options.test.mjs', 'tests/build/example.test.mjs']);
run(process.execPath, ['scripts/check-json-composition.mjs']);
run(process.execPath, ['scripts/check-heap-literals.mjs']);
run('cargo', ['run', '--locked', '--offline', '--manifest-path', 'spikes/001-frontend-semantics/Cargo.toml']);
run(process.execPath, ['spikes/001-frontend-semantics/oracle.mjs']);
run(process.execPath, ['scripts/poc.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/remainder-kernel.test.mjs']);
run(process.execPath, ['--test', 'tests/build/benchmark.test.mjs', 'tests/build/benchmark-integration.test.mjs',
  'tests/build/benchmark-comparison.test.mjs', 'tests/build/compare-portable-replays.test.mjs']);
// P1 diagnostic and portability contracts are regular Node tests. Individual
// empirical cross-host builds remain explicit, not a required PR check per experiment.
run(process.execPath, ["--test", "tools/bench/tests/*.test.mjs", "tests/build/native-build-identity.test.mjs", "tests/build/portable-affine.test.mjs", "tests/build/markdown-capture.test.mjs", "tests/build/markdown-comparison.test.mjs"]);
// Native-host P1 portability: compile once, execute actual Wasm on Node and Wasmtime.
run(process.execPath, ['--test', 'tests/build/p1-native-portability.test.mjs']);
run(process.execPath, ['scripts/check-markdown-portability.mjs']);
run(process.execPath, ['scripts/check-markdown-outline.mjs']);
run(process.execPath, ['scripts/check-document-outline.mjs']);
console.log(`PASS all checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
