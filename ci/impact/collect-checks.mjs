// Enumerate the actual canonical check invocation list without running its children.
// Runs in an isolated Node process; neither modifies nor wraps production check files.
import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';

const originalSpawn = childProcess.spawnSync;
const originalLog = console.log;
const commands = [];
try {
  childProcess.spawnSync = (command, args, options) => {
    if (typeof command !== 'string' || !Array.isArray(args) ||
        args.some(arg => typeof arg !== 'string') ||
        options?.shell !== false || !options.cwd) {
      throw new Error('unexpected canonical subprocess invocation');
    }
    commands.push({ command: command === process.execPath ? 'node' : command, args: [...args] });
    return { status: 0, signal: null, error: null };
  };
  syncBuiltinESMExports();
  console.log = () => {}; // Silence artificial CHECK START/END records.
  await import(new URL('../../scripts/check.mjs', import.meta.url));
} finally {
  console.log = originalLog;
  childProcess.spawnSync = originalSpawn;
  syncBuiltinESMExports();
}
if (!commands.length) throw new Error('canonical runner has no instrumentable commands');
process.stdout.write(JSON.stringify(commands) + '\n');
