// Protocol-level tests of the real parent adapter with a controlled worker.
// These exercise queue and lifecycle semantics, not native/WIT execution.
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { syncBuiltinESMExports } from 'node:module';
import workerThreads from 'node:worker_threads';
import test, { afterEach, beforeEach } from 'node:test';
import { createAsyncNoteSession } from '../../examples/note-preview/async-session.mjs';

function frame(sequence, scrollY = 0) {
  return { expanded: 1, visibleCount: 1, eventsProcessed: sequence,
    scene: { html: 'frame ' + sequence, visibleCount: 1, input: Buffer.from([sequence]) },
    pixels: Buffer.from([sequence, scrollY]), scrollY, maxScrollY: 100,
    controlPoint: { x: 1, y: 2 }, noteChoices: [{ id: 0, label: 'Note' }], selectedNoteId: 0 };
}

class ControlledWorker extends EventEmitter {
  static instances = [];
  messages = [];
  terminated = false;

  constructor() {
    super();
    ControlledWorker.instances.push(this);
    queueMicrotask(() => this.emit('message', { type: 'ready', snapshot: frame(0) }));
  }

  postMessage(message) { this.messages.push(message); }
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
const sessions = [];
beforeEach(() => {
  ControlledWorker.instances.length = 0;
  workerThreads.Worker = ControlledWorker;
  syncBuiltinESMExports();
});
afterEach(async () => {
  for (const session of sessions.splice(0)) await session.close();
  workerThreads.Worker = originalWorker;
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
