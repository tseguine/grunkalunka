import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { latestNonCancelled, evaluateReference, findReference, githubReader,
  MAX_BASELINE_AGE_MS } from '../macos-baseline.mjs';

const clock = Date.parse('2026-10-28T20:00:00Z');
const run = (id, conclusion = 'success', extra = {}) => ({
  id, status: 'completed', conclusion,
  created_at: '2026-10-28T18:00:00Z',
  updated_at: '2026-10-28T19:00:00Z',
  event: 'schedule', head_branch: 'master', head_sha: 'a'.repeat(40),
  html_url: 'https://github.com/example/repo/actions/runs/' + id, ...extra,
});

test('the newest completed failure is not masked by an older success', () => {
  const older = run(1);
  const failed = run(2, 'failure', { created_at: '2026-10-28T19:00:00Z' });
  const cancelled = run(3, 'cancelled', { created_at: '2026-10-28T19:30:00Z' });
  assert.equal(latestNonCancelled([older, cancelled, failed]).id, 2);
  assert.equal(evaluateReference({ kind: 'master', outcome: 'failure',
    completed: failed.updated_at }, clock).ok, false);
});

test('a green master reference is bounded by completion age, not PR status', () => {
  const recent = { kind: 'master', outcome: 'success',
    completed: new Date(clock - MAX_BASELINE_AGE_MS + 1000).toISOString() };
  assert.equal(evaluateReference(recent, clock).ok, true);
  const stale = { ...recent,
    completed: new Date(clock - MAX_BASELINE_AGE_MS - 1000).toISOString() };
  assert.equal(evaluateReference(stale, clock).ok, false);
  assert.equal(evaluateReference({ ...stale, kind: 'PR' }, clock).ok, true);
  assert.equal(evaluateReference(null, clock).ok, false);
});

test('a failed explicit exact-PR run takes precedence over a successful master baseline', async () => {
  const sha = 'c'.repeat(40);
  const reader = {
    runs: async (workflow, query) => workflow === 'macos.yml'
      ? query.head_sha ? [run(4, 'failure', {
          event: 'workflow_dispatch', head_branch: 'topic', head_sha: sha,
        })] : [run(5)] : [],
    jobs: async () => { throw new Error('legacy should not run'); },
  };
  const ref = await findReference({ ...reader, head: sha });
  assert.equal(ref.kind, 'PR');
  assert.equal(ref.outcome, 'failure');
  assert.equal(evaluateReference(ref, clock).ok, false);
});

test('new scheduled master runs take precedence over legacy CI', async () => {
  const reader = {
    runs: async (workflow, query) => workflow === 'macos.yml'
      ? query.branch ? [run(8)] : [] : [],
    jobs: async () => { throw new Error('no legacy lookup'); },
  };
  const ref = await findReference({ ...reader, head: 'd'.repeat(40) });
  assert.equal(ref.run.id, 8);
  assert.equal(ref.kind, 'master');
});

test('legacy bootstrap checks the actual macOS job even when aggregate CI was cancelled', async () => {
  const reader = {
    runs: async (workflow) => workflow === 'ci.yml' ? [
      run(9, 'cancelled', { created_at: '2026-10-28T18:30:00Z' }),
      run(10, 'success', { created_at: '2026-10-28T18:00:00Z' }),
    ] : [],
    jobs: async id => ({ jobs: id === 9 ? [
      { name: 'Checks (macOS)', status: 'completed', conclusion: 'success',
        completed_at: '2026-10-28T18:40:00Z' },
    ] : [] }),
  };
  const ref = await findReference({ ...reader, head: 'e'.repeat(40) });
  assert.equal(ref.run.id, 9);
  assert.equal(ref.outcome, 'success');
  assert.equal(ref.kind, 'master');
});

test('new workflow 404 is allowed during bootstrap; other API errors propagate', async () => {
  const fetchImpl = async url => {
    if (url.includes('macos.yml')) return { ok: false, status: 404 };
    return { ok: true, json: async () => ({ workflow_runs: [] }) };
  };
  const reader = githubReader({ repository: 'example/repo', token: 'test',
    fetchImpl });
  assert.deepEqual(await reader.runs('macos.yml', { branch: 'master' }), []);
  assert.deepEqual(await reader.runs('ci.yml', { branch: 'master' }), []);
});

test('native macOS master push trigger is unfiltered and active runs are preserved', () => {
  const workflow = readFileSync(new URL('../../.github/workflows/macos.yml', import.meta.url), 'utf8');
  const triggers = workflow.split('\non:\n')[1]?.split('\npermissions:')[0];
  assert.ok(triggers, 'macOS workflow must declare triggers');
  assert.match(triggers, /^  push:\n    branches: \[master\]$/m);
  assert.match(triggers, /^  workflow_dispatch:$/m);
  assert.match(triggers, /^  workflow_call:$/m);
  assert.doesNotMatch(triggers, /^  schedule:$/m);
  assert.doesNotMatch(triggers, /^    paths:$/m);
  assert.match(workflow, /^  cancel-in-progress: false$/m);
});
