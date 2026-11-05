// Cross-platform PR impact gate: selectively run the validated dependency plan
// on native Linux, Windows and macOS. PR fallbacks run the complete bounded PR
// profile; other events retain the full canonical suite.
// Only planning failures fall back. A selected check failure is always fatal.
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { commandTimeoutMs, projectPrCommands } from '../../scripts/check-catalog.mjs';
import { verifyCanonicalChecks, verifyPrChecks } from './canonical.mjs';
import { previewCargoScope } from './cargo-scope.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const directory = dirname(fileURLToPath(import.meta.url));
const fullCommands = [
  ['cargo', 'build', '--workspace', '--locked', '--offline', '-j', '4'],
  ['node', 'scripts/check.mjs'],
];
const prCommands = [fullCommands[0], ['node', 'scripts/check.mjs', '--profile=pr']];
const prCargoTests = { command: 'node', args: ['scripts/pr-cargo-tests.mjs'] };
const workspaceChecks = Object.freeze({
  'check:workspace-format': 'format',
  'check:workspace-clippy': 'clippy',
  'check:workspace-tests': 'test',
});
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const sorted = values => [...values].sort();
const nativePlatforms = new Set(['Linux', 'Windows', 'macOS']);
const selectivePr = (eventName, runnerOS, enabled) =>
  eventName === 'pull_request' && nativePlatforms.has(runnerOS) && enabled === true;
const eventProfile = eventName => eventName === 'pull_request' ? 'pr' : 'full';
const completeCommands = profile => profile === 'pr' ? prCommands : fullCommands;

function fallback(reason, profile) {
  return { mode: 'full', profile, reason, commands: structuredClone(completeCommands(profile)) };
}

// No path, package or workstream whitelist: the real plan owns selection.
// The canonical check inventory and Cargo metadata independently guard the
// translation from advisory check IDs into executable commands.
export function choosePrChecks({
  eventName, runnerOS, enabled, plan, declared, observed, prObserved, workspacePackages,
}) {
  const profile = eventProfile(eventName);
  if (!selectivePr(eventName, runnerOS, enabled)) {
    return fallback('only enabled native-platform pull requests use partial verification', profile);
  }
  if (!plan || plan.mode !== 'shadow' || plan.advisoryOnly !== true ||
      plan.full !== false || !Array.isArray(plan.reasons) || plan.reasons.length) {
    return fallback('planner requires complete verification', profile);
  }
  try {
    if (!Array.isArray(plan.changedPaths) || !plan.changedPaths.length ||
        new Set(plan.changedPaths).size !== plan.changedPaths.length ||
        !Array.isArray(plan.tests) || !Array.isArray(plan.build) ||
        !Array.isArray(plan.selectedCommands) || !Array.isArray(declared) ||
        !Array.isArray(observed) || !Array.isArray(workspacePackages)) {
      throw new Error('missing or duplicated impact inputs');
    }
    const coverage = verifyCanonicalChecks(observed, declared);
    verifyPrChecks(prObserved, declared);
    if (!same(coverage, plan.canonicalCoverage)) {
      throw new Error('planner command inventory differs from live canonical runner');
    }
    const external = plan.externalWorkflows;
    if (!external || external.valid !== true ||
        !same(external.errors, []) || !same(external.requiredNotTriggered, [])) {
      throw new Error('independent browser workflow trigger coverage is incomplete');
    }

    const checks = plan.tests.filter(id => id.startsWith('check:'));
    const cargoTests = plan.tests.filter(id => id.startsWith('cargo-test:'));
    if (!plan.tests.every(id => typeof id === 'string' &&
        (id.startsWith('check:') || id.startsWith('cargo-test:'))) ||
        !same(plan.selectedCommands, sorted(checks)) ||
        new Set(plan.tests).size !== plan.tests.length ||
        new Set(plan.selectedCommands).size !== plan.selectedCommands.length ||
        new Set(plan.build).size !== plan.build.length ||
        new Set(workspacePackages).size !== workspacePackages.length) {
      throw new Error('unrecognized, duplicated or incomplete selected test nodes');
    }
    if (cargoTests.some(id => !workspacePackages.includes(id.slice('cargo-test:'.length)))) {
      throw new Error('unknown selected Cargo test package');
    }

    const preview = previewCargoScope(plan, workspacePackages);
    if (preview.mode !== 'scoped-preview' || preview.advisoryOnly !== true) {
      throw new Error(preview.reason || 'Cargo package scope unavailable');
    }
    const byKey = new Map(declared.map(d => [
      JSON.stringify([d.command, ...d.args]), d,
    ]));
    const selected = new Set(plan.selectedCommands);
    const mapped = new Set();
    const commands = [];
    if (preview.commands.build) commands.push(preview.commands.build);

    // Preserve the original canonical command order for the selected scripts,
    // while replacing only the three virtual workspace-wide Cargo commands.
    for (const observedCheck of observed) {
      const descriptor = byKey.get(JSON.stringify([
        observedCheck.command, ...observedCheck.args,
      ]));
      if (!descriptor || !selected.has(descriptor.id)) continue;
      if (mapped.has(descriptor.id)) throw new Error('duplicated selected command mapping');
      mapped.add(descriptor.id);
      const projected = projectPrCommands(descriptor);
      if (Object.hasOwn(workspaceChecks, descriptor.id)) {
        if (descriptor.scope !== 'workspace') {
          throw new Error('workspace Cargo descriptor lost its scope');
        }
        if (descriptor.id === 'check:workspace-tests') {
          if (!same(projected, [prCargoTests]) || !preview.selectedTestPackages.length) {
            throw new Error('workspace tests lost their bounded PR helper');
          }
          commands.push([prCargoTests.command, ...prCargoTests.args,
            ...preview.selectedTestPackages.flatMap(name => ['--package', name])]);
          continue;
        }
        if (!same(projected, [{ command: descriptor.command, args: descriptor.args }])) {
          throw new Error('unsupported workspace Cargo PR projection: ' + descriptor.id);
        }
        const replacement = preview.commands[workspaceChecks[descriptor.id]];
        if (!replacement) throw new Error('missing scoped Cargo replacement: ' + descriptor.id);
        commands.push(replacement);
      } else {
        if (descriptor.scope === 'workspace') {
          throw new Error('unrecognized workspace-wide command: ' + descriptor.id);
        }
        commands.push(...projected.map(({ command, args }) => [command, ...args]));
      }
    }
    if (mapped.size !== selected.size) {
      throw new Error('incomplete or duplicated selected command mapping');
    }
    return {
      mode: 'partial',
      profile,
      reason: 'bounded PR variants of selected canonical checks and scoped Cargo packages',
      changedPaths: [...plan.changedPaths],
      selectedTests: [...plan.tests],
      selectedPackages: [...preview.selectedTestPackages],
      commands: structuredClone(commands),
      independentWorkflowsExpected: [...(external.expectedToTrigger ?? [])],
      // Independent browser workflows retain separate triggers; P1 tests are canonical.
      externalJobsVerified: false,
    };
  } catch (error) {
    return fallback('unsafe partial plan: ' + error.message, profile);
  }
}

function invoke(command, args, { capture = false, timeout = 30_000 } = {}) {
  const result = spawnSync(command === 'node' ? process.execPath : command, args, {
    cwd: root, shell: false, timeout, maxBuffer: 16 * 1024 * 1024,
    ...(timeout ? { killSignal: 'SIGKILL' } : {}),
    encoding: capture ? 'utf8' : undefined,
    stdio: capture ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error(command + ' ' + args.join(' ') + ' failed: ' +
      (result.error?.message ?? result.signal ?? result.status) +
      (capture ? '\n' + result.stderr : ''));
  }
  return capture ? result.stdout : null;
}

const readCatalog = name => readdirSync(join(directory, name))
  .filter(file => file.endsWith('.json')).sort()
  .map(file => JSON.parse(readFileSync(join(directory, name, file), 'utf8')));

function captureInputs() {
  const plan = JSON.parse(invoke('node', ['ci/impact/plan.mjs', 'origin/master'],
    { capture: true }));
  const metadata = JSON.parse(invoke('cargo',
    ['metadata', '--format-version', '1', '--no-deps', '--offline'], { capture: true }));
  const packages = metadata.packages.filter(pkg => metadata.workspace_members.includes(pkg.id));
  return {
    plan,
    declared: readCatalog('checks'),
    observed: JSON.parse(invoke('node', ['ci/impact/collect-checks.mjs'], { capture: true })),
    prObserved: JSON.parse(invoke('node', ['ci/impact/collect-checks.mjs', '--profile=pr'],
      { capture: true })),
    workspacePackages: packages.map(pkg => pkg.name),
  };
}

export function chooseWithFallback({ eventName, runnerOS, enabled, capture = captureInputs }) {
  const profile = eventProfile(eventName);
  if (!selectivePr(eventName, runnerOS, enabled)) {
    return fallback('only enabled native-platform pull requests use partial verification', profile);
  }
  try {
    return choosePrChecks({ eventName, runnerOS, enabled, ...capture() });
  } catch (error) {
    return fallback('impact planning unavailable: ' + error.message, profile);
  }
}

export function runDecision(decision, execute) {
  if (!decision || !['full', 'partial'].includes(decision.mode) ||
      !['full', 'pr'].includes(decision.profile) ||
      !Array.isArray(decision.commands) || typeof execute !== 'function') {
    throw new Error('invalid CI decision');
  }
  if (decision.mode === 'full' && !same(decision.commands, completeCommands(decision.profile))) {
    throw new Error('full CI command contract was modified');
  }
  const receipts = [];
  for (const entry of decision.commands) {
    if (!Array.isArray(entry) || !['cargo', 'node'].includes(entry[0]) ||
        entry.length < 2 || !entry.every(a => typeof a === 'string' && a.length)) {
      throw new Error('invalid selected executable command');
    }
    const [command, ...args] = entry;
    // Aggregate runners own their per-command bounds; the hosted job owns the
    // total deadline. Every PR planning fallback uses the bounded PR profile.
    const fullAggregate = decision.mode === 'full' &&
      same(entry, completeCommands(decision.profile)[1]);
    const timeout = fullAggregate ? 0 : decision.profile === 'pr'
      ? commandTimeoutMs(command, args, 'pr') : 45 * 60_000;
    const elapsedMs = execute(command, args, { timeout });
    receipts.push({ command, args, elapsedMs, status: 'passed' });
    // Failure propagates. Do not retry with full CI after a partial test fails.
  }
  return receipts;
}

function execute(command, args, options) {
  const started = performance.now();
  console.log('PR CI START ' + JSON.stringify({ command, args, timeoutMs: options.timeout }));
  invoke(command, args, options);
  const elapsedMs = Math.round(performance.now() - started);
  console.log('\nPR CI END ' + JSON.stringify({ command, args, elapsedMs, status: 'passed' }));
  return elapsedMs;
}

function main() {
  if (process.argv.length !== 2) {
    throw new Error('usage: node ci/impact/pr-gate.mjs');
  }
  const decision = chooseWithFallback({
    eventName: process.env.GITHUB_EVENT_NAME,
    runnerOS: process.env.RUNNER_OS,
    enabled: process.env.PARTIAL_PR_CI === 'true',
  });
  console.log('PR CI DECISION ' + JSON.stringify({ runnerOS: process.env.RUNNER_OS, ...decision }));
  const receipts = runDecision(decision, execute);
  console.log('PASS PR CI ' + JSON.stringify({
    runnerOS: process.env.RUNNER_OS,
    mode: decision.mode, profile: decision.profile, receipts,
    externalJobsVerified: decision.externalJobsVerified ?? false,
  }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
