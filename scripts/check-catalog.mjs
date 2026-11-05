// Read one deterministic, independently owned manifest per canonical command.
// Validate the entire catalog before executing anything.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const defaultDirectory = fileURLToPath(new URL('./checks/', import.meta.url));
const manifestName = /^\d{6}-[a-z0-9]+(?:-[a-z0-9]+)*\.json$/;

export const PR_COMMAND_TIMEOUT_MS = 120_000;
export const PR_COMPILE_TIMEOUT_MS = 600_000;

function assertProfile(profile) {
  if (!['full', 'pr'].includes(profile)) throw new Error('unknown canonical check profile: ' + profile);
}

export function parseCheckProfile(args) {
  if (!Array.isArray(args) || args.length > 1 ||
      args.length === 1 && !['--profile=full', '--profile=pr'].includes(args[0])) {
    throw new Error('usage: node scripts/check.mjs [--profile=full|--profile=pr]');
  }
  return args.length ? args[0].slice('--profile='.length) : 'full';
}

function commandCopy(spec, strict = false) {
  if (spec === null || Array.isArray(spec) || typeof spec !== 'object' ||
      strict && Object.keys(spec).sort().join(',') !== 'args,command' ||
      !['node', 'cargo'].includes(spec.command) ||
      !Array.isArray(spec.args) || !spec.args.length ||
      spec.args.some(arg => typeof arg !== 'string' || !arg.length || arg.includes('\0'))) {
    throw new Error('invalid canonical command');
  }
  return { command: spec.command, args: [...spec.args] };
}

const commandKey = spec => JSON.stringify([spec.command, ...spec.args]);

// The same projection is usable for independent impact descriptors, whose
// ownership fields are deliberately not part of the executable command.
export function projectPrCommands(check) {
  const original = commandCopy(check);
  if (!Object.hasOwn(check, 'pr')) return [original];
  const policy = check.pr;
  if (policy === null || Array.isArray(policy) || typeof policy !== 'object' ||
      Object.keys(policy).sort().join(',') !== 'commands,reason' ||
      typeof policy.reason !== 'string' || !policy.reason.trim() || policy.reason.includes('\0') ||
      !Array.isArray(policy.commands)) {
    throw new Error('invalid canonical PR policy');
  }
  const seen = new Set();
  return policy.commands.map(spec => {
    const command = commandCopy(spec, true);
    const key = commandKey(command);
    if (seen.has(key)) throw new Error('duplicate canonical PR command: ' + key);
    seen.add(key);
    return command;
  });
}

export function commandTimeoutMs(command, args, profile = 'full') {
  assertProfile(profile);
  commandCopy({ command, args });
  if (profile === 'full') return 0;
  // This fixed aggregate applies its own 30-second discovery, ten-minute
  // compilation and two-minute execution limits. The PR job bounds the total.
  if (command === 'node' && args[0] === 'scripts/pr-cargo-tests.mjs' &&
      args.slice(1).every((arg, index, rest) => index % 2 === 0 ?
        arg === '--package' && index + 1 < rest.length : !arg.startsWith('-'))) return 0;
  const cargoArgs = args.slice(0, args.indexOf('--') < 0 ? args.length : args.indexOf('--'));
  const compilation = command === 'cargo' && (['build', 'clippy'].includes(args[0]) ||
    args[0] === 'test' && cargoArgs.includes('--no-run'));
  return compilation
    ? PR_COMPILE_TIMEOUT_MS : PR_COMMAND_TIMEOUT_MS;
}

export function loadCanonicalCommands(directory = defaultDirectory, { profile = 'full' } = {}) {
  assertProfile(profile);
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

  const full = [], pr = [];
  const fullSeen = new Set(), prSeen = new Set();
  for (const name of names) {
    let spec;
    try {
      spec = JSON.parse(readFileSync(join(directory, name), 'utf8'));
    } catch (error) {
      throw new Error('invalid canonical-check manifest: ' + name, { cause: error });
    }
    let command, projected;
    try {
      if (spec === null || Array.isArray(spec) || typeof spec !== 'object' ||
          !['args,command', 'args,command,pr'].includes(Object.keys(spec).sort().join(','))) {
        throw new Error('unsupported manifest fields');
      }
      command = commandCopy(spec);
      projected = projectPrCommands(spec);
    } catch (error) {
      throw new Error('invalid canonical-check command: ' + name + ': ' + error.message, { cause: error });
    }
    const key = commandKey(command);
    if (fullSeen.has(key)) throw new Error('duplicate canonical-check command: ' + name);
    fullSeen.add(key);
    full.push(command);
    for (const item of projected) {
      const projectedKey = commandKey(item);
      if (prSeen.has(projectedKey)) throw new Error('duplicate canonical PR command: ' + name);
      prSeen.add(projectedKey);
      pr.push(item);
    }
  }
  // Validate both complete inventories before choosing either profile. A bad
  // excluded entry must not disappear behind filtering, including on master.
  return profile === 'pr' ? pr : full;
}
