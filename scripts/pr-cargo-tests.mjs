// Bounded PR Cargo coverage. Full master CI still runs cargo test --workspace.
// Classify every runtime integration target before executing any command so a
// newly added matrix cannot silently enter, or disappear from, the PR profile.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { commandTimeoutMs } from './check-catalog.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const classificationPath = new URL('../ci/impact/pr-cargo-tests.json', import.meta.url);
const runtimePackage = 'grunkalunka-runtime';
const identifier = /^[A-Za-z0-9_][A-Za-z0-9_-]*$/;
const sorted = values => [...values].sort();
const keys = value => value && !Array.isArray(value) && typeof value === 'object'
  ? Object.keys(value).sort().join(',') : null;

export function parsePackageArgs(args) {
  if (!Array.isArray(args) || args.length % 2 !== 0) {
    throw new Error('usage: node scripts/pr-cargo-tests.mjs [--package NAME ...]');
  }
  const packages = [];
  for (let index = 0; index < args.length; index += 2) {
    if (args[index] !== '--package' || typeof args[index + 1] !== 'string' ||
        !identifier.test(args[index + 1])) {
      throw new Error('usage: node scripts/pr-cargo-tests.mjs [--package NAME ...]');
    }
    if (packages.includes(args[index + 1])) throw new Error('duplicate selected Cargo package');
    packages.push(args[index + 1]);
  }
  return packages.length ? packages : null;
}

function workspacePackages(metadata) {
  if (!metadata || !Array.isArray(metadata.packages) ||
      !Array.isArray(metadata.workspace_members) || !metadata.workspace_members.length ||
      metadata.workspace_members.some(id => typeof id !== 'string' || !id) ||
      new Set(metadata.workspace_members).size !== metadata.workspace_members.length) {
    throw new Error('invalid Cargo workspace metadata');
  }
  const byId = new Map();
  for (const pkg of metadata.packages) {
    if (!pkg || typeof pkg.id !== 'string' || !pkg.id || byId.has(pkg.id) ||
        typeof pkg.name !== 'string' || !identifier.test(pkg.name)) {
      throw new Error('invalid or duplicate Cargo package metadata');
    }
    byId.set(pkg.id, pkg);
  }
  const workspace = metadata.workspace_members.map(id => {
    const pkg = byId.get(id);
    if (!pkg) throw new Error('missing Cargo workspace member: ' + id);
    return pkg;
  });
  if (new Set(workspace.map(pkg => pkg.name)).size !== workspace.length) {
    throw new Error('duplicate Cargo workspace package name');
  }
  return workspace;
}

function validateRuntimeClassification(runtime, classification) {
  if (keys(classification) !== 'fast,masterOnly,package,schemaVersion' ||
      classification.schemaVersion !== 1 || classification.package !== runtimePackage) {
    throw new Error('invalid PR Cargo classification');
  }
  if (!runtime || !Array.isArray(runtime.targets) ||
      runtime.targets.some(target => !target || typeof target.name !== 'string' ||
        !identifier.test(target.name) || !Array.isArray(target.kind) || !target.kind.length ||
        target.kind.some(kind => typeof kind !== 'string' || !kind))) {
    throw new Error('missing or invalid runtime Cargo targets');
  }
  if (!runtime.targets.some(target => target.kind.includes('lib'))) {
    throw new Error('runtime library target is missing');
  }
  const actual = runtime.targets.filter(target => target.kind.includes('test'))
    .map(target => target.name);
  if (new Set(actual).size !== actual.length) throw new Error('duplicate runtime test target');
  const seen = new Set();
  for (const group of ['fast', 'masterOnly']) {
    const entry = classification[group];
    if (keys(entry) !== 'reason,targets' ||
        typeof entry.reason !== 'string' || !entry.reason.trim() ||
        !Array.isArray(entry.targets) || !entry.targets.length) {
      throw new Error('invalid PR Cargo target group: ' + group);
    }
    for (const name of entry.targets) {
      if (typeof name !== 'string' || !identifier.test(name) || seen.has(name)) {
        throw new Error('invalid or duplicate classified runtime target: ' + name);
      }
      seen.add(name);
    }
  }
  const missing = actual.filter(name => !seen.has(name));
  const stale = [...seen].filter(name => !actual.includes(name));
  if (missing.length || stale.length) {
    throw new Error('runtime PR target classification is incomplete: ' +
      JSON.stringify({ unclassified: sorted(missing), stale: sorted(stale) }));
  }
  return structuredClone(classification);
}

export function planPrCargoTests({ metadata, classification, packages = null }) {
  const workspace = workspacePackages(metadata);
  const policy = validateRuntimeClassification(workspace.find(pkg => pkg.name === runtimePackage), classification);
  const names = sorted(workspace.map(pkg => pkg.name));
  if (packages !== null && (!Array.isArray(packages) || !packages.length ||
      packages.some(name => typeof name !== 'string' || !names.includes(name)) ||
      new Set(packages).size !== packages.length)) {
    throw new Error('unknown, empty or duplicated selected Cargo packages');
  }
  const selectedPackages = packages === null ? names : sorted(packages);
  const others = selectedPackages.filter(name => name !== runtimePackage);
  const groups = [];
  const base = ['test', '--locked', '--offline', '-j', '4'];
  if (others.length) groups.push([...base, ...others.flatMap(name => ['-p', name])]);
  if (selectedPackages.includes(runtimePackage)) {
    // The compiled-values integration target dominates the Windows execution
    // budget when grouped with other integration binaries. Run it independently,
    // still within the original two-minute limit and with no coverage removed.
    // Split the other fast targets deterministically; library tests run once.
    const fast = policy.fast.targets;
    const heavyweight = fast.includes('compiled-values') ? ['compiled-values'] : [];
    const ordinary = fast.filter(name => name !== 'compiled-values');
    const split = ordinary.length > 12 ? Math.ceil(ordinary.length / 2) : ordinary.length;
    groups.push([...base, '-p', runtimePackage, '--lib',
      ...ordinary.slice(0, split).flatMap(name => ['--test', name])]);
    if (heavyweight.length) {
      groups.push([...base, '-p', runtimePackage, '--test', 'compiled-values']);
    }
    if (split < ordinary.length) {
      groups.push([...base, '-p', runtimePackage,
        ...ordinary.slice(split).flatMap(name => ['--test', name])]);
    }
  }
  // Link only the selected test binaries before starting the execution clock.
  // There is no retry or expansion to the exhaustive test suite after failure.
  const commands = ['compile', 'test'].flatMap(phase => groups.map(args => {
    const commandArgs = phase === 'compile' ? [...args, '--no-run'] : [...args];
    return { phase, command: 'cargo', args: commandArgs,
      timeoutMs: commandTimeoutMs('cargo', commandArgs, 'pr') };
  }));
  return { profile: 'pr', selectedPackages, runtime: policy, commands };
}

export function runPrCargoTests(plan, execute) {
  if (!plan || plan.profile !== 'pr' || !Array.isArray(plan.commands) ||
      !plan.commands.length || typeof execute !== 'function') {
    throw new Error('invalid PR Cargo execution plan');
  }
  const receipts = [];
  for (const { phase, command, args, timeoutMs } of plan.commands) {
    if (!['compile', 'test'].includes(phase) || command !== 'cargo' ||
        !Array.isArray(args) || args[0] !== 'test' ||
        args.includes('--no-run') !== (phase === 'compile') ||
        timeoutMs !== commandTimeoutMs(command, args, 'pr')) {
      throw new Error('invalid PR Cargo command budget');
    }
    const elapsedMs = execute(command, args, { timeout: timeoutMs });
    receipts.push({ phase, command, args: [...args], elapsedMs, status: 'passed' });
  }
  return receipts;
}

function invoke(command, args, { capture = false, timeout } = {}) {
  const result = spawnSync(command, args, {
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

function main() {
  const packages = parsePackageArgs(process.argv.slice(2));
  const classification = JSON.parse(readFileSync(classificationPath, 'utf8'));
  const metadata = JSON.parse(invoke('cargo',
    ['metadata', '--format-version', '1', '--no-deps', '--locked', '--offline'],
    { capture: true, timeout: 30_000 }));
  const plan = planPrCargoTests({ metadata, classification, packages });
  console.log('PR CARGO PLAN ' + JSON.stringify(plan));
  const receipts = runPrCargoTests(plan, (command, args, options) => {
    const started = performance.now();
    console.log('PR CARGO START ' + JSON.stringify({ command, args, timeoutMs: options.timeout }));
    invoke(command, args, options);
    const elapsedMs = Math.round(performance.now() - started);
    console.log('\nPR CARGO END ' + JSON.stringify({ command, args, elapsedMs, status: 'passed' }));
    return elapsedMs;
  });
  console.log('PASS PR CARGO ' + JSON.stringify({ profile: 'pr', receipts }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
