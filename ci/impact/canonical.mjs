// Compare actual canonical invocations with independent command descriptors.
// A stale or ambiguous catalog must never be treated as a safe partial plan.
function commandKey(check) {
  if (!check || !['node', 'cargo'].includes(check.command) ||
      !Array.isArray(check.args) || !check.args.length ||
      !check.args.every(arg => typeof arg === 'string' && arg.length > 0)) {
    throw new Error('invalid canonical command declaration');
  }
  return JSON.stringify([check.command, ...check.args]);
}

export function verifyCanonicalChecks(observed, declared) {
  if (!Array.isArray(observed) || !Array.isArray(declared) || !observed.length) {
    throw new Error('missing canonical command inventory');
  }
  const expected = new Map();
  const ids = new Set();
  for (const check of declared) {
    if (typeof check.id !== 'string' || !check.id.startsWith('check:') || ids.has(check.id)) {
      throw new Error('invalid or duplicate canonical check ID: ' + check.id);
    }
    ids.add(check.id);
    const key = commandKey(check);
    if (expected.has(key)) throw new Error('duplicate canonical declaration: ' + key);
    expected.set(key, check.id);
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

// Workspace-wide canonical commands must track new Cargo members automatically.
// These commands are still indivisible; only a future selective runner can
// translate a selected package into a per-package Cargo invocation.
export function checkNodes(declared, workspaceModules) {
  if (!Array.isArray(workspaceModules) ||
      !workspaceModules.every(id => typeof id === 'string' && id)) {
    throw new Error('invalid Cargo workspace package IDs');
  }
  return declared.map(({ command, args, scope, ...node }) => {
    if (scope !== undefined && scope !== 'workspace') {
      throw new Error('unsupported scope: ' + scope);
    }
    return scope === 'workspace' ?
      { ...node,
        dependsOn: [...new Set([...(node.dependsOn ?? []), ...workspaceModules])],
        // In the selective preview this check is lowered to package-scoped
        // invocations. Its broad impact edges must not force all crates to build.
        buildDependsOn: [] } : node;
  });
}
