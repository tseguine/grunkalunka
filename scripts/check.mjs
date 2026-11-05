// Stable canonical entry point. Add checks under scripts/checks/, not here.
import { fileURLToPath } from 'node:url';
import { runCheck } from './check-command.mjs';
import { commandTimeoutMs, loadCanonicalCommands, parseCheckProfile } from './check-catalog.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const profile = parseCheckProfile(process.argv.slice(2));
for (const { command, args } of loadCanonicalCommands(undefined, { profile })) {
  runCheck(command === 'node' ? process.execPath : command, args, root,
    { timeout: commandTimeoutMs(command, args, profile) });
}
console.log(`PASS all ${profile === 'pr' ? 'PR ' : ''}checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
