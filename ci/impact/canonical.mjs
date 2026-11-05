// Compare actual canonical invocations with independent command descriptors.
// A stale or ambiguous catalog must never be treated as a safe partial plan.
import { projectPrCommands } from '../../scripts/check-catalog.mjs';

function commandKey(check) {
  if (!check || !['node', 'cargo'].includes(check.command) ||
      !Array.isArray(check.args) || !check.args.length ||
      !check.args.every(arg => typeof arg === 'string' && arg.length > 0 && !arg.includes('\0'))) {
    throw new Error('invalid canonical command declaration');
  }
  return JSON.stringify([check.command, ...check.args]);
}

function declaredInventories(declared) {
  if (!Array.isArray(declared) || !declared.length) throw new Error('missing canonical declarations');
  const full = new Map(), pr = new Map(), variants = [];
  const ids = new Set();
  for (const check of declared) {
    if (!check || typeof check.id !== 'string' || !check.id.startsWith('check:') || ids.has(check.id)) {
      throw new Error('invalid or duplicate canonical check ID: ' + check?.id);
    }
    ids.add(check.id);
    const key = commandKey(check);
    if (full.has(key)) throw new Error('duplicate canonical declaration: ' + key);
    full.set(key, check.id);
    const projected = projectPrCommands(check);
    variants.push({ id: check.id, keys: projected.map(commandKey) });
    for (const command of projected) {
      const projectedKey = commandKey(command);
      if (pr.has(projectedKey)) throw new Error('duplicate canonical PR declaration: ' + projectedKey);
      pr.set(projectedKey, check.id);
    }
  }
  return { full, pr, variants };
}

function verifyInventory(observed, expected, allowEmpty = false) {
  if (!Array.isArray(observed) || !allowEmpty && !observed.length) {
    throw new Error('missing canonical command inventory');
  }
  const seen = new Set();
  for (const command of observed) {
    const key = commandKey(command);
    if (seen.has(key)) throw new Error('duplicate canonical invocation: ' + key);
    seen.add(key);
    if (!expected.has(key)) throw new Error('unmapped canonical invocation: ' + key);
  }
  for (const [key, id] of expected) {
    if (!seen.has(key)) throw new Error('stale canonical declaration ' + id + ': ' + key);
  }
  return { commands: observed.length, mapped: expected.size };
}

export function verifyCanonicalChecks(observed, declared) {
  return verifyInventory(observed, declaredInventories(declared).full);
}

// Capture the PR runner independently. Matching the full catalog alone cannot
// prove that an exclusion or bounded variant is the one the PR actually runs.
export function verifyPrChecks(observed, declared) {
  const { pr, variants } = declaredInventories(declared);
  const coverage = verifyInventory(observed, pr, true);
  const positions = new Map(observed.map((command, index) => [commandKey(command), index]));
  for (const { id, keys } of variants) {
    for (let index = 1; index < keys.length; index++) {
      if (positions.get(keys[index - 1]) >= positions.get(keys[index])) {
        throw new Error('canonical PR variant order differs from declaration: ' + id);
      }
    }
  }
  return coverage;
}

// Workspace-wide canonical commands must track new Cargo members automatically.
// These commands are still indivisible; only a future selective runner can
// translate a selected package into a per-package Cargo invocation.
export function checkNodes(declared, workspaceModules, cargoTestTargets = []) {
  if (!Array.isArray(cargoTestTargets) ||
      cargoTestTargets.some(id => typeof id !== 'string' ||
        !id.startsWith('cargo-test:') || !id.slice('cargo-test:'.length)) ||
      new Set(cargoTestTargets).size !== cargoTestTargets.length) {
    throw new Error('invalid Cargo executable test targets');
  }
  if (!Array.isArray(workspaceModules) ||
      !workspaceModules.every(id => typeof id === 'string' && id)) {
    throw new Error('invalid Cargo workspace package IDs');
  }
  return declared.map(({ command, args, scope, pr, ...node }) => {
    if (scope !== undefined && scope !== 'workspace') {
      throw new Error('unsupported scope: ' + scope);
    }
    return scope === 'workspace' ?
      { ...node,
        dependsOn: [...new Set([...(node.dependsOn ?? []), ...workspaceModules,
          // A runtime test can depend on a non-Cargo fixture (include_str!).
          // The virtual workspace test must be reached whenever that Cargo
          // test is selected, even though its build edge remains empty.
          ...(node.id === 'check:workspace-tests' ? cargoTestTargets : [])])],
        // In the selective preview this check is lowered to package-scoped
        // invocations. Its broad impact edges must not force all crates to build.
        buildDependsOn: [] } : node;
  });
}
