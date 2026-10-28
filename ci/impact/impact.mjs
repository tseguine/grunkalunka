// Pure, conservative impact calculation. Graph edges point to dependencies.
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';

function repoPath(root, file) {
  const path = relative(root, file).split(sep).join('/');
  if (path === '..' || path.startsWith('../') || isAbsolute(path)) {
    throw new Error(`manifest outside repository: ${file}`);
  }
  return path;
}

export function cargoModules(metadata, root) {
  const members = new Set(metadata.workspace_members);
  const packages = metadata.packages.filter(pkg => members.has(pkg.id));
  const byDirectory = new Map(packages.map(pkg => [resolve(dirname(pkg.manifest_path)), pkg.name]));
  const unique = new Set();
  return packages.map(pkg => {
    if (unique.has(pkg.name)) throw new Error(`duplicate Cargo package ${pkg.name}`);
    unique.add(pkg.name);
    const path = repoPath(root, dirname(pkg.manifest_path));
    const dependencies = pkg.dependencies.flatMap(dep => {
      if (!dep.path) return [];
      const id = byDirectory.get(resolve(dep.path));
      // Path dependencies outside the workspace do not have CI-owned modules.
      return id ? [id] : [];
    });
    return { id: pkg.name, paths: [path + '/'], dependsOn: [...new Set(dependencies)] };
  });
}

function matches(path, pattern) {
  return pattern.endsWith('/') ? path.startsWith(pattern) : path === pattern;
}

function validatePath(path) {
  return typeof path === 'string' && path.length > 0 && !path.startsWith('/') &&
    !path.includes('\\') && !path.includes('\0') && !path.split('/').includes('..') &&
    !path.split('/').includes('.');
}

function closure(start, edges) {
  const visited = new Set(start), stack = [...visited];
  while (stack.length) {
    const node = stack.pop();
    for (const next of edges.get(node) ?? []) {
      if (visited.has(next)) continue;
      visited.add(next);
      stack.push(next);
    }
  }
  return visited;
}

// All tests are planned in shadow mode; no skip decision is authoritative yet.
export function planImpact({ changedPaths, modules, groups = [], tests, ignored = [], global = [] }) {
  const definitions = [
    ...modules.map(node => ({ ...node, kind: 'module' })),
    ...groups.map(node => ({ ...node, kind: 'group' })),
    ...tests.map(node => ({ ...node, kind: 'test' })),
  ];
  const nodes = new Map();
  const failure = reason => ({ full: true, reasons: [reason], changedPaths,
    tests: tests.map(x => x.id).sort(), build: modules.map(x => x.id).sort(),
    changedOwners: [], ignoredPaths: [] });
  for (const node of definitions) {
    if (typeof node.id !== 'string' || !node.id || nodes.has(node.id)) {
      return failure(`invalid or duplicate module/test ID: ${node.id}`);
    }
    if (!Array.isArray(node.paths ?? []) || !Array.isArray(node.dependsOn ?? []) ||
        (node.buildDependsOn !== undefined && !Array.isArray(node.buildDependsOn))) {
      return failure(`invalid ownership or dependency list: ${node.id}`);
    }
    if ([...(node.paths ?? []), ...(node.dependsOn ?? []), ...(node.buildDependsOn ?? [])]
        .some(x => typeof x !== 'string' || !x)) {
      return failure(`invalid entry in ${node.id}`);
    }
    if ((node.paths ?? []).some(p => !validatePath(p.endsWith('/') ? p.slice(0, -1) : p))) {
      return failure(`invalid owner path: ${node.id}`);
    }
    nodes.set(node.id, node);
  }
  // Reverse selection edges decide which tests are affected. Forward build
  // edges decide which modules the *selected* checks actually need compiled.
  // A workspace-wide check is a virtual selection umbrella in partial mode:
  // its replacement `cargo -p` checks do not themselves build every crate.
  const forward = new Map(), reverse = new Map();
  for (const node of definitions) {
    const selectionDeps = node.dependsOn ?? [];
    const buildDeps = node.buildDependsOn ?? selectionDeps;
    for (const dep of [...selectionDeps, ...buildDeps]) {
      if (!nodes.has(dep)) return failure(`unknown dependency ${dep} from ${node.id}`);
    }
    forward.set(node.id, new Set(buildDeps));
    for (const dep of selectionDeps) {
      if (!reverse.has(dep)) reverse.set(dep, new Set());
      reverse.get(dep).add(node.id);
    }
  }
  if (!Array.isArray(changedPaths) || !Array.isArray(ignored) || !Array.isArray(global)) {
    return failure('invalid change/path-policy list');
  }
  const changedOwners = new Set(), ignoredPaths = [];
  for (const path of changedPaths) {
    if (!validatePath(path)) return failure(`invalid Git path: ${path}`);
    if (global.some(rule => matches(path, rule)) || /(?:^|\/)Cargo\.(?:toml|lock)$/.test(path)) {
      return failure(`global build input changed: ${path}`);
    }
    if (ignored.some(rule => matches(path, rule))) {
      ignoredPaths.push(path);
      continue;
    }
    const matching = definitions.flatMap(node => (node.paths ?? [])
      .filter(rule => matches(path, rule)).map(rule => ({ id: node.id, specificity: rule.length })));
    if (!matching.length) return failure(`unattributed code change: ${path}`);
    // Exact files override coarse directory owners; equally specific conflicts fail closed.
    const best = Math.max(...matching.map(owner => owner.specificity));
    const owners = new Set(matching.filter(owner => owner.specificity === best).map(owner => owner.id));
    if (owners.size !== 1) return failure(`ambiguous code change: ${path}`);
    changedOwners.add([...owners][0]);
  }
  const upward = closure(changedOwners, reverse);
  const selectedTests = [...upward].filter(id => nodes.get(id).kind === 'test').sort();
  // Logical test groups connect dependencies but do not execute a check.
  // Each changed non-test owner must reach an actual executable test target.
  // This remains fail-closed even when some *other* owner has runnable tests.
  for (const id of changedOwners) {
    if (nodes.get(id).kind === 'test') continue;
    if (![...closure([id], reverse)].some(node => nodes.get(node).kind === 'test')) {
      return failure(`no executable test covers changed owner: ${id}`);
    }
  }
  const downward = closure([...changedOwners, ...selectedTests], forward);
  const requiredBuild = [...downward].filter(id => nodes.get(id).kind === 'module').sort();
  return { full: false, reasons: [], changedPaths, changedOwners: [...changedOwners].sort(),
    tests: selectedTests, build: requiredBuild, ignoredPaths };
}
