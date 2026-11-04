// Lightweight PR check for the latest completed native macOS result.
// A successful master baseline is NOT evidence that this PR ran on macOS.
import { appendFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

export const MAX_BASELINE_AGE_MS = 24 * 60 * 60 * 1000;
const isNonCancelled = r => r && r.status === 'completed' &&
  !!r.conclusion && r.conclusion !== 'cancelled';

function orderedNonCancelled(runs, filter = () => true) {
  return [...runs].filter(r => isNonCancelled(r) && filter(r))
    .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at) ||
      (b.run_attempt ?? 0) - (a.run_attempt ?? 0));
}

export function latestNonCancelled(runs, filter = () => true) {
  return orderedNonCancelled(runs, filter)[0] ?? null;
}

function wasCancelledBeforeStarting(run, result) {
  if (result?.total_count !== 1 || !Array.isArray(result.jobs) ||
      result.jobs.length !== 1) return false;
  const job = result.jobs[0];
  return job?.name === 'Native macOS (Apple Silicon)' &&
    job.status === 'completed' && job.conclusion === 'cancelled' &&
    job.runner_id === 0 && job.runner_name === '' &&
    Array.isArray(job.steps) && job.steps.length === 0 &&
    Number.isSafeInteger(run.id) && run.id > 0 && job.run_id === run.id &&
    Number.isSafeInteger(run.run_attempt) && run.run_attempt > 0 &&
    job.run_attempt === run.run_attempt &&
    typeof run.head_sha === 'string' && run.head_sha.length > 0 &&
    job.head_sha === run.head_sha;
}

async function latestNativeResult(runs, jobs, filter) {
  for (const run of orderedNonCancelled(runs, filter)) {
    // GitHub can fail the workflow when its only native job is cancelled before
    // acquiring a runner. Skip only complete evidence for this same run attempt;
    // actual failures and uncertain job metadata must still require fallback.
    if (run.conclusion === 'failure' &&
        wasCancelledBeforeStarting(run, await jobs(run.id))) continue;
    return run;
  }
  return null;
}

export function evaluateReference(ref, now = Date.now()) {
  if (!ref) return { ok: false, reason: 'No completed, non-cancelled macOS job found' };
  const completed = Date.parse(ref.completed);
  if (!Number.isFinite(completed) || completed > now) {
    return { ok: false, reason: 'Invalid macOS completion timestamp' };
  }
  if (ref.outcome !== 'success') {
    return { ok: false, reason: 'Latest native macOS result is ' + ref.outcome };
  }
  if (ref.kind !== 'PR' && now - completed > MAX_BASELINE_AGE_MS) {
    return { ok: false, reason: 'Master macOS verification is over 24 hours old' };
  }
  return { ok: true, reason: ref.kind === 'PR' ?
    'Exact PR head passed on native macOS' : 'Recent master macOS baseline passed' };
}

export async function findReference({ runs, jobs, head }) {
  const exact = await latestNativeResult(await runs('macos.yml',
    { head_sha: head, status: 'completed' }), jobs, r => r.event === 'workflow_dispatch' &&
      r.head_branch !== 'master' && r.head_sha === head);
  if (exact) return {
    kind: 'PR', run: exact, outcome: exact.conclusion,
    completed: exact.updated_at, url: exact.html_url,
  };

  const master = await latestNativeResult(await runs('macos.yml',
    { branch: 'master', status: 'completed' }), jobs, r => r.head_branch === 'master');
  if (master) return {
    kind: 'master', run: master, outcome: master.conclusion,
    completed: master.updated_at, url: master.html_url,
  };

  // Bootstrap when the scheduled workflow has not run yet. Inspect the actual
  // macOS job from legacy master CI, not its aggregate workflow conclusion.
  const old = await runs('ci.yml', { branch: 'master', status: 'completed' });
  for (const run of [...old].sort((a, b) =>
    Date.parse(b.created_at) - Date.parse(a.created_at))) {
    if (run.head_branch !== 'master') continue;
    const entries = (await jobs(run.id)).jobs ?? [];
    const mac = entries.find(j => j.name === 'Checks (macOS)' &&
      j.status === 'completed' && j.conclusion && j.conclusion !== 'cancelled');
    if (mac) return {
      kind: 'master', run, outcome: mac.conclusion,
      completed: mac.completed_at, url: mac.html_url ?? run.html_url,
    };
  }
  return null;
}

export function githubReader({ repository, token, apiUrl = 'https://api.github.com',
  fetchImpl = fetch }) {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository ?? '')) {
    throw new Error('Invalid GITHUB_REPOSITORY');
  }
  const origin = apiUrl.replace(/\/$/, '');
  const request = async path => {
    const result = await fetchImpl(origin + '/repos/' + repository + path, {
      headers: {
        Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28',
        ...(token ? { Authorization: 'Bearer ' + token } : {}),
      },
    });
    if (!result.ok) {
      const error = new Error('GitHub API HTTP ' + result.status + ' at ' + path);
      error.status = result.status;
      throw error;
    }
    return result.json();
  };
  const runs = async (workflow, query) => {
    const all = [];
    for (let page = 1; page <= 5; page++) {
      const params = new URLSearchParams({ ...query, per_page: '100',
        page: String(page) });
      let result;
      try {
        result = await request('/actions/workflows/' + workflow + '/runs?' + params);
      } catch (error) {
        if (error.status === 404 && workflow === 'macos.yml') return [];
        throw error;
      }
      const batch = result.workflow_runs ?? [];
      all.push(...batch);
      if (batch.length < 100) break;
    }
    return all;
  };
  const jobs = id => request('/actions/runs/' + id + '/jobs?per_page=100');
  return { runs, jobs };
}

async function main() {
  if (!process.env.GITHUB_REPOSITORY || !process.env.GITHUB_TOKEN ||
      !process.env.PR_HEAD_SHA) throw new Error('Missing GitHub Actions context');
  const reader = githubReader({
    repository: process.env.GITHUB_REPOSITORY,
    token: process.env.GITHUB_TOKEN,
    apiUrl: process.env.GITHUB_API_URL,
  });
  const ref = await findReference({ ...reader, head: process.env.PR_HEAD_SHA });
  const result = evaluateReference(ref);
  const age = ref ? (Date.now() - Date.parse(ref.completed)) / 3600000 : null;
  const summary = ref ? [
    'Result: ' + ref.outcome,
    'Source: ' + ref.kind,
    'Revision: ' + ref.run.head_sha,
    'Completed: ' + ref.completed,
    'Age: ' + age.toFixed(1) + ' hours',
    'Run: ' + ref.url,
  ].join('\n') : 'No non-cancelled macOS run was found';
  console.log(result.reason + '\n' + summary);
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY,
      '### macOS baseline probe: ' + (result.ok ? 'FRESH' : 'FALLBACK REQUIRED') + '\n\n' +
      (ref?.kind === 'PR' ? 'This exact PR head ran on native macOS.\n\n' :
        '**This PR was NOT run on macOS.** This check uses a master baseline.\n\n') +
      result.reason + '\n\n' + summary.replaceAll('\n', '\n\n') + '\n');
  }
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, 'fresh=' + (result.ok ? 'true' : 'false') + '\n');
  }
  // An unhealthy reference is not a failed PR: request the native fallback.
  if (!result.ok) console.warn('Native macOS fallback required: ' + result.reason);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => {
    console.error('Baseline lookup unavailable; native macOS fallback required:', error);
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, 'fresh=false\n');
    // A lookup failure must never produce a green baseline; fallback runs instead.
  });
}
