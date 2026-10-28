// Read-only PR impact forecast. It never changes the canonical CI gate.
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cargoModules, planImpact } from './impact.mjs';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const directory = dirname(fileURLToPath(import.meta.url));
const run = (exe, args) => execFileSync(exe, args, {
  cwd: root, maxBuffer: 16 * 1024 * 1024, encoding: 'utf8',
  timeout: 10000, stdio: ['ignore', 'pipe', 'pipe'],
});
const readCatalog = folder => readdirSync(join(directory, folder))
  .filter(x => x.endsWith('.json')).sort().map(x =>
    JSON.parse(readFileSync(join(directory, folder, x), 'utf8')));
const global = [
  '.github/', 'ci/impact/', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
  'package.json', '.gitignore', 'scripts/check.mjs', 'scripts/check-command.mjs',
];
const ignored = ['docs/', 'README.md', 'LICENSE', 'AGENTS.md'];

function changedAgainst(baseRef) {
  const base = run('git', ['merge-base', baseRef, 'HEAD']).trim();
  if (!/^[a-f0-9]{40}$/.test(base)) throw new Error('unusable merge-base');
  // With --no-renames, both sides of a rename are independently attributed.
  const raw = execFileSync('git', ['diff', '--name-only', '-z', '--no-renames', base, 'HEAD'],
    { cwd: root, timeout: 10000, maxBuffer: 16 * 1024 * 1024 });
  return raw.toString('utf8').split('\0').filter(Boolean);
}

const baseRef = process.argv[2];
let result;
try {
  if (!baseRef || baseRef.startsWith('-')) throw new Error('usage: node ci/impact/plan.mjs origin/master');
  const changes = changedAgainst(baseRef);
  const metadata = JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps', '--offline']));
  const workspacePackages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
  const modules = [...cargoModules(metadata, root), ...readCatalog('owners')];
  const declared = readCatalog('checks');
  const canonical = JSON.parse(run(process.execPath, ['ci/impact/collect-checks.mjs']));
  const coverage = verifyCanonicalChecks(canonical, declared);
  // Group entries are dependency junctions, not runnable tests.
  const groups = readCatalog('targets');
  const tests = [
    ...workspacePackages.map(pkg => ({ id: `cargo-test:${pkg.name}`, dependsOn: [pkg.name] })),
    ...checkNodes(declared, workspacePackages.map(pkg => pkg.name)),
  ];
  result = { ...planImpact({ changedPaths: changes, modules, groups, tests, global, ignored }),
    canonicalCoverage: coverage };
  result.selectedCommands = result.full ? declared.map(x => x.id).sort() :
    result.tests.filter(id => id.startsWith('check:'));
} catch (error) {
  // If Git, Cargo, canonical capture, or a descriptor is unavailable, select full by default.
  result = { full: true, reasons: [`planning unavailable: ${error.message}`],
    changedPaths: [], changedOwners: [], tests: ['all'], selectedCommands: ['all'],
    build: ['all'], ignoredPaths: [], canonicalCoverage: null };
}
console.log(JSON.stringify({ mode: 'shadow', advisoryOnly: true, ...result }, null, 2));
