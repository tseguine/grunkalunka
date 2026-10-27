// Internal canonical-check subprocess runner. Logging does not change exit policy.
import { spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';

export function runCheck(command, args, cwd) {
  console.log('\nCHECK START ' + JSON.stringify({ command, args }));
  const started = performance.now();
  const result = spawnSync(command, args, { cwd, stdio: 'inherit', shell: false });
  // Start on a new line even when the child did not terminate its last output line.
  console.log('\nCHECK END ' + JSON.stringify({
    command, args, elapsedMs: Math.round(performance.now() - started),
    status: result.status, signal: result.signal, error: result.error?.code ?? null,
  }));
  if (result.error) throw result.error;
  if (result.signal || result.status !== 0) {
    throw new Error(`${command} failed: ${result.signal ?? result.status}`);
  }
}
