// Cargo package ownership comes from Cargo metadata. These optional per-package
// test-only edges capture repository inputs consumed outside a Cargo package,
// e.g. Rust tests that include_str! external JS fixtures. Build prerequisites
// stay tied to the Cargo package, not to unrelated fixture owners.
export function cargoTestNodes(workspacePackages, supplemental = []) {
  if (!Array.isArray(workspacePackages) || !workspacePackages.length ||
      !Array.isArray(supplemental)) {
    throw new Error('missing Cargo test graph inputs');
  }
  const names = workspacePackages.map(pkg => pkg?.name);
  if (names.some(name => typeof name !== 'string' || !name) ||
      new Set(names).size !== names.length) {
    throw new Error('duplicate or invalid Cargo workspace packages');
  }
  const known = new Set(names), extraByPackage = new Map();
  for (const entry of supplemental) {
    if (!entry || typeof entry.package !== 'string' ||
        !known.has(entry.package) || extraByPackage.has(entry.package) ||
        !Array.isArray(entry.dependsOn) || !entry.dependsOn.length ||
        entry.dependsOn.some(id => typeof id !== 'string' || !id ||
          id === entry.package) ||
        new Set(entry.dependsOn).size !== entry.dependsOn.length ||
        Object.keys(entry).sort().join(',') !== 'dependsOn,package') {
      throw new Error('invalid or stale external test inputs for ' + entry?.package);
    }
    extraByPackage.set(entry.package, [...entry.dependsOn]);
  }
  return names.map(name => ({
    id: 'cargo-test:' + name,
    dependsOn: [name, ...(extraByPackage.get(name) ?? [])],
    // Selection edges represent test inputs; they must not make file-only
    // owners into Cargo build prerequisites.
    buildDependsOn: [name],
  }));
}
