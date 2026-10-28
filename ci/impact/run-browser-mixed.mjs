// Advisory-only composite rehearsal: real Rust layout + offline browser oracles.
// Browser engine smoke remains an independent workflow, NOT simulated here.
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { cargoModules } from './impact.mjs';
import { checkNodes, verifyCanonicalChecks } from './canonical.mjs';
import { browserLayoutPlan } from './browser-mixed.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const ciDir = dirname(fileURLToPath(import.meta.url));
const readCatalog = name => readdirSync(join(ciDir, name)).filter(x => x.endsWith('.json'))
  .sort().map(x => JSON.parse(readFileSync(join(ciDir, name, x), 'utf8')));
const browserWorkflow = name => readFileSync(join(root, '.github/workflows', name), 'utf8');

function run(command, args, { capture = false, timeout = 300000 } = {}) {
  const executable = command === 'node' ? process.execPath : command;
  const started = performance.now();
  console.log('MIXED START ' + JSON.stringify({ command, args }));
  const result = spawnSync(executable, args, {
    cwd: root, shell: false, timeout, maxBuffer: 16 * 1024 * 1024,
    encoding: capture ? 'utf8' : undefined,
    stdio: capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  });
  const elapsedMs = Math.round(performance.now() - started);
  console.log('\nMIXED END ' + JSON.stringify({
    command, args, elapsedMs, status: result.status, signal: result.signal,
    error: result.error?.code ?? null,
  }));
  if (result.error || result.signal || result.status !== 0) {
    throw new Error(command + ' failed: ' + (result.error?.message ?? result.signal ?? result.status) +
      (capture ? '\n' + result.stderr : ''));
  }
  return capture ? result.stdout : elapsedMs;
}

function expandNodeArgs(args) {
  if (args.length === 2 && args[0] === '--test' && args[1] === 'tests/browser-oracles/*.test.mjs') {
    const folder = 'tests/browser-oracles';
    const entries = readdirSync(join(root, folder)).filter(x => x.endsWith('.test.mjs')).sort();
    if (!entries.length) throw new Error('browser oracle contract inventory unexpectedly empty');
    return ['--test', ...entries.map(x => folder + '/' + x)];
  }
  if (args.length === 2 && args[0] === '--test' &&
      args[1] === 'tools/browser-oracles/collect.test.mjs') return args;
  throw new Error('unexpected browser unit-test command');
}

if (process.argv.length !== 2) throw new Error('unexpected command-line input');
const metadata = JSON.parse(run('cargo',
  ['metadata', '--format-version', '1', '--no-deps', '--offline'], { capture: true, timeout: 20000 }));
const workspace = metadata.packages.filter(p => metadata.workspace_members.includes(p.id));
const cargoNames = workspace.map(p => p.name);
const modules = [...cargoModules(metadata, root), ...readCatalog('owners')];
const groups = readCatalog('targets');
const canonical = readCatalog('checks');
const observed = JSON.parse(run('node', ['ci/impact/collect-checks.mjs'],
  { capture: true, timeout: 20000 }));
const correspondence = verifyCanonicalChecks(observed, canonical);
const checks = [...workspace.map(pkg => ({
  id: 'cargo-test:' + pkg.name, dependsOn: [pkg.name],
})), ...checkNodes(canonical, cargoNames)];
const workflows = Object.fromEntries(['browser-oracles.yml', 'browser-collector.yml',
  'browser-smoke.yml'].map(name => [name, browserWorkflow(name)]));
const plan = browserLayoutPlan({
  modules, groups, tests: checks, packages: cargoNames, workflows,
});
if (plan.full) throw new Error('must use full CI: ' + plan.reason);
const revision = run('git', ['rev-parse', 'HEAD'], { capture: true, timeout: 10000 }).trim();
console.log('MIXED PLAN ' + JSON.stringify({
  advisoryOnly: true, revision,
  correspondence, changedPaths: plan.changedPaths, baselineFallback: plan.baselineFallback,
  tests: plan.selectedTests, build: plan.selectedBuildModules,
  preservedNotExecuted: plan.preservedNotExecuted,
}));

const receipts = [];
for (const [kind, cmd] of Object.entries(plan.cargoCommands)) {
  if (!cmd) throw new Error('missing required Cargo command ' + kind);
  const elapsedMs = run(cmd[0], cmd.slice(1));
  receipts.push({ kind: 'cargo-' + kind, elapsedMs });
}
for (const cmd of plan.offlineBrowserCommands) {
  if (cmd[0] !== 'node') throw new Error('unsupported browser test runner');
  const elapsedMs = run(cmd[0], expandNodeArgs(cmd.slice(1)));
  receipts.push({ kind: 'browser-' + (cmd[2].includes('tests/browser') ? 'oracles' : 'collector'),
    elapsedMs });
}
console.log('PASS advisory mixed browser/layout rehearsal ' + JSON.stringify({
  revision, correspondence, receipts,
  elapsedMs: receipts.reduce((sum, x) => sum + x.elapsedMs, 0),
  preservedNotExecuted: plan.preservedNotExecuted,
  limitation: 'historical mixed PR paths on current source; no Playwright engine execution',
}));
