// Shared Node/Bun entry point. No package dependencies or shell command parsing.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false });
  if (result.error) throw result.error;
  if (result.signal || result.status !== 0) {
    throw new Error(`${command} failed: ${result.signal ?? result.status}`);
  }
}

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
run('cargo', ['fmt', '--all', '--check']);
run('cargo', ['clippy', '--workspace', '--all-targets', '--locked', '--offline', '-j', '4', '--', '-D', 'warnings']);
run('cargo', ['test', '--workspace', '--locked', '--offline', '-j', '4']);
run(process.execPath, ['scripts/check-json-guest.mjs']);
run('cargo', ['run', '--locked', '--offline', '--manifest-path', 'spikes/001-frontend-semantics/Cargo.toml']);
run(process.execPath, ['spikes/001-frontend-semantics/oracle.mjs']);
run(process.execPath, ['scripts/poc.mjs']);
run(process.execPath, ['--test', 'tests/compatibility/remainder-kernel.test.mjs']);
console.log(`PASS all checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
