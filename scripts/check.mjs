// Shared Node/Bun entry point. No package dependencies or shell command parsing.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const manifest = 'spikes/001-frontend-semantics/Cargo.toml';
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit', shell: false });
  if (result.error) throw result.error;
  if (result.signal || result.status !== 0) {
    throw new Error(`${command} failed: ${result.signal ?? result.status}`);
  }
}

run('cargo', ['fmt', '--manifest-path', manifest, '--check']);
run('cargo', ['clippy', '--locked', '--offline', '--manifest-path', manifest, '--', '-D', 'warnings']);
run('cargo', ['run', '--locked', '--offline', '--manifest-path', manifest]);
run(process.execPath, ['spikes/001-frontend-semantics/oracle.mjs']);
console.log(`PASS all checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
