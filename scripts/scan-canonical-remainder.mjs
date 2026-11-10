// Diagnostic full-profile check sweep. The workspace Cargo tests ran
// Report every independent failure rather than aborting on the first command.
// separately with --no-fail-fast, so skip that exact manifest command.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { loadCanonicalCommands } from './check-catalog.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const failures = [];
let passed = 0;
for (const { command, args } of loadCanonicalCommands(undefined, { profile: 'full' })) {
  if (command === 'cargo' && args[0] === 'test' && args.includes('--workspace')) continue;
  const executable = command === 'node' ? process.execPath : command;
  console.log('FULL CHECK START ' + JSON.stringify({ command, args }));
  const result = spawnSync(executable, args, {
    cwd: root,
    stdio: 'inherit',
    shell: false,
    killSignal: 'SIGKILL',
    timeout: 30 * 60_000,
  });
  const success = result.status === 0 && result.signal === null && !result.error;
  console.log('FULL CHECK END ' + JSON.stringify({
    command, args, status: result.status, signal: result.signal,
    error: result.error?.message ?? null, success,
  }));
  if (success) passed++;
  else failures.push({ command, args, status: result.status, signal: result.signal, error: result.error?.message ?? null });
}
console.log('FULL CHECK SUMMARY ' + JSON.stringify({ passed, failures }));
if (failures.length) process.exitCode = 1;
