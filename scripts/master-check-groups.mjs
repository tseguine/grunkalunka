// Full master checks are partitioned into parallel jobs. This reads the
// independently maintained impact descriptors and verifies the entire catalog
// before selecting any group; omitted tags mean the remaining group.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyCanonicalChecks } from '../ci/impact/canonical.mjs';

const descriptorDirectory = fileURLToPath(new URL('../ci/impact/checks/', import.meta.url));
export const MASTER_GROUPS = Object.freeze(['rust-tests', 'style', 'applications', 'remaining']);

const key = ({ command, args }) => JSON.stringify([command, ...args]);

export function partitionMasterChecks(commands, descriptors) {
  verifyCanonicalChecks(commands, descriptors);
  const registered = new Map(descriptors.map(descriptor => [key(descriptor), descriptor]));
  const groups = new Map(MASTER_GROUPS.map(group => [group, []]));
  for (const command of commands) {
    const descriptor = registered.get(key(command));
    const group = descriptor.masterGroup ?? 'remaining';
    if (!groups.has(group)) {
      throw new Error('unknown master CI group for ' + descriptor.id + ': ' + group);
    }
    groups.get(group).push(command);
  }
  for (const [group, checks] of groups) {
    if (!checks.length) throw new Error('empty master CI group: ' + group);
  }
  return groups;
}

export function loadMasterCheckGroups(commands, directory = descriptorDirectory) {
  const entries = readdirSync(directory, { withFileTypes: true });
  if (!entries.length || entries.some(entry => !entry.isFile() || !entry.name.endsWith('.json'))) {
    throw new Error('invalid CI impact descriptor catalog');
  }
  const descriptors = entries.map(entry =>
    JSON.parse(readFileSync(join(directory, entry.name), 'utf8')));
  return partitionMasterChecks(commands, descriptors);
}
