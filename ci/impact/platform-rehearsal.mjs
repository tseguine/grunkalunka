// Real native-platform execution of a historic layout impact selection against
// current source. Validates the exact shared PR executor on Linux, Windows and
// macOS. The canonical full suite runs separately on the same PR.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { cargoModules, planImpact } from './impact.mjs';
import { cargoTestNodes } from './cargo-test-nodes.mjs';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { auditBrowserWorkflows } from './external-workflows.mjs';
import { choosePrChecks, runDecision } from './pr-gate.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const directory = dirname(fileURLToPath(import.meta.url));
const readCatalog = folder => readdirSync(join(directory, folder))
  .filter(file => file.endsWith('.json')).sort()
  .map(file => JSON.parse(readFileSync(join(directory, folder, file), 'utf8')));

function invoke(command, args, { capture = false, timeout = 300000 } = {}) {
  const started = performance.now();
  console.log('PLATFORM REHEARSAL START ' + JSON.stringify({ command, args }));
  const result = spawnSync(command === 'node' ? process.execPath : command, args, {
    cwd: root, shell: false, timeout,
    encoding: capture ? 'utf8' : undefined,
    stdio: capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    maxBuffer: 16 * 1024 * 1024,
  });
  const elapsedMs = Math.round(performance.now() - started);
  console.log('\nPLATFORM REHEARSAL END ' + JSON.stringify({
    command, args, elapsedMs, status: result.status, error: result.error?.code ?? null,
  }));
  if (result.error || result.signal || result.status !== 0) {
    throw new Error(command + ' failed: ' +
      (result.error?.message ?? result.signal ?? result.status) +
      (capture ? '\n' + result.stderr : ''));
  }
  return capture ? result.stdout : elapsedMs;
}

if (process.argv.length !== 2) throw new Error('unexpected CLI arguments');
const nativePlatform = process.env.RUNNER_OS;
if (!['Linux', 'Windows', 'macOS'].includes(nativePlatform)) {
  throw new Error('rehearsal requires a supported native CI platform');
}
const historicalPaths = [
  // Exact source-file changes in merged browser/layout PR #75.
  'crates/grunkalunka-layout/examples/click_reflow_ppm.rs',
  'crates/grunkalunka-layout/tests/headless_input_reflow.rs',
  'docs/headless-hit-test.md',
];
const metadata = JSON.parse(invoke('cargo',
  ['metadata', '--format-version', '1', '--no-deps', '--offline'], { capture: true }));
const packages = metadata.packages.filter(p => metadata.workspace_members.includes(p.id));
const workspacePackages = packages.map(p => p.name);
const declared = readCatalog('checks');
const observed = JSON.parse(invoke('node',
  ['ci/impact/collect-checks.mjs'], { capture: true }));
const coverage = verifyCanonicalChecks(observed, declared);
const modules = [...cargoModules(metadata, root), ...readCatalog('owners')];
const groups = readCatalog('targets');
const cargoTests = cargoTestNodes(packages, readCatalog('cargo-tests'));
const tests = [
  ...cargoTests,
  ...checkNodes(declared, workspacePackages, cargoTests.map(t => t.id)),
];
const externalWorkflows = auditBrowserWorkflows(historicalPaths, readCatalog('workflows'),
  file => readFileSync(join(root, file), 'utf8'));
const impact = planImpact({
  changedPaths: historicalPaths, modules, groups, tests,
  ignored: ['docs/'], global: ['.github/', 'ci/impact/', 'Cargo.toml', 'Cargo.lock'],
});
const plan = {
  mode: 'shadow', advisoryOnly: true, ...impact,
  canonicalCoverage: coverage, externalWorkflows,
  selectedCommands: impact.full ? declared.map(d => d.id).sort() :
    impact.tests.filter(id => id.startsWith('check:')),
};
const choice = choosePrChecks({
  eventName: 'pull_request', runnerOS: nativePlatform, enabled: true,
  plan, declared, observed, workspacePackages,
});
assert.equal(choice.mode, 'partial', choice.reason);
assert.deepEqual(choice.selectedPackages, ['grunkalunka-layout']);
assert.deepEqual(choice.commands, [
  ['cargo', 'build', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
  ['cargo', 'fmt', '--all', '--check'],
  ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '-j', '4',
    '-p', 'grunkalunka-layout', '--', '-D', 'warnings'],
  ['cargo', 'test', '--locked', '--offline', '-j', '4', '-p', 'grunkalunka-layout'],
]);
const revision = invoke('git', ['rev-parse', 'HEAD'], { capture: true }).trim();
console.log('PLATFORM REHEARSAL PLAN ' + JSON.stringify({
  revision, nativePlatform, advisoryOnly: true, historicalPaths,
  affectedTests: choice.selectedTests, selectedPackages: choice.selectedPackages,
  canonicalCoverage: coverage, externalWorkflows, commands: choice.commands,
}));
const receipts = runDecision(choice, (command, args) => invoke(command, args));
console.log('PASS native-platform partial rehearsal ' + JSON.stringify({
  revision, nativePlatform, receipts,
  elapsedMs: receipts.reduce((sum, r) => sum + r.elapsedMs, 0),
  caveat: 'historical changed paths replayed against current source; full PR CI is authoritative',
}));
