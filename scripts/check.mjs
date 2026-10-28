// Stable canonical entry point. Add checks under scripts/checks/, not here.
import { fileURLToPath } from 'node:url';
import { runCheck } from './check-command.mjs';
import { loadCanonicalCommands } from './check-catalog.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
for (const { command, args } of loadCanonicalCommands()) {
  runCheck(command === 'node' ? process.execPath : command, args, root);
}
console.log(`PASS all checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
