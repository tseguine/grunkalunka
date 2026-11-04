import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { latestNonCancelled, evaluateReference, findReference, githubReader,
  MAX_BASELINE_AGE_MS } from '../macos-baseline.mjs';

const clock = Date.parse('2026-10-28T20:00:00Z');
const run = (id, conclusion = 'success', extra = {}) => ({
  id, run_attempt: 1, status: 'completed', conclusion,
  created_at: '2026-10-28T18:00:00Z',
  updated_at: '2026-10-28T19:00:00Z',
  event: 'schedule', head_branch: 'master', head_sha: 'a'.repeat(40),
  html_url: 'https://github.com/example/repo/actions/runs/' + id, ...extra,
});
const unstartedNativeJob = (candidate, extra = {}) => ({
  name: 'Native macOS (Apple Silicon)', status: 'completed', conclusion: 'cancelled',
  run_id: candidate.id, run_attempt: candidate.run_attempt, head_sha: candidate.head_sha,
  runner_id: 0, runner_name: '', steps: [], ...extra,
});
const singleJob = job => ({ total_count: 1, jobs: [job] });

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

test('an unstarted cancelled native job does not replace a recent master baseline', async () => {
  const older = run(41847977311, 'success', {
    created_at: '2026-11-04T14:45:00Z', updated_at: '2026-11-04T14:59:12Z',
    head_sha: '2dd461bb1d156e103abb68d60f20bf05c92b3674',
  });
  const unstarted = run(41848074200, 'failure', {
    created_at: '2026-11-04T14:59:12Z', updated_at: '2026-11-04T15:14:14Z',
    head_sha: 'fcc6a6b4a0e008b752527ffa45c0c1a402ad7d2a',
  });
  const lookups = [];
  const ref = await findReference({
    head: 'b'.repeat(40),
    runs: async (workflow, query) => {
      assert.equal(workflow, 'macos.yml');
      return query.branch ? [older, unstarted] : [];
    },
    jobs: async id => {
      lookups.push(id);
      return singleJob(unstartedNativeJob(unstarted));
    },
  });
  assert.equal(ref.run.id, older.id);
  assert.equal(ref.kind, 'master');
  assert.equal(ref.completed, older.updated_at);
  assert.equal(evaluateReference(ref, Date.parse('2026-11-04T17:00:00Z')).ok, true);
  assert.deepEqual(lookups, [unstarted.id]);
});

test('skipping an unstarted cancellation does not refresh an expired master baseline', async () => {
  const older = run(11, 'success', {
    created_at: new Date(clock - MAX_BASELINE_AGE_MS - 2000).toISOString(),
    updated_at: new Date(clock - MAX_BASELINE_AGE_MS - 1000).toISOString(),
  });
  const unstarted = run(12, 'failure');
  const ref = await findReference({
    head: 'b'.repeat(40),
    runs: async (workflow, query) => workflow === 'macos.yml' && query.branch
      ? [unstarted, older] : [],
    jobs: async id => {
      assert.equal(id, unstarted.id);
      return singleJob(unstartedNativeJob(unstarted));
    },
  });
  assert.equal(ref.run.id, older.id);
  assert.equal(ref.completed, older.updated_at);
  assert.deepEqual(evaluateReference(ref, clock), {
    ok: false, reason: 'Master macOS verification is over 24 hours old',
  });
});

test('skipping an unstarted cancellation preserves the next actual native failure', async t => {
  for (const kind of ['master', 'PR']) await t.test(kind, async () => {
    const sha = 'b'.repeat(40);
    const scope = kind === 'PR'
      ? { event: 'workflow_dispatch', head_branch: 'topic', head_sha: sha } : {};
    const older = run(13, 'success', scope);
    const failed = run(14, 'failure', {
      ...scope, created_at: '2026-10-28T18:30:00Z',
    });
    const unstarted = run(15, 'failure', {
      ...scope, created_at: '2026-10-28T19:00:00Z',
    });
    const lookups = [];
    const ref = await findReference({
      head: sha,
      runs: async (workflow, query) => {
        assert.equal(workflow, 'macos.yml');
        if (kind === 'PR') {
          assert.ok(query.head_sha, 'failed exact-head result must precede master');
          return [older, unstarted, failed];
        }
        return query.branch ? [older, unstarted, failed] : [];
      },
      jobs: async id => {
        lookups.push(id);
        return singleJob(id === unstarted.id ? unstartedNativeJob(unstarted) :
          unstartedNativeJob(failed, { conclusion: 'failure', runner_id: 42,
            runner_name: 'hosted-native', steps: [{ conclusion: 'failure' }] }));
      },
    });
    assert.equal(ref.run.id, failed.id);
    assert.equal(ref.kind, kind);
    assert.equal(ref.outcome, 'failure');
    assert.equal(evaluateReference(ref, clock).ok, false);
    assert.deepEqual(lookups, [unstarted.id, failed.id]);
  });
});

test('an unstarted exact-head cancellation permits the master baseline', async () => {
  const sha = 'c'.repeat(40);
  const exact = run(16, 'failure', {
    event: 'workflow_dispatch', head_branch: 'topic', head_sha: sha,
  });
  const master = run(17);
  const ref = await findReference({
    head: sha,
    runs: async (workflow, query) => {
      assert.equal(workflow, 'macos.yml');
      return query.head_sha ? [exact] : [master];
    },
    jobs: async id => {
      assert.equal(id, exact.id);
      return singleJob(unstartedNativeJob(exact));
    },
  });
  assert.equal(ref.run.id, master.id);
  assert.equal(ref.kind, 'master');
  assert.equal(evaluateReference(ref, clock).ok, true);
});

test('a failed explicit exact-PR run takes precedence over a successful master baseline', async () => {
  const sha = 'c'.repeat(40);
  const exact = run(4, 'failure', {
    event: 'workflow_dispatch', head_branch: 'topic', head_sha: sha,
  });
  const reader = {
    runs: async (workflow, query) => workflow === 'macos.yml'
      ? query.head_sha ? [exact] : [run(5)] : [],
    jobs: async id => {
      assert.equal(id, exact.id);
      return singleJob(unstartedNativeJob(exact, { conclusion: 'failure',
        runner_id: 42, runner_name: 'hosted-native', steps: [{ conclusion: 'failure' }] }));
    },
  };
  const ref = await findReference({ ...reader, head: sha });
  assert.equal(ref.kind, 'PR');
  assert.equal(ref.outcome, 'failure');
  assert.equal(evaluateReference(ref, clock).ok, false);
});

test('uncertain or executed native jobs keep a failed wrapper unhealthy', async t => {
  const failed = run(18, 'failure', { created_at: '2026-10-28T19:00:00Z' });
  const job = unstartedNativeJob(failed);
  const cases = [
    ['missing response', undefined],
    ['missing job count', { jobs: [job] }],
    ['non-numeric job count', { total_count: '1', jobs: [job] }],
    ['partial job list', { total_count: 2, jobs: [job] }],
    ['missing job list', { total_count: 1 }],
    ['non-array job list', { total_count: 1, jobs: { 0: job, length: 1 } }],
    ['empty job list', { total_count: 1, jobs: [] }],
    ['extra job', { total_count: 2, jobs: [job, { name: 'Another job' }] }],
    ['extra job despite count', { total_count: 1, jobs: [job, { name: 'Another job' }] }],
    ['missing job', singleJob(null)],
    ...[
      ['different job', { name: 'Checks (macOS)' }],
      ['missing completion status', { status: undefined }],
      ['unfinished job', { status: 'in_progress' }],
      ['missing conclusion', { conclusion: undefined }],
      ['failed job', { conclusion: 'failure' }],
      ['timed out job', { conclusion: 'timed_out' }],
      ['missing run identity', { run_id: undefined }],
      ['different run', { run_id: failed.id + 1 }],
      ['missing attempt identity', { run_attempt: undefined }],
      ['different attempt', { run_attempt: failed.run_attempt + 1 }],
      ['missing head identity', { head_sha: undefined }],
      ['different head', { head_sha: 'f'.repeat(40) }],
      ['missing runner ID', { runner_id: undefined }],
      ['assigned runner ID', { runner_id: 42 }],
      ['missing runner name', { runner_name: undefined }],
      ['assigned runner name', { runner_name: 'hosted-native' }],
      ['missing steps', { steps: undefined }],
      ['non-array steps', { steps: { length: 0 } }],
      ['started steps', { steps: [{ name: 'Set up job', conclusion: 'success' }] }],
    ].map(([name, extra]) => [name, singleJob({ ...job, ...extra })]),
  ];
  for (const [name, response] of cases) await t.test(name, async () => {
    const ref = await findReference({
      head: 'b'.repeat(40),
      runs: async (workflow, query) => workflow === 'macos.yml' && query.branch
        ? [run(19), failed] : [],
      jobs: async id => {
        assert.equal(id, failed.id);
        return response;
      },
    });
    assert.equal(ref.run.id, failed.id);
    assert.equal(ref.outcome, 'failure');
    assert.equal(evaluateReference(ref, clock).ok, false);
  });
});

test('missing identity on both the workflow and job is not evidence of cancellation', async t => {
  for (const field of ['id', 'run_attempt', 'head_sha']) await t.test(field, async () => {
    const failed = run(24, 'failure', { created_at: '2026-10-28T19:00:00Z' });
    delete failed[field];
    const ref = await findReference({
      head: 'b'.repeat(40),
      runs: async (workflow, query) => workflow === 'macos.yml' && query.branch
        ? [run(25), failed] : [],
      jobs: async () => singleJob(unstartedNativeJob(failed)),
    });
    assert.equal(ref.run, failed);
    assert.equal(evaluateReference(ref, clock).ok, false);
  });
});

test('timed out workflows remain unhealthy without inspecting an unstarted job', async () => {
  const timedOut = run(20, 'timed_out', { created_at: '2026-10-28T19:00:00Z' });
  const ref = await findReference({
    head: 'b'.repeat(40),
    runs: async (workflow, query) => workflow === 'macos.yml' && query.branch
      ? [run(21), timedOut] : [],
    jobs: async () => { throw new Error('only failed wrappers need inspection'); },
  });
  assert.equal(ref.run.id, timedOut.id);
  assert.equal(evaluateReference(ref, clock).ok, false);
});

test('a failed wrapper job lookup error propagates instead of selecting older success', async () => {
  const failure = new Error('GitHub jobs lookup unavailable');
  await assert.rejects(findReference({
    head: 'b'.repeat(40),
    runs: async (workflow, query) => workflow === 'macos.yml' && query.branch
      ? [run(22), run(23, 'failure', { created_at: '2026-10-28T19:00:00Z' })] : [],
    jobs: async id => {
      assert.equal(id, 23);
      throw failure;
    },
  }), error => error === failure);
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
  // Git checkouts may use CRLF on Windows; inspect the same YAML on every host.
  const normalized = workflow.replace(/\r\n/g, '\n');
  const triggers = normalized.split('\non:\n')[1]?.split('\npermissions:')[0];
  assert.ok(triggers, 'macOS workflow must declare triggers');
  assert.match(triggers, /^  push:\n    branches: \[master\]$/m);
  assert.match(triggers, /^  workflow_dispatch:$/m);
  assert.match(triggers, /^  workflow_call:$/m);
  assert.doesNotMatch(triggers, /^  schedule:$/m);
  assert.doesNotMatch(triggers, /^    paths:$/m);
  assert.match(normalized, /^  cancel-in-progress: false$/m);
});
