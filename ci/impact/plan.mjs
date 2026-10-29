// Read-only PR impact forecast. It never changes the canonical CI gate.
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cargoModules, planImpact } from './impact.mjs';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { auditBrowserWorkflows } from './external-workflows.mjs';
import { scopedAddedChecks } from './added-checks.mjs';
import { auditSourceSelection } from './source-dependency-audit.mjs';

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
  'scripts/check-catalog.mjs', 'scripts/checks/',
];
const ignored = ['docs/', 'README.md', 'LICENSE', 'AGENTS.md'];

function changedAgainst(baseRef) {
  const base = run('git', ['merge-base', baseRef, 'HEAD']).trim();
  if (!/^[a-f0-9]{40}$/.test(base)) throw new Error('unusable merge-base');
  // With --no-renames, both sides of a rename are independently attributed.
  const names = extra => execFileSync('git',
    ['diff', '--name-only', '-z', '--no-renames', ...extra, base, 'HEAD'],
    { cwd: root, timeout: 10000, maxBuffer: 16 * 1024 * 1024 })
    .toString('utf8').split('\0').filter(Boolean);
  return { changedPaths: names([]), addedPaths: names(['--diff-filter=A']) };
}

const baseRef = process.argv[2];
let result;
try {
  if (!baseRef || baseRef.startsWith('-')) throw new Error('usage: node ci/impact/plan.mjs origin/master');
  const { changedPaths: changes, addedPaths } = changedAgainst(baseRef);
  // This models independent workflow *triggers*, not their hosted completion.
  // It covers the browser-oracle workflows only, not all optional workflows.
  const externalWorkflows = auditBrowserWorkflows(
    changes, readCatalog('workflows'),
    file => readFileSync(join(root, file), 'utf8'));
  const metadata = JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps', '--offline']));
  const workspacePackages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
  const modules = [...cargoModules(metadata, root), ...readCatalog('owners')];
  const declared = readCatalog('checks');
  const canonical = JSON.parse(run(process.execPath, ['ci/impact/collect-checks.mjs']));
  const coverage = verifyCanonicalChecks(canonical, declared);
  // A new test plus its per-command descriptor is safe to select directly.
  // Any edited, deleted, unpaired, or ambiguous registration fails closed.
  const scopedPaths = scopedAddedChecks({
    changedPaths: changes, addedPaths, declared, observed: canonical,
  });
  // Group entries are dependency junctions, not runnable tests.
  const groups = readCatalog('targets');
  const cargoTests = cargoTestNodes(workspacePackages, readCatalog('cargo-tests'));
  const tests = [
    ...cargoTests,
    ...checkNodes(declared, workspacePackages.map(pkg => pkg.name),
      cargoTests.map(test => test.id)),
  ];
  result = { ...planImpact({ changedPaths: changes, modules, groups, tests, global, ignored,
    scopedPaths }),
    canonicalCoverage: coverage, externalWorkflows };
  // Independently discover the actual literal JS/Rust file inputs in audited
  // consumer sources; graph descriptors cannot assert their own completeness.
  // A missing executable test edge forces full, even on unrelated PRs, until
  // its ownership/selection edges are corrected.
  const sourceDependencyAudit = auditSourceSelection({
    specs: readCatalog('source-audits'),
    readSource: file => readFileSync(join(root, file), 'utf8'),
    exists: file => existsSync(join(root, file)),
    planForPath: file => planImpact({ changedPaths: [file], modules, groups, tests,
      global, ignored }),
  });
  result.sourceDependencyAudit = sourceDependencyAudit;
  // An unavailable required external workflow is NOT made safe by a passing
  // full Cargo/Node suite. Report the gap and forbid a selective plan.
  const externalReasons = [
    ...externalWorkflows.errors.map(x => 'browser workflow contract drift: ' + x),
    ...externalWorkflows.requiredNotTriggered.map(x => 'required browser workflow not triggered: ' + x),
  ];
  const sourceReasons = sourceDependencyAudit.errors.map(x =>
    'source dependency under-selection: ' + x);
  if (externalReasons.length || sourceReasons.length) {
    result.full = true;
    result.reasons = [...new Set([...result.reasons, ...externalReasons, ...sourceReasons])];
    result.tests = tests.map(x => x.id).sort();
    result.build = modules.map(x => x.id).sort();
  }
  result.selectedCommands = result.full ? declared.map(x => x.id).sort() :
    result.tests.filter(id => id.startsWith('check:'));
} catch (error) {
  // If Git, Cargo, canonical capture, or a descriptor is unavailable, select full by default.
  result = { full: true, reasons: [`planning unavailable: ${error.message}`],
    changedPaths: [], changedOwners: [], tests: ['all'], selectedCommands: ['all'],
    build: ['all'], ignoredPaths: [], canonicalCoverage: null,
    externalWorkflows: null };
}
console.log(JSON.stringify({ mode: 'shadow', advisoryOnly: true, ...result }, null, 2));
