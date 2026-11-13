// Guard the checked-in PR routes and smoke coverage without running native
// fixtures or the exhaustive suites merely to inspect their selection policy.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';
import { commandTimeoutMs, loadCanonicalCommands, projectPrCommands } from '../../scripts/check-catalog.mjs';
import { planImpact } from './impact.mjs';

const read = path => readFileSync(new URL('../../' + path, import.meta.url), 'utf8')
  .replace(/\r\n/g, '\n');
const json = path => JSON.parse(read(path));
const full = loadCanonicalCommands();
const pr = loadCanonicalCommands(undefined, { profile: 'pr' });
const notePreview = 'tests/applications/note-preview.test.mjs';
const liveRun = 'tests/applications/a1-live-run.test.mjs';
const liveClient = 'tests/applications/a1-live-client.test.mjs';

// Deliberately accept the current literal YAML structure only. A new layout or
// expression must be reviewed instead of silently dropping an execution route.
function block(source, heading) {
  const lines = source.split('\n');
  const start = lines.indexOf(heading);
  assert.ok(start >= 0 && start === lines.lastIndexOf(heading), `one ${heading.trim()} block`);
  const indent = heading.length - heading.trimStart().length;
  const end = lines.findIndex((line, index) => index > start && line.trim() &&
    !line.trimStart().startsWith('#') && line.length - line.trimStart().length <= indent);
  return lines.slice(start + 1, end < 0 ? lines.length : end).join('\n');
}

function scalar(source, name, indent, optional = false) {
  const prefix = ' '.repeat(indent) + name + ': ';
  const values = source.split('\n').filter(line => line.startsWith(prefix))
    .map(line => line.slice(prefix.length));
  assert.ok(values.length === 1 || optional && values.length === 0, `one ${name} value`);
  return values[0];
}

function evaluate(value, { eventName, prChecks = false, fresh = 'false', probe = 'success' }) {
  let expression = value.trim().replace(/^\$\{\{\s*([\s\S]*?)\s*\}\}$/, '$1');
  for (const [name, replacement] of [
    ['github.event_name', eventName], ['inputs.pr-checks', prChecks],
    ['needs.macos-probe.outputs.fresh', fresh], ['needs.macos-probe.result', probe],
    ['cancelled()', false],
  ]) expression = expression.replaceAll(name, JSON.stringify(replacement));
  const operators = expression.replace(/'[^'\n]*'|"[^"\n]*"|\btrue\b|\bfalse\b/g, '');
  assert.ok(expression.length > 0, 'workflow condition must not be empty');
  assert.match(operators, /^[\s0-9!&|=()]*$/, 'unsupported workflow condition');
  return runInNewContext(expression, {}, { timeout: 100 });
}

function activeRuns(job, context) {
  return job.split(/^      - /m).slice(1).flatMap(step => {
    const condition = scalar(step, 'if', 8, true);
    if (condition !== undefined && !evaluate(condition, context)) return [];
    const run = scalar(step, 'run', 8, true);
    if (run === undefined) return [];
    if (run !== '|') return [run];
    const lines = step.split('\n');
    const start = lines.indexOf('        run: |');
    const end = lines.findIndex((line, index) => index > start && line.trim() &&
      !line.trimStart().startsWith('#') && !line.startsWith('          '));
    return lines.slice(start + 1, end < 0 ? lines.length : end)
      .filter(line => line.trim() && !line.trimStart().startsWith('#'))
      .map(line => line.trim());
  });
}

const verificationRuns = runs => runs.filter(line =>
  /^(?:node (?:scripts\/check\.mjs|ci\/impact\/pr-gate\.mjs)|cargo test)(?:\s|$)/.test(line));

test('Linux and Windows keep the bounded PR gate and full master groups', () => {
  const ci = read('.github/workflows/ci.yml');
  const checks = block(ci, '  checks:');
  const fullChecks = block(ci, '  full-checks:');

  const prLimit = evaluate(scalar(checks, 'timeout-minutes', 4), { eventName: 'pull_request' });
  const fullLimit = evaluate(scalar(fullChecks, 'timeout-minutes', 4), { eventName: 'push' });
  assert.ok(Number.isSafeInteger(prLimit) && prLimit > 0, 'PR job must be bounded');
  assert.ok(Number.isSafeInteger(fullLimit) && fullLimit >= prLimit,
    'the full job must not have a smaller budget than the fast PR job');

  // Do not mistake one group for full coverage: every named group must be
  // included for both hosts, with the selected PR route disabled on master.
  assert.equal(scalar(fullChecks, 'platform', 8), '[Linux, Windows]');
  const groupMatrix = scalar(fullChecks, 'group', 8);
  assert.match(groupMatrix, /^\[[a-z, -]+\]$/, 'explicit full-group matrix');
  assert.deepEqual(groupMatrix.slice(1, -1).split(',').map(group => group.trim()), MASTER_GROUPS);

  for (const eventName of ['pull_request', 'push', 'workflow_dispatch']) {
    const context = { eventName };
    const prActive = evaluate(scalar(checks, 'if', 4), context);
    const fullActive = evaluate(scalar(fullChecks, 'if', 4), context);
    assert.equal(prActive, eventName === 'pull_request', 'selected PR job condition');
    assert.equal(fullActive, eventName !== 'pull_request', 'full job condition');
    const actual = [
      ...(prActive ? verificationRuns(activeRuns(checks, context)) : []),
      ...(fullActive ? verificationRuns(activeRuns(fullChecks, context)) : []),
    ];
    const expected = [eventName === 'pull_request'
      ? 'node ci/impact/pr-gate.mjs'
      : 'node scripts/check.mjs --master-group=${{ matrix.group }}'];
    assert.deepEqual(actual, expected, `verification route for ${eventName}`);
  }
});

test('automatic macOS fallback uses the bounded profile while master and manual runs stay full', () => {
  const ci = read('.github/workflows/ci.yml');
  const fallback = block(ci, '  macos-fallback:');
  assert.equal(scalar(fallback, 'uses', 4), './.github/workflows/macos.yml');
  assert.equal(scalar(block(fallback, '    with:'), 'pr-checks', 6), 'true');
  const condition = scalar(fallback, 'if', 4);
  assert.equal(evaluate(condition, { eventName: 'pull_request' }), true);
  assert.equal(evaluate(condition, { eventName: 'pull_request', fresh: 'true' }), false);
  for (const eventName of ['push', 'workflow_dispatch']) {
    assert.equal(evaluate(condition, { eventName }), false);
  }

  const macos = read('.github/workflows/macos.yml');
  const input = block(block(block(macos, 'on:'), '  workflow_call:'), '      pr-checks:');
  assert.equal(scalar(input, 'type', 8), 'boolean');
  assert.equal(scalar(input, 'default', 8), 'false');
  const verify = block(macos, '  verify:');
  // Guard the execution policy, not a sacred timeout number. The queue-aware
  // job envelope must be positive and leave room beyond the bounded PR steps.
  const jobEnvelope = evaluate(scalar(verify, 'timeout-minutes', 4), {
    eventName: 'workflow_call', prChecks: true,
  });
  assert.ok(Number.isSafeInteger(jobEnvelope) && jobEnvelope > 0,
    'native fallback must have a positive job timeout');
  const steps = verify.split(/^      - /m).slice(1);
  const namedStep = name => {
    const matches = steps.filter(step => step.startsWith('name: ' + name + '\n'));
    assert.equal(matches.length, 1, 'one ' + name + ' step');
    return matches[0];
  };
  const build = namedStep('Full workspace build');
  const prBuildLimit = evaluate(scalar(build, 'timeout-minutes', 8), {
    eventName: 'workflow_call', prChecks: true,
  });
  const fullBuildLimit = evaluate(scalar(build, 'timeout-minutes', 8), {
    eventName: 'push', prChecks: false,
  });
  const fast = namedStep('Complete fast PR profile');
  const prTestLimit = evaluate(scalar(fast, 'timeout-minutes', 8), {
    eventName: 'workflow_call', prChecks: true,
  });
  assert.ok(Number.isSafeInteger(prBuildLimit) && prBuildLimit > 0,
    'PR build step must be bounded');
  assert.ok(Number.isSafeInteger(prTestLimit) && prTestLimit > 0,
    'PR test step must be bounded');
  assert.ok(Number.isSafeInteger(fullBuildLimit) && fullBuildLimit >= prBuildLimit,
    'full build must not have a smaller budget than PR build');
  assert.ok(jobEnvelope > prBuildLimit + prTestLimit,
    'macOS job envelope must leave time for runner queueing and setup');
  assert.equal(evaluate(scalar(fast, 'if', 8), {
    eventName: 'workflow_call', prChecks: true,
  }), true);
  for (const [eventName, prChecks, expected] of [
    ['workflow_call', true, 'node scripts/check.mjs --profile=pr'],
    ['workflow_call', false, 'node scripts/check.mjs'],
    ['push', false, 'node scripts/check.mjs'],
    ['workflow_dispatch', false, 'node scripts/check.mjs'],
  ]) {
    assert.deepEqual(verificationRuns(activeRuns(verify, { eventName, prChecks })), [expected]);
  }
});

test('the platform rehearsal has only an explicit manual trigger', () => {
  const triggers = block(read('.github/workflows/impact-platform-rehearsal.yml'), 'on:');
  const events = triggers.split('\n').filter(line => line.trim() && !line.trimStart().startsWith('#'));
  assert.deepEqual(events, ['  workflow_dispatch:']);
});

function commandFor(commands, path) {
  const selected = commands.filter(({ args }) => args.includes(path));
  assert.equal(selected.length, 1, `one canonical invocation for ${path}`);
  return selected[0];
}

function selectedNames(path) {
  const source = read(path);
  const declarations = [...source.matchAll(/^test\(/gm)];
  const names = [...source.matchAll(/^test\('([^'\\\n]*)',/gm)].map(match => match[1]);
  assert.ok(names.length > 0);
  assert.equal(names.length, declarations.length, 'test names must remain statically inspectable');
  assert.equal(new Set(names).size, names.length, 'smoke names must be unique');
  const command = commandFor(pr, path);
  const filters = command.args.filter(arg => arg.startsWith('--test-name-pattern='));
  assert.equal(command.command, 'node');
  assert.equal(filters.length, 1, 'native smoke must retain one explicit positive filter');
  assert.deepEqual(command.args, ['--test', filters[0], path],
    'native smoke cannot acquire a second file, skip flag or ambiguous filter');
  const pattern = new RegExp(filters[0].slice('--test-name-pattern='.length));
  return names.filter(name => pattern.test(name));
}

test('the note-preview PR filter selects exactly the 14 intended real test names', () => {
  assert.deepEqual(selectedNames(notePreview), [
    'trusted lifted outline displays source ranges, hierarchy and Unicode',
    'collapsed and partial outlines produce distinct deterministic HTML',
    'denial is a distinct static result with no protected backing reads',
    'inert heading labels escape markup, attributes and NUL',
    'malformed spans, duplicate starts, ancestry, section ends and quotas fail closed',
    'actual compiled JS/TS Wasm switches HTML and changed source changes HTML',
    'native A1 session scene keeps a reversible control with distinct hit identity',
    'native A1 viewport keeps zero-offset output and inert scenes compatible',
    'native A1 viewport translates pixels and hits through the full admitted outline',
    'native A1 viewport reports a native-hit-tested control until its last visible pixel',
    'native A1 viewport rejects invalid arguments and out-of-range offsets before output',
    'native A1 session scene rejects contradictory control state before output',
    'native A1 scene rejects forged denial and malformed WIT rows before painting',
    'A1 development build covers exactly the required existing guest, hosts and compiler',
  ]);
});

test('the two real HTTP smoke names remain selected and every pure client test remains unfiltered', () => {
  assert.deepEqual(selectedNames(liveRun), [
    'live HTTP host transports reversible native pixels and bounds authenticated input',
    'live synchronization fences slow old input without reloading or consuming native attempts',
  ]);
  assert.deepEqual(commandFor(pr, liveClient), { command: 'node', args: ['--test', liveClient] });
});

test('exhaustive native replays remain full-only while document smoke and license checks survive PR projection', () => {
  for (const path of [
    'scripts/check-heap-literals.mjs', 'scripts/poc.mjs',
    'tests/applications/a1-service-interaction.test.mjs',
    'tests/applications/a1-interaction-run.test.mjs',
    'tests/applications/a1-live-session.test.mjs',
  ]) {
    commandFor(full, path);
    assert.ok(!pr.some(({ args }) => args.includes(path)), `${path} must remain full-only`);
  }
  assert.deepEqual(commandFor(pr, 'scripts/check-document-outline.mjs'), {
    command: 'node', args: ['scripts/check-document-outline.mjs', '--pr'],
  });
  const poc = json('scripts/checks/025000-compiler-compatibility-replay.json');
  const license = { command: 'node', args: ['scripts/licenses.mjs'] };
  assert.deepEqual(projectPrCommands(poc), [license], 'POC exclusion must retain its license inventory');
  assert.deepEqual(commandFor(pr, 'scripts/licenses.mjs'), license);
});

test('JSON composition prepares its guest and native examples before the unchanged bounded oracle', () => {
  const path = 'scripts/check-json-composition.mjs';
  const original = commandFor(full, path);
  assert.deepEqual(original, { command: 'node', args: [path] });
  const projection = projectPrCommands(json('scripts/checks/021000-json-composition.json'));
  assert.deepEqual(projection.at(-1), original, 'all original JSON/heap/engine checks still execute');
  assert.equal(projection.length, 4, 'prepare exactly the three Cargo artifacts used by the oracle');
  const preparations = projection.slice(0, -1);
  const value = (args, flag) => args[args.indexOf(flag) + 1];
  const artifacts = preparations.map(({ command, args }) => {
    assert.equal(command, 'cargo');
    assert.equal(args[0], 'build', 'preparation must not consume the test execution budget');
    assert.ok(args.includes('--offline'));
    assert.ok(args.includes('--locked'));
    assert.equal(value(args, '-j'), '2');
    assert.equal(commandTimeoutMs(command, args, 'pr'), 600_000);
    return {
      package: value(args, '-p'),
      example: args.includes('--example') ? value(args, '--example') : null,
      target: args.includes('--target') ? value(args, '--target') : 'host',
      profile: args.includes('--release') ? 'release' : 'debug',
    };
  });
  assert.deepEqual(artifacts, [
    { package: 'grunkalunka-json-guest', example: null, target: 'wasm32-unknown-unknown', profile: 'release' },
    { package: 'grunkalunka-runtime', example: 'json_guest_check', target: 'host', profile: 'debug' },
    { package: 'grunkalunka-wasm-link', example: 'json_composition', target: 'host', profile: 'debug' },
  ]);
  const first = pr.findIndex(check => check.command === preparations[0].command &&
    JSON.stringify(check.args) === JSON.stringify(preparations[0].args));
  assert.ok(first >= 0);
  assert.deepEqual(pr.slice(first, first + projection.length), projection,
    'the live canonical PR inventory prepares all artifacts before invoking the oracle');
  assert.equal(commandTimeoutMs(original.command, original.args, 'pr'), 120_000);
});

test('edits to exact-owned A1 test names select the PR filter regression check', () => {
  const checks = ['a1-note-preview', 'a1-live-application', 'impact-planner-regressions']
    .map(name => json(`ci/impact/checks/${name}.json`));
  const owner = json('ci/impact/owners/a1-note-preview.json');
  const group = json('ci/impact/targets/a1-note-preview.json');
  // These leaves bound the graph to the exact-file override and reverse test
  // edges under inspection; native package internals do not affect this rule.
  const leaves = new Set(['canonical-scripts', ...owner.dependsOn, ...group.dependsOn]);
  leaves.delete(owner.id);
  const modules = [owner, ...[...leaves].map(id => ({ id }))];
  const known = new Set([...modules, group, ...checks].map(node => node.id));
  const otherChecks = checks.flatMap(check => check.dependsOn ?? [])
    .filter(id => !known.has(id));
  assert.ok(otherChecks.every(id => id.startsWith('check:')));
  const tests = [...checks, ...[...new Set(otherChecks)].map(id => ({ id }))];
  for (const [path, expectedOwner] of [
    [notePreview, 'check:a1-note-preview'], [liveRun, 'check:a1-live-application'],
  ]) {
    const result = planImpact({ changedPaths: [path], modules, groups: [group], tests });
    assert.equal(result.full, false, result.reasons.join('; '));
    assert.deepEqual(result.changedOwners, [expectedOwner]);
    assert.ok(result.tests.includes('check:impact-planner-regressions'),
      `${path} must execute the filter-name contract when its declarations change`);
  }
});
