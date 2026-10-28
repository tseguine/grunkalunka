// Advisory-only Cargo command preview; never executes or weakens canonical CI.
const packageArgs = ids => ids.flatMap(id => ['-p', id]);
const sorted = entries => [...new Set(entries)].sort();

export function previewCargoScope(impact, workspacePackages) {
  const full = reason => ({ mode: 'full', advisoryOnly: true, reason,
    selectedBuildPackages: [], selectedTestPackages: [],
    commands: null });
  if (!impact || impact.full) return full(impact?.reasons?.join('; ') || 'full impact required');
  if (!Array.isArray(impact.tests) || !Array.isArray(impact.build) ||
      !Array.isArray(workspacePackages) || workspacePackages.length === 0) {
    return full('missing graph or workspace metadata');
  }
  if (workspacePackages.some(pkg => typeof pkg !== 'string' || !pkg.length) ||
      new Set(workspacePackages).size !== workspacePackages.length) {
    return full('invalid or duplicated Cargo package identities');
  }
  const packages = new Set(workspacePackages);
  if (impact.tests.some(t => typeof t !== 'string') ||
      impact.build.some(id => typeof id !== 'string')) {
    return full('invalid test or build node identity');
  }
  if (impact.tests.some(id => id.startsWith('test:'))) {
    return full('unresolved logical test group was selected as an executable check');
  }
  const candidateTests = impact.tests
    .filter(id => id.startsWith('cargo-test:')).map(id => id.slice('cargo-test:'.length));
  if (candidateTests.some(id => !packages.has(id))) {
    return full('unknown Cargo test package in impact selection');
  }
  const selectedTestPackages = sorted(candidateTests);
  const selectedBuildPackages = sorted(impact.build.filter(id => packages.has(id)));
  if (selectedTestPackages.some(id => !selectedBuildPackages.includes(id))) {
    return full('selected test package is missing from build closure');
  }
  const hasWorkspaceTest = impact.tests.includes('check:workspace-tests');
  // The full workspace test is indivisible today. Translating it into -p
  // requires a complete set of per-package test selections.
  if (hasWorkspaceTest !== (selectedTestPackages.length > 0)) {
    return full('workspace test selection disagrees with per-package test targets');
  }

  const build = selectedBuildPackages.length
    ? ['cargo', 'build', '--locked', '--offline', '-j', '4', ...packageArgs(selectedBuildPackages)]
    : null;
  const test = selectedTestPackages.length
    ? ['cargo', 'test', '--locked', '--offline', '-j', '4', ...packageArgs(selectedTestPackages)]
    : null;
  const clippy = selectedTestPackages.length
    ? ['cargo', 'clippy', '--all-targets', '--locked', '--offline',
      '-j', '4', ...packageArgs(selectedTestPackages), '--', '-D', 'warnings']
    : null;
  return { mode: 'scoped-preview', advisoryOnly: true, reason: null,
    selectedBuildPackages, selectedTestPackages, commands: {
      // Cheap repository-wide formatting stays global in any proposed partial mode.
      format: impact.tests.length || impact.build.length
        ? ['cargo', 'fmt', '--all', '--check'] : null,
      build, clippy, test,
    } };
}
