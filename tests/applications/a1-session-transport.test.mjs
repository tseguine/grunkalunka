// Protocol-level tests of the real parent adapter with a controlled worker.
// These exercise queue/lifecycle semantics and the real loopback HTTP host,
// not native/WIT execution. Native integration has separate application tests.
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import http from 'node:http';
import { syncBuiltinESMExports } from 'node:module';
import workerThreads from 'node:worker_threads';
import test, { afterEach, beforeEach } from 'node:test';
import { createAsyncNoteSession } from '../../examples/note-preview/async-session.mjs';
import { startLiveNote } from '../../examples/note-preview/live-run.mjs';

function frame(sequence, scrollY = 0) {
  const header = Buffer.from('P6\n320 160\n255\n');
  const pixels = Buffer.alloc(header.length + 320 * 160 * 3, sequence);
  header.copy(pixels);
  pixels[header.length + 1] = scrollY;
  return { expanded: 1, visibleCount: 1, eventsProcessed: sequence,
    scene: { html: 'frame ' + sequence, visibleCount: 1, input: Buffer.from([sequence]) },
    pixels, scrollY, maxScrollY: 100,
    controlPoint: { x: 1, y: 2 }, noteChoices: [{ id: 0, label: 'Note' }], selectedNoteId: 0 };
}

class ControlledWorker extends EventEmitter {
  static instances = [];
  messages = [];
  sent = [];
  terminated = false;

  constructor() {
    super();
    ControlledWorker.instances.push(this);
    queueMicrotask(() => this.emit('message', { type: 'ready', snapshot: frame(0) }));
  }

  postMessage(message) {
    this.messages.push(message);
    this.sent.push(message);
  }
  terminate() { this.terminated = true; return Promise.resolve(0); }

  reply(snapshot, error) {
    const request = this.messages.shift();
    assert.ok(request, 'only dispatched worker inputs can reply');
    const result = { sequence: snapshot.eventsProcessed, status: 'ok', changed: true,
      beforeScrollY: 0, scrollY: snapshot.scrollY, pixels: snapshot.pixels };
    this.emit('message', { type: 'result', id: request.id,
      ok: error === undefined, result, error, snapshot });
  }
}

const originalWorker = workerThreads.Worker;
const originalCreateServer = http.createServer;
const sessions = [];
const servers = [];
const httpRequests = [];
beforeEach(() => {
  ControlledWorker.instances.length = 0;
  servers.length = 0;
  workerThreads.Worker = ControlledWorker;
  http.createServer = (...args) => {
    const server = originalCreateServer(...args);
    servers.push(server);
    return server;
  };
  syncBuiltinESMExports();
});
afterEach(async () => {
  for (const session of sessions.splice(0)) await session.close();
  await Promise.allSettled(httpRequests.splice(0));
  workerThreads.Worker = originalWorker;
  http.createServer = originalCreateServer;
  syncBuiltinESMExports();
});

async function session(overrides = {}) {
  const active = await createAsyncNoteSession({
    executable: 'controlled-scene', serviceExecutable: 'controlled-service',
    component: Uint8Array.from([0, 97, 115, 109, 13, 0, 1, 0]),
    document: { profile: 'denied', backingReads: 0, sourceBytes: Buffer.alloc(0), headings: [] },
    ...overrides,
  });
  sessions.push(active);
  return { active, worker: ControlledWorker.instances.at(-1) };
}

function trackedRequest(promise) {
  // Deliberate abort/close cases must also remain handled if an assertion fails.
  promise.catch(() => {});
  httpRequests.push(promise);
  return promise;
}

async function application(overrides = {}) {
  const app = await startLiveNote({ profile: 'granted', note: 'controlled-note.md',
    componentWasm: Uint8Array.from([0, 97, 115, 109, 13, 0, 1, 0]),
    binaryTarget: 'controlled-native-target', ...overrides });
  sessions.push(app);
  const headers = { 'x-a1-session': new URL(app.url).hash.slice(1),
    Origin: app.origin, 'Content-Type': 'application/json' };
  const post = (path, body = {}, generation, extra = {}) => trackedRequest((async () => {
    const response = await fetch(app.origin + path, {
      method: 'POST', headers: { ...headers,
        ...(generation === undefined ? {} : { 'x-a1-input-generation': String(generation) }) },
      body: JSON.stringify(body), ...extra,
    });
    return { status: response.status, body: await response.json() };
  })());
  const readFrame = async () => {
    const response = await fetch(app.origin + '/frame', { headers });
    assert.equal(response.status, 200);
    return response.json();
  };
  return { app, headers, post, readFrame, worker: ControlledWorker.instances.at(-1),
    server: servers.at(-1) };
}

function observeRequest(server, path, finished = true) {
  // The production request listener runs first. Its readInput() end listener
  // settles admission before this observer lets a test continue; no arbitrary
  // timer is used to guess whether a body reached the real server.
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      server.off('request', onRequest);
      reject(new Error('Expected HTTP request was not observed: ' + path));
    }, 5_000);
    const onRequest = request => {
      if (request.url !== path) return;
      server.off('request', onRequest);
      const done = () => { clearTimeout(timeout); resolve(); };
      if (finished) request.once('end', done);
      else done();
    };
    server.on('request', onRequest);
  });
}

function partialInput(app, headers, path, body, generation) {
  let request;
  const response = trackedRequest(new Promise((resolve, reject) => {
    request = http.request(app.origin + path, { method: 'POST', headers: {
      ...headers, 'Content-Length': Buffer.byteLength(body),
      ...(generation === undefined ? {} : { 'x-a1-input-generation': String(generation) }),
    } }, reply => {
      const chunks = [];
      reply.on('data', bytes => chunks.push(bytes));
      reply.once('end', () => resolve({ status: reply.statusCode,
        body: JSON.parse(Buffer.concat(chunks).toString('utf8')) }));
      reply.once('error', reject);
    });
    request.once('error', reject);
    request.write(body.slice(0, 1));
  }));
  return { response, finish: () => request.end(body.slice(1)), abort: () => request.destroy() };
}

test('settled snapshots copy their FIFO boundary without dispatching a native input', async () => {
  const { active, worker } = await session();
  const first = active.scrollTo(10);
  const boundary = active.settledSnapshot();
  const second = active.scrollTo(20);
  assert.equal(active.snapshot().eventsProcessed, 0, 'ordinary snapshots remain nonblocking');
  assert.deepEqual(worker.messages.map(message => message.type), ['scroll']);

  worker.reply(frame(1, 10));
  assert.deepEqual(worker.messages.map(message => [message.type, message.y]), [['scroll', 20]]);
  worker.reply(frame(2, 20));
  const saved = await boundary;
  assert.deepEqual([saved.eventsProcessed, saved.scrollY], [1, 10]);
  assert.deepEqual([(await first).sequence, (await second).sequence], [1, 2]);
  assert.equal(active.snapshot().eventsProcessed, 2);
  saved.pixels.fill(99);
  saved.scene.input.fill(99);
  saved.controlPoint.x = 99;
  saved.noteChoices[0].label = 'Changed';
  assert.deepEqual(active.snapshot(), frame(2, 20), 'read results cannot mutate retained state');
});

test('read barriers share the bounded queue but do not consume the action budget', async () => {
  const { active, worker } = await session({ maxPending: 2, maxEvents: 1 });
  const action = active.scrollTo(10);
  const boundary = active.settledSnapshot();
  await assert.rejects(active.settledSnapshot(), { code: 'A1_QUEUE_FULL' });
  await assert.rejects(active.scrollTo(20), { code: 'A1_QUEUE_FULL' });
  worker.reply(frame(1, 10));
  await action;
  assert.equal((await boundary).eventsProcessed, 1);
  await assert.rejects(active.scrollTo(20), { code: 'A1_EVENT_LIMIT' });
  for (let index = 0; index < 32; index++) {
    assert.deepEqual(await active.settledSnapshot(), frame(1, 10));
  }
  assert.equal(worker.messages.length, 0, 'synchronization performs no worker or source action');
});

test('a failed earlier action publishes its retained frame before the read barrier', async () => {
  const { active, worker } = await session();
  const rejected = assert.rejects(active.scrollTo(10), /native action failed/);
  const boundary = active.settledSnapshot();
  worker.reply(frame(1), { name: 'Error', message: 'native action failed' });
  await rejected;
  assert.deepEqual(await boundary, frame(1));
  const next = active.scrollTo(20);
  worker.reply(frame(2, 20));
  assert.equal((await next).sequence, 2);
});

test('close rejects queued read barriers and leaves the last received snapshot available', async () => {
  const { active, worker } = await session();
  const action = assert.rejects(active.scrollTo(10), { code: 'A1_SESSION_CLOSED' });
  const boundary = assert.rejects(active.settledSnapshot(), { code: 'A1_SESSION_CLOSED' });
  await active.close();
  await Promise.all([action, boundary]);
  assert.equal(worker.terminated, true);
  await assert.rejects(active.settledSnapshot(), { code: 'A1_SESSION_CLOSED' });
  worker.reply(frame(1, 10));
  assert.deepEqual(active.snapshot(), frame(0), 'late worker messages cannot publish after close');
});

test('worker loss rejects queued read barriers without resolving them from stale data', async () => {
  const { active, worker } = await session();
  const action = assert.rejects(active.scrollTo(10), { code: 'A1_WORKER_FAILED' });
  const boundary = assert.rejects(active.settledSnapshot(), { code: 'A1_WORKER_FAILED' });
  worker.emit('exit', 1);
  await Promise.all([action, boundary]);
  await assert.rejects(active.settledSnapshot(), { code: 'A1_WORKER_FAILED' });
  assert.deepEqual(active.snapshot(), frame(0));
});

test('HTTP synchronization fences a waiting old body and later old requests before worker admission',
  { timeout: 15_000 }, async t => {
    const { app, headers, post, readFrame, worker, server } = await application();
    const started = observeRequest(server, '/scroll', false);
    const oldBody = partialInput(app, headers, '/scroll', '{"y":10}');
    t.after(oldBody.abort);
    await started;
    const synchronized = await post('/sync');
    assert.equal(synchronized.status, 200);
    assert.deepEqual([synchronized.body.sequence, synchronized.body.inputGeneration,
      synchronized.body.remainingEvents], [0, 1, 256]);
    oldBody.finish();
    const fencedBody = await oldBody.response;
    assert.deepEqual([fencedBody.status, fencedBody.body.requiresSync,
      fencedBody.body.frame.sequence, fencedBody.body.frame.inputGeneration], [409, true, 0, 1]);

    for (const [path, body] of [['/input', { x: 1, y: 2 }], ['/scroll', { y: 20 }],
      ['/reload', {}], ['/select', { id: 0 }]]) {
      for (const oldGeneration of [undefined, 0]) {
        const rejected = await post(path, body, oldGeneration);
        assert.deepEqual([rejected.status, rejected.body.requiresSync], [409, true]);
        assert.deepEqual(rejected.body.frame, synchronized.body);
      }
    }
    const alreadyStale = partialInput(app, headers, '/scroll', '{"y":20}', 0);
    t.after(alreadyStale.abort);
    assert.equal((await alreadyStale.response).status, 409,
      'an already stale request is rejected without waiting for its incomplete body');
    assert.deepEqual(worker.sent, [], 'fenced requests consume no native work or input budget');

    const admitted = observeRequest(server, '/scroll');
    const fresh = post('/scroll', { y: 20 }, 1);
    await admitted;
    assert.deepEqual(worker.messages.map(message => message.type), ['scroll']);
    worker.reply(frame(1, 20));
    const result = await fresh;
    assert.deepEqual([result.status, result.body.sequence, result.body.inputGeneration], [200, 1, 1]);
    assert.deepEqual(await readFrame(), result.body);
  });

test('HTTP synchronization settles a lost reply at its exact FIFO boundary and captures each generation',
  { timeout: 15_000 }, async () => {
    const { post, readFrame, worker, server } = await application({ maxPending: 4 });
    const abort = new AbortController();
    const admitted = observeRequest(server, '/scroll');
    const lost = post('/scroll', { y: 10 }, undefined, { signal: abort.signal });
    await admitted;
    abort.abort();
    await assert.rejects(lost);

    const firstAccepted = observeRequest(server, '/sync');
    const firstSync = post('/sync');
    await firstAccepted;
    const secondAccepted = observeRequest(server, '/sync');
    const secondSync = post('/sync');
    await secondAccepted;
    const laterAccepted = observeRequest(server, '/scroll');
    const laterAction = post('/scroll', { y: 20 }, 2);
    await laterAccepted;
    const cached = await readFrame();
    assert.deepEqual([cached.sequence, cached.inputGeneration, cached.scrollY], [0, 2, 0],
      'GET /frame is a cheap cached read while synchronization remains queued');

    worker.reply(frame(1, 10));
    assert.deepEqual(worker.messages.map(message => message.y), [20]);
    worker.reply(frame(2, 20));
    const [first, second, later] = await Promise.all([firstSync, secondSync, laterAction]);
    assert.deepEqual([first.status, first.body.sequence, first.body.scrollY, first.body.inputGeneration],
      [200, 1, 10, 1]);
    assert.deepEqual([second.status, second.body.sequence, second.body.scrollY, second.body.inputGeneration],
      [200, 1, 10, 2]);
    assert.equal(first.body.pixels, second.body.pixels);
    assert.notEqual(first.body.pixels, later.body.pixels,
      'later publication cannot replace the boundary copied for synchronization');
    assert.deepEqual([later.status, later.body.sequence, later.body.inputGeneration], [200, 2, 2]);
    assert.equal(worker.sent.length, 2, 'the lost mutation is completed once and never replayed');
  });

test('HTTP synchronization retains bounded queue admission and remains available after event exhaustion',
  { timeout: 15_000 }, async () => {
    const { post, worker, server } = await application({ maxPending: 1, maxEvents: 1 });
    const admitted = observeRequest(server, '/scroll');
    const action = post('/scroll', { y: 10 });
    await admitted;
    const full = await post('/sync');
    assert.deepEqual([full.status, full.body.requiresSync, full.body.frame.inputGeneration,
      full.body.frame.sequence], [429, true, 1, 0]);
    const fenced = await post('/scroll', { y: 20 });
    assert.deepEqual([fenced.status, fenced.body.requiresSync], [409, true]);
    worker.reply(frame(1, 10));
    assert.equal((await action).status, 200);
    const settled = await post('/sync');
    assert.deepEqual([settled.status, settled.body.sequence, settled.body.inputGeneration,
      settled.body.remainingEvents], [200, 1, 2, 0]);
    assert.equal((await post('/scroll', { y: 20 }, 2)).status, 429);
    assert.equal(worker.sent.length, 1);
  });

test('HTTP synchronization validates authority and bounded input without changing read-only grants',
  { timeout: 15_000 }, async () => {
    const { app, headers, post, readFrame, worker } = await application({ uiProfile: 'denied' });
    for (const generation of ['', '-1', '+1', '01', '1.0', 'NaN', '0, 0', '9007199254740992']) {
      const rejected = await post('/scroll', { y: 10 }, generation);
      assert.equal(rejected.status, 400, generation);
    }
    const future = await post('/scroll', { y: 10 }, Number.MAX_SAFE_INTEGER);
    assert.deepEqual([future.status, future.body.requiresSync], [409, true],
      'safe integers parse but cannot choose or advance the current generation');
    for (const body of [null, [], { inputGeneration: 1 }, { note: 'other.md' }]) {
      const rejected = await post('/sync', body);
      assert.deepEqual([rejected.status, rejected.body.requiresSync], [400, true]);
    }
    const oversized = await post('/sync', {}, undefined, { body: 'x'.repeat(129) });
    assert.deepEqual([oversized.status, oversized.body.requiresSync], [413, true]);
    assert.equal((await post('/sync', {}, undefined,
      { headers: { 'Content-Type': 'application/json' } })).status, 403);
    assert.equal((await post('/sync', {}, undefined,
      { headers: { ...headers, Origin: 'https://foreign.example' } })).status, 403);
    const wrongMethod = await fetch(app.origin + '/sync', { headers });
    assert.equal(wrongMethod.status, 404);
    await wrongMethod.arrayBuffer();
    const before = await readFrame();
    assert.equal(before.inputGeneration, 0);
    for (const generation of [1, 2]) {
      const result = await post('/sync');
      assert.equal(result.status, 200);
      assert.deepEqual(result.body, { ...before, inputGeneration: generation });
      assert.equal(result.body.canActivate, false);
      assert.equal(result.body.canReload, false);
    }
    assert.deepEqual(worker.sent, [], 'read-only synchronization causes no worker or source action');
  });

test('HTTP synchronization never succeeds from a cached frame after worker loss or host close',
  { timeout: 15_000 }, async () => {
    for (const stop of ['worker', 'host']) {
      const { app, post, worker, server } = await application();
      const admitted = observeRequest(server, '/scroll');
      const action = post('/scroll', { y: 10 });
      await admitted;
      const synchronized = observeRequest(server, '/sync');
      const sync = post('/sync');
      await synchronized;
      if (stop === 'worker') {
        worker.emit('exit', 1);
        const [failedAction, failedSync] = await Promise.all([action, sync]);
        assert.equal(failedAction.status, 503);
        assert.deepEqual([failedSync.status, failedSync.body.requiresSync,
          failedSync.body.frame.sequence, failedSync.body.frame.inputGeneration], [503, true, 0, 1]);
        assert.equal(failedSync.body.frame.canScroll, false);
        const again = await post('/sync');
        assert.deepEqual([again.status, again.body.requiresSync], [503, true]);
      } else {
        await app.close();
        await assert.rejects(action);
        await assert.rejects(sync);
      }
      assert.equal(worker.terminated, true);
    }
  });
