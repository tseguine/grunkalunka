// Permit a new canonical test command to select only its declared dependencies.
// Never exempt edits/deletions, unmatched files, or changes to the planner itself.
const manifestPattern = /^scripts\/checks\/\d{6}-([a-z0-9]+(?:-[a-z0-9]+)*)\.json$/;
const descriptorPattern = /^ci\/impact\/checks\/([a-z0-9]+(?:-[a-z0-9]+)*)\.json$/;
const isRegistration = path =>
  path.startsWith('scripts/checks/') || path.startsWith('ci/impact/checks/');

export function scopedAddedChecks({ changedPaths, addedPaths, declared, observed }) {
  if (![changedPaths, addedPaths, declared, observed].every(Array.isArray)) {
    throw new Error('missing registration validation inputs');
  }
  const registrationPaths = changedPaths.filter(isRegistration);
  if (!registrationPaths.length) return {};
  const additions = new Set(addedPaths);
  if (additions.size !== addedPaths.length) throw new Error('duplicate added paths');
  const pairs = new Map();
  for (const path of registrationPaths) {
    if (!additions.has(path)) throw new Error('existing or deleted test registration changed: ' + path);
    const manifest = manifestPattern.exec(path);
    const descriptor = descriptorPattern.exec(path);
    if (!manifest && !descriptor) throw new Error('unrecognized test registration path: ' + path);
    const name = (manifest ?? descriptor)[1];
    const pair = pairs.get(name) ?? {};
    const kind = manifest ? 'manifest' : 'descriptor';
    if (pair[kind]) throw new Error('duplicate registration for ' + name);
    pair[kind] = path;
    pairs.set(name, pair);
  }

  const declaredById = new Map(declared.map(d => [d.id, d]));
  const observedCommands = new Set(observed.map(d => JSON.stringify([d.command, ...d.args])));
  const scoped = {};
  for (const [name, pair] of pairs) {
    if (!pair.manifest || !pair.descriptor) {
      throw new Error('new canonical check requires both registration files: ' + name);
    }
    const id = 'check:' + name;
    const descriptor = declaredById.get(id);
    if (!descriptor || descriptor.scope === 'workspace' ||
        !Array.isArray(descriptor.args) || !descriptor.args.length ||
        !['node', 'cargo'].includes(descriptor.command) ||
        !(Array.isArray(descriptor.dependsOn) && descriptor.dependsOn.length ||
          Array.isArray(descriptor.paths) && descriptor.paths.length) ||
        !observedCommands.has(JSON.stringify([descriptor.command, ...descriptor.args]))) {
      throw new Error('unsafe or incomplete new canonical check: ' + name);
    }
    scoped[pair.manifest] = id;
    scoped[pair.descriptor] = id;
  }
  return scoped;
}
