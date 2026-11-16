// Stable canonical entry point. Add checks under scripts/checks/, not here.
import { fileURLToPath } from 'node:url';
import { runCheck } from './check-command.mjs';
import { loadMasterCheckGroups } from './master-check-groups.mjs';
import { commandTimeoutMs, loadCanonicalCommands, parseCheckProfile } from './check-catalog.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const argv = process.argv.slice(2);
const groupArgs = argv.filter(arg => arg.startsWith('--master-group='));
if (groupArgs.length > 1) throw new Error('duplicate master group');
const profile = parseCheckProfile(argv.filter(arg => !arg.startsWith('--master-group=')));
if (groupArgs.length && profile !== 'full') throw new Error('master groups require full profile');
const allCommands = loadCanonicalCommands(undefined, { profile });
const commands = groupArgs.length
  ? loadMasterCheckGroups(allCommands).get(groupArgs[0].slice('--master-group='.length))
  : allCommands;
if (!commands) throw new Error('unknown master CI group: ' + groupArgs[0]);
for (const { command, args } of commands) {
  runCheck(command === 'node' ? process.execPath : command, args, root,
    { timeout: commandTimeoutMs(command, args, profile) });
}
console.log(`PASS ${groupArgs.length ? groupArgs[0] : 'all'} ${profile === 'pr' ? 'PR ' : ''}checks (${process.versions.bun ? `Bun ${process.versions.bun}` : `Node ${process.version}`})`);
