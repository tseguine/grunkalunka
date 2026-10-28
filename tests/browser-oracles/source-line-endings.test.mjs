// Source-hashed browser fixtures and workflow contracts require stable EOL
// bytes. Git attributes enforce this on Windows, not test-time rewriting.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = path => readFileSync(new URL('../../' + path, import.meta.url));
test('browser fixture and workflow source bytes have canonical LF on every host', () => {
  const attr = read('.gitattributes').toString('utf8');
  for (const pattern of [
    'tests/browser-oracles/** text eol=lf',
    'tools/browser-oracles/** text eol=lf',
    '.github/workflows/browser-*.yml text eol=lf',
  ]) assert.ok(attr.includes(pattern), 'missing checked-out LF policy for ' + pattern);
  for (const path of [
    'tests/browser-oracles/native-block-bridge.html',
    'tests/browser-oracles/flex-wrap-lines.html',
    'tests/browser-oracles/event-dispatch.html',
    'tests/browser-oracles/grid-placement.html',
    'tests/browser-oracles/native-block-bridge-workflow.test.mjs',
    'tools/browser-oracles/native-hit-sensitivity.test.mjs',
    '.github/workflows/browser-native-bridge.yml',
    '.github/workflows/browser-smoke.yml',
  ]) {
    const bytes = read(path);
    assert.ok(!bytes.includes(13), path + ' contains a carriage return after checkout');
    assert.ok(bytes.includes(10), path + ' must contain LF');
  }
});
