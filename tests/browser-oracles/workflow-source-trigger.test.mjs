// The offline oracle suite depends on the tool-side helpers it imports.
// This source contract guards both GitHub event path filters from regression.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const path = new URL('../../.github/workflows/browser-oracles.yml', import.meta.url);
const source = readFileSync(path, 'utf8');

function verifyWorkflowTriggers(workflow) {
  const pr = workflow.split('  pull_request:\n');
  assert.equal(pr.length, 2, 'one PR trigger');
  const push = pr[1].split('  push:\n');
  assert.equal(push.length, 2, 'one push trigger');
  const dispatch = push[1].split('  workflow_dispatch:\n');
  assert.equal(dispatch.length, 2, 'manual dispatch preserved');

  for (const [name, event] of [['pull_request', push[0]], ['push', dispatch[0]]]) {
    assert.match(event, /^    branches: \[master\]$/m, name + ' master target');
    assert.match(event, /^    paths:$/m, name + ' path filter');
    for (const pattern of ['tests/browser-oracles/**', 'tools/browser-oracles/**',
      '.github/workflows/browser-oracles.yml']) {
      const entry = "      - '" + pattern + "'";
      assert.equal(event.split(entry).length, 2, name + ' must include ' + pattern + ' exactly once');
    }
  }
}

test('both PR and push rerun offline oracles when imported browser tools change', () => {
  verifyWorkflowTriggers(source);
});

test('losing the helper path in either event fails the source contract', () => {
  const pattern = "      - 'tools/browser-oracles/**'\n";
  assert.equal(source.split(pattern).length, 3, 'one tool trigger per event');
  const first = source.replace(pattern, '');
  assert.throws(() => verifyWorkflowTriggers(first), /must include tools\/browser-oracles/);
  const last = source.slice(0, source.lastIndexOf(pattern)) + source.slice(source.lastIndexOf(pattern) + pattern.length);
  assert.throws(() => verifyWorkflowTriggers(last), /must include tools\/browser-oracles/);
});
