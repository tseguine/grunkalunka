// Real but non-gating execution of a deliberately fixed, narrow change scenario.
// This does not use PR contents to decide what may be skipped. Full CI still runs.
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { cargoModules, planImpact } from './impact.mjs';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { previewCargoScope } from './cargo-scope.mjs';
import { SAMPLE_CHANGES, validateLayoutSelection } from './validate-layout-selection.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const directory = dirname(fileURLToPath(import.meta.url));
const readCatalog = name => readdirSync(join(directory, name))
  .filter(file => file.endsWith('.json')).sort()
  .map(file => JSON.parse(readFileSync(join(directory, name, file), 'utf8')));

function invoke(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root, shell: false,
    timeout: options.timeout ?? 20_000,
    encoding: options.capture ? 'utf8' : undefined,
    stdio: options.capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error(command + ' ' + args.join(' ') + ' failed: ' +
      (result.error?.message ?? result.signal ?? result.status) +
      (options.capture ? '\n' + result.stderr : ''));
  }
  return result.stdout;
}

async function main() {
  if (process.argv.length !== 2) throw new Error('unexpected CLI arguments');
  const metadata = JSON.parse(invoke('cargo',
    ['metadata', '--format-version', '1', '--no-deps', '--offline'], { capture: true }));
  const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
  const packageNames = packages.map(pkg => pkg.name);
  const modules = [...cargoModules(metadata, root), ...readCatalog('owners')];
  const groups = readCatalog('targets');
  const declared = readCatalog('checks');
  const observed = JSON.parse(invoke(process.execPath,
    ['ci/impact/collect-checks.mjs'], { capture: true }));
  const canonicalCoverage = verifyCanonicalChecks(observed, declared);
  const tests = [
    ...packages.map(pkg => ({ id: 'cargo-test:' + pkg.name, dependsOn: [pkg.name] })),
    ...checkNodes(declared, packageNames),
  ];
  // Exact changed-file list from merged #75, not a claimed edit in this PR.
  const impact = planImpact({
    changedPaths: [...SAMPLE_CHANGES], modules, groups, tests,
    global: ['.github/', 'ci/impact/', 'Cargo.toml', 'Cargo.lock'],
    ignored: ['docs/'],
  });
  const preview = previewCargoScope(impact, packageNames);
  const commands = validateLayoutSelection(impact, preview);
  const revision = invoke('git', ['rev-parse', 'HEAD'], { capture: true }).trim();
  console.log('SCOPED SAMPLE PLAN ' + JSON.stringify({
    advisoryOnly: true, revision,
    scenario: 'merged PR #75 layout change list', changedPaths: SAMPLE_CHANGES, canonicalCoverage,
    changedOwners: impact.changedOwners,
    selectedTests: impact.tests, selectedBuildPackages: preview.selectedBuildPackages,
    selectedTestPackages: preview.selectedTestPackages,
  }));
  const results = [];
  for (const [kind, args] of Object.entries(commands)) {
    const command = args[0];
    console.log('\nSCOPED START ' + JSON.stringify({ kind, command, args: args.slice(1) }));
    const started = performance.now();
    invoke(command, args.slice(1), { timeout: 300_000 });
    const elapsedMs = Math.round(performance.now() - started);
    results.push({ kind, elapsedMs, status: 'passed' });
    console.log('\nSCOPED END ' + JSON.stringify({ kind, elapsedMs, status: 'passed' }));
  }
  console.log('PASS scoped layout sample ' + JSON.stringify({
    revision, scenario: 'merged PR #75 layout change list', changedPaths: SAMPLE_CHANGES, canonicalCoverage,
    results, totalElapsedMs: results.reduce((sum, item) => sum + item.elapsedMs, 0),
    limitation: 'historical PR #75 changed paths replayed against current code; full CI authoritative',
  }));
}

await main();
