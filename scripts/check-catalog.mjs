// Read one deterministic, independently owned manifest per canonical command.
// Validate the entire catalog before executing anything.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const defaultDirectory = fileURLToPath(new URL('./checks/', import.meta.url));
const manifestName = /^\d{6}-[a-z0-9]+(?:-[a-z0-9]+)*\.json$/;

export function loadCanonicalCommands(directory = defaultDirectory) {
  const entries = readdirSync(directory, { withFileTypes: true });
  const names = [];
  for (const entry of entries) {
    if (entry.isFile() && entry.name === 'README.md') continue;
    if (!entry.isFile() || !manifestName.test(entry.name)) {
      throw new Error('unsupported canonical-check catalog entry: ' + entry.name);
    }
    names.push(entry.name);
  }
  names.sort();
  if (!names.length) throw new Error('empty canonical-check catalog');

  const seen = new Set();
  return names.map(name => {
    let spec;
    try {
      spec = JSON.parse(readFileSync(join(directory, name), 'utf8'));
    } catch (error) {
      throw new Error('invalid canonical-check manifest: ' + name, { cause: error });
    }
    if (spec === null || Array.isArray(spec) || typeof spec !== 'object' ||
        Object.keys(spec).sort().join(',') !== 'args,command' ||
        !['node', 'cargo'].includes(spec.command) ||
        !Array.isArray(spec.args) || !spec.args.length ||
        spec.args.some(arg => typeof arg !== 'string' || !arg.length || arg.includes('\0'))) {
      throw new Error('invalid canonical-check command: ' + name);
    }
    const key = JSON.stringify([spec.command, ...spec.args]);
    if (seen.has(key)) throw new Error('duplicate canonical-check command: ' + name);
    seen.add(key);
    return { command: spec.command, args: [...spec.args] };
  });
}
