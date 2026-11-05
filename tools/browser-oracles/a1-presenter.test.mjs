import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { offsetPresenterPointer, readPresenterAssets, startA1PresenterFixture } from './a1-presenter-fixture.mjs';
import { runA1Presenter } from './a1-presenter.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');

async function fixture(t) {
  const server = await startA1PresenterFixture();
  t.after(() => server.close());
  const token = new URL(server.url).hash.slice(1);
  async function post(path, body, generation) {
    return fetch(server.origin + path, { method: 'POST',
      headers: { 'Content-Type': 'application/json', 'x-a1-session': token,
        ...(generation === undefined ? {} : { 'x-a1-input-generation': String(generation) }) },
      body: JSON.stringify(body) });
  }
  return { server, post };
}

test('presenter fixture serves production assets and hashes the actual alternate source', async t => {
  const assets = await readPresenterAssets();
  const { server } = await fixture(t);
  assert.equal(await (await fetch(server.origin + '/')).text(), assets.page);
  assert.equal(await (await fetch(server.origin + '/style.css')).text(), assets.styles);
  assert.equal(await (await fetch(server.origin + '/client.mjs')).text(), assets.client);
  const changed = offsetPresenterPointer(assets.client);
  assert.notEqual(hash(changed), hash(assets.client));
  const alternate = await startA1PresenterFixture({ clientSource: changed });
  t.after(() => alternate.close());
  assert.equal(hash(await (await fetch(alternate.origin + '/client.mjs')).text()),
    alternate.provenance.served.client);
  assert.notEqual(alternate.provenance.served.client, server.provenance.served.client);
  assert.deepEqual(alternate.provenance.sources, server.provenance.sources);
  assert.throws(() => offsetPresenterPointer('unrelated source'), /Pointer control source changed/);
  assert.throws(() => offsetPresenterPointer(assets.client + assets.client), /Pointer control source changed/);
});

test('fixture sync does not spend input budget and stale generations cannot commit', async t => {
  const { server, post } = await fixture(t);
  const initial = await (await post('/sync', {})).json();
  assert.deepEqual([initial.sequence, initial.inputGeneration, initial.remainingEvents], [0, 1, 64]);
  const input = await (await post('/input', { x: 24, y: 16 }, 1)).json();
  assert.deepEqual([input.sequence, input.expanded], [1, 0]);
  const sync = await (await post('/sync', {})).json();
  assert.deepEqual([sync.sequence, sync.inputGeneration, sync.remainingEvents], [1, 2, 63]);
  const stale = await post('/input', { x: 24, y: 16 }, 1);
  assert.equal(stale.status, 409);
  const failure = await stale.json();
  assert.equal(failure.requiresSync, true);
  assert.deepEqual([failure.frame.sequence, failure.frame.expanded], [1, 0]);
  assert.equal(server.state().sequence, 1);
});

test('a deliberately lost reply commits exactly once and sync exposes that frame', async t => {
  const { server, post } = await fixture(t);
  await (await post('/sync', {})).arrayBuffer();
  server.loseNextInputReply();
  await assert.rejects(async () => (await post('/input', { x: 24, y: 16 }, 1)).json());
  const synced = await (await post('/sync', {})).json();
  assert.deepEqual([synced.sequence, synced.expanded, synced.inputGeneration], [1, 0, 2]);
  assert.equal(server.observations().filter(entry => entry.path === '/input').length, 1);
  assert.equal(server.observations().find(entry => entry.path === '/input').replyLost, true);
});

test('fixture selection failure retains the prior note and rejects malformed input', async t => {
  const { server, post } = await fixture(t);
  const initial = await (await post('/sync', {})).json();
  const release = server.holdNextSelectionFailure();
  const pending = post('/select', { id: 1 }, 1);
  release();
  const response = await pending;
  assert.equal(response.status, 422);
  const failure = await response.json();
  assert.deepEqual([failure.frame.selectedNoteId, failure.frame.sequence], [0, 1]);
  assert.equal(failure.frame.pixels, initial.pixels);
  const invalid = await post('/select', { id: 1, path: 'arbitrary.md' }, 1);
  assert.equal(invalid.status, 400);
  await invalid.arrayBuffer();
  assert.equal(server.state().sequence, 1);
  const missing = await fetch(server.origin + '/private.md');
  assert.equal(missing.status, 404);
  await missing.arrayBuffer();
});

test('real presenter runner requires an explicit independent engine and output', async () => {
  await assert.rejects(runA1Presenter('chromium'), /Explicit browser engine/);
  await assert.rejects(runA1Presenter('invented', { browserTypes: {}, playwrightVersion: '1',
    outputDir: 'unused' }), /Explicit browser engine/);
});
