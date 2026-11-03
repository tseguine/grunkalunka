import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test, { before } from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const executable = join(target, 'debug', 'examples',
  'a1_ui_action' + (process.platform === 'win32' ? '.exe' : ''));
const MAX_COMPONENT = 4 * 1024 * 1024;

function checked(result, label) {
  if (result.error) throw result.error;
  assert.equal(result.signal, null, label + ' must not terminate by signal');
  assert.equal(result.status, 0, result.stderr?.toString('utf8') || label);
  return result.stdout;
}

before(() => {
  checked(spawnSync('cargo', [
    'build', '-p', 'grunkalunka-runtime', '--example', 'a1_ui_action',
    '--locked', '--offline',
  ], { cwd: root, shell: false, timeout: 180_000,
    killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024 }), 'build A1 action host');
}, { timeout: 180_000 });

function invoke(args, input = Buffer.alloc(0)) {
  return spawnSync(executable, args, {
    cwd: root, input, shell: false, timeout: 30_000,
    killSignal: 'SIGKILL', maxBuffer: MAX_COMPONENT + 1024,
  });
}

function source(body = 'return choose(a, b);', language = 'js') {
  const parameters = language === 'ts' ? 'a: number, b: number' : 'a, b';
  return 'import { selectVisible as choose } from "grunkalunka:a1/ui";\n' +
    'export function run(' + parameters + ') { ' + body + ' }\n';
}

function compile(body, language = 'js') {
  const bytes = checked(invoke(['--compile', language],
    Buffer.from(source(body, language))), 'compile ' + language);
  assert.deepEqual(bytes.subarray(0, 4), Buffer.from([0, 97, 115, 109]));
  assert.ok(bytes.length > 8 && bytes.length <= MAX_COMPONENT);
  return bytes;
}

function action(component, profile = 'granted', count = 2, expanded = 1) {
  return JSON.parse(checked(invoke(
    ['--run', profile, String(count), String(expanded)], component), 'run A1 action'));
}

function native(profile = 'granted', count = 2, expanded = 1) {
  return JSON.parse(checked(invoke(
    ['--native', profile, String(count), String(expanded)]), 'native A1 action'));
}

function failed(args, input) {
  const result = invoke(args, input);
  if (result.error) throw result.error;
  assert.equal(result.signal, null);
  assert.notEqual(result.status, 0);
  assert.equal(result.stdout.length, 0, 'failure must not publish partial state/artifact');
  assert.match(result.stderr.toString('utf8'), /A1 UI action:/);
  return result.stderr.toString('utf8');
}

test('real source-compiled JS/TS components match the native Rust action contract',
  { timeout: 180_000 }, () => {
    const component = compile();
    assert.deepEqual(compile(undefined, 'ts'), component,
      'TypeScript annotations must preserve the actual component bytes');
    for (const count of [0, 2, 64]) {
      for (const expanded of [0, 1]) {
        const receipt = action(component, 'granted', count, expanded);
        assert.deepEqual(receipt, native('granted', count, expanded));
        assert.deepEqual(receipt, {
          schemaVersion: 1, profile: 'granted', permittedCount: count,
          attempts: 1, transitions: 1, visible: expanded ? count : 0,
          result: expanded ? count : 0, status: 'ok',
        });
      }
    }
    for (const expanded of [0, 1]) {
      assert.deepEqual(action(component, 'denied', 2, expanded),
        native('denied', 2, expanded));
    }
    assert.deepEqual(action(component, 'denied'), {
      schemaVersion: 1, profile: 'denied', permittedCount: 2,
      attempts: 1, transitions: 0, visible: 0, result: null, status: 'denied',
    });
  });

test('changing actual source changes authoritative state independently of return values',
  { timeout: 120_000 }, () => {
    const collapse = compile('return choose(0, b) + 17;');
    const expand = compile('return choose(1, b) + 17;');
    assert.notDeepEqual(collapse, expand);
    const hidden = action(collapse);
    const shown = action(expand);
    assert.deepEqual([hidden.visible, hidden.result, hidden.transitions], [0, 17, 1]);
    assert.deepEqual([shown.visible, shown.result, shown.transitions], [2, 19, 1]);

    const twice = action(compile('choose(1, b); return choose(0, b) + 17;'));
    assert.deepEqual([twice.attempts, twice.transitions, twice.visible, twice.result],
      [2, 2, 0, 17]);
    assert.deepEqual(action(collapse, 'denied'), {
      schemaVersion: 1, profile: 'denied', permittedCount: 2,
      attempts: 1, transitions: 0, visible: 0, result: null, status: 'denied',
    });
  });

test('wrong counts and expanded values reject before committing staged mutations',
  { timeout: 120_000 }, () => {
    for (const [body, status, attempts] of [
      ['return choose(a, b + 1);', 'invalid-count', 1],
      ['choose(1, b); return choose(0, b + 1);', 'invalid-count', 2],
      ['choose(1, b); return choose(2, b);', 'invalid-expanded', 2],
      ['choose(1, b); return choose(0 / 0, b);', 'invalid-expanded', 2],
      ['choose(1, b); return choose(0, b / 0);', 'invalid-count', 2],
    ]) {
      const receipt = action(compile(body));
      assert.deepEqual(receipt, {
        schemaVersion: 1, profile: 'granted', permittedCount: 2,
        attempts, transitions: 0, visible: 0, result: null, status,
      }, body);
    }
  });

test('non-finite source results and service-call exhaustion discard the whole action',
  { timeout: 120_000 }, () => {
    for (const expression of ['0 / 0', '1 / 0', '-1 / 0']) {
      const receipt = action(compile('choose(1, b); return ' + expression + ';'));
      assert.deepEqual([receipt.status, receipt.attempts, receipt.transitions,
        receipt.visible, receipt.result], ['invalid-result', 1, 0, 0, null]);
    }
    const receipt = action(compile(
      'for (let i = 0; i < 257; i++) { choose(a, b); } return 0;'));
    assert.deepEqual([receipt.status, receipt.attempts, receipt.transitions,
      receipt.visible, receipt.result], ['call-limit', 257, 0, 0, null]);

    // No component state is retained across separate action invocations.
    const next = action(compile());
    assert.deepEqual([next.attempts, next.transitions, next.visible], [1, 1, 2]);
  });

test('malformed commands, source and component bytes publish no partial result',
  { timeout: 120_000 }, () => {
    for (const args of [
      [], ['--compile', 'unknown'], ['--native', 'other', '2', '1'],
      ['--native', 'granted', '65', '1'], ['--native', 'granted', '02', '1'],
      ['--native', 'granted', '-1', '1'], ['--native', 'granted', '2.5', '1'],
      ['--native', 'granted', '2', 'NaN'], ['--native', 'granted', '2', '2'],
      ['--run', 'granted', '2', '1', 'extra'],
    ]) failed(args);
    failed(['--compile', 'js'], Buffer.from('export function run( {'));
    failed(['--compile', 'js'], Buffer.from([0xff]));
    assert.match(failed(['--compile', 'js'], Buffer.alloc(64 * 1024 + 1, 0x20)),
      /exceeds/);
    failed(['--compile', 'js'], Buffer.from(source().replace('grunkalunka:a1/ui', 'node:fs')));
    failed(['--run', 'granted', '2', '1'], Buffer.from([0, 97, 115, 109]));
    assert.match(failed(['--run', 'granted', '2', '1'],
      Buffer.alloc(MAX_COMPONENT + 1)), /exceeds/);
  });

test('fuel exhaustion after a service call publishes no committed state',
  { timeout: 120_000 }, () => {
    const component = compile('choose(1, b); while (a > 0) { b += 1; } return b;');
    assert.match(failed(['--run', 'granted', '2', '1'], component), /fuel/i);
  });
