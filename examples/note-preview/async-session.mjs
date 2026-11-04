// Private A1 scheduling adapter. The worker owns the synchronous native session;
// this thread only keeps the latest completed snapshot and a bounded input queue.
import { Worker } from 'node:worker_threads';
import { prepareSelectedNoteScene } from './scene.mjs';

const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const MAX_COMPONENT = 4 * 1024 * 1024;
const MAX_EVENTS = 256;
const MAX_PENDING = 16;

function ownedOptions({ executable, serviceExecutable, component, document,
  expanded = 1, uiProfile = 'granted', maxEvents = MAX_EVENTS, maxPending = 8 }) {
  if (![executable, serviceExecutable].every(value =>
    typeof value === 'string' && value.length > 0 && !value.includes('\0')) ||
      ![0, 1].includes(expanded) || !['granted', 'denied'].includes(uiProfile) ||
      !Number.isSafeInteger(maxEvents) || maxEvents < 1 || maxEvents > MAX_EVENTS ||
      !Number.isSafeInteger(maxPending) || maxPending < 1 || maxPending > MAX_PENDING) {
    throw new TypeError('Invalid asynchronous A1 session arguments');
  }
  if (!(component instanceof Uint8Array) || component.byteLength < COMPONENT_HEADER.length ||
      component.byteLength > MAX_COMPONENT) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
  if (document === null || typeof document !== 'object' ||
      !(document.sourceBytes instanceof Uint8Array) || document.sourceBytes.byteLength > 4096 ||
      !Array.isArray(document.headings) || document.headings.length > 64) {
    throw new TypeError('Expected a bounded authorized A1 document');
  }
  // Copy only known bounded fields before starting the worker or yielding.
  // Fresh Uint8Arrays transfer their exact views, not a pooled Buffer's backing
  // allocation or arbitrary extra properties supplied by a caller.
  const payload = new Uint8Array(component);
  if (!Buffer.from(payload.subarray(0, COMPONENT_HEADER.length)).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
  const owned = {
    profile: document.profile, backingReads: document.backingReads,
    sourceBytes: new Uint8Array(document.sourceBytes),
    headings: document.headings.map(heading => {
      if (heading === null || typeof heading !== 'object' ||
          typeof heading.text !== 'string' || Buffer.byteLength(heading.text, 'utf8') > 4096) {
        throw new TypeError('Expected a bounded owned WIT heading');
      }
      return { level: heading.level, start: heading.start, end: heading.end,
        sectionEnd: heading.sectionEnd, parent: heading.parent, text: heading.text };
    }),
  };
  if (owned.profile === 'denied' && owned.sourceBytes.byteLength !== 0) {
    throw new Error('Denied session disclosed protected bytes');
  }
  // The existing pure projection validates scalar fields, ranges, ancestry and
  // the aggregate label quota before anything crosses the worker boundary.
  prepareSelectedNoteScene({ ...owned, visibleCount: 0 });
  return { maxPending, options: { executable, serviceExecutable, component: payload,
    document: owned, expanded, uiProfile, maxEvents } };
}

const copyScene = scene => ({ html: scene.html, visibleCount: scene.visibleCount,
  input: Buffer.from(scene.input) });
const copySnapshot = snapshot => ({ expanded: snapshot.expanded,
  visibleCount: snapshot.visibleCount, eventsProcessed: snapshot.eventsProcessed,
  scene: copyScene(snapshot.scene), pixels: Buffer.from(snapshot.pixels) });
const copyAction = action => ({ sequence: action.sequence, target: action.target,
  activated: action.activated, changed: action.changed, beforeExpanded: action.beforeExpanded,
  expanded: action.expanded, before: copyScene(action.before), after: copyScene(action.after),
  previousPixels: Buffer.from(action.previousPixels), pixels: Buffer.from(action.pixels),
  service: action.service === null ? null : { ...action.service } });

function codedError(code, message, ErrorType = Error) {
  const error = new ErrorType(message);
  error.code = code;
  return error;
}

function receivedError(error) {
  const ErrorType = error?.name === 'RangeError' ? RangeError :
    error?.name === 'TypeError' ? TypeError : Error;
  const received = new ErrorType(error?.message ?? 'A1 session worker failed');
  if (error?.code === 'A1_EVENT_LIMIT') received.code = error.code;
  return received;
}

export async function createAsyncNoteSession(arguments_) {
  const { options, maxPending } = ownedOptions(arguments_);
  let worker;
  try {
    worker = new Worker(new URL('./session-worker.mjs', import.meta.url), {
      workerData: options, execArgv: [],
      transferList: [options.component.buffer, options.document.sourceBytes.buffer],
    });
  } catch (error) {
    throw codedError('A1_WORKER_FAILED', error.message);
  }
  let state = 'starting';
  let latest;
  let current = null;
  let nextId = 0;
  const queued = [];
  let terminalError;
  let stopped;
  let readyResolve;
  let readyReject;
  const ready = new Promise((resolve, reject) => {
    readyResolve = resolve;
    readyReject = reject;
  });

  function stopWorker() {
    stopped ??= worker.terminate().then(() => undefined);
    return stopped;
  }

  function rejectPending(error) {
    current?.reject(error);
    current = null;
    for (const request of queued.splice(0)) request.reject(error);
  }

  function fail(error) {
    if (state === 'closed' || state === 'failed') return;
    state = 'failed';
    terminalError = codedError('A1_WORKER_FAILED', error.message);
    readyReject(terminalError);
    rejectPending(terminalError);
    // Event handlers cannot await cleanup. close() and failed initialization
    // still expose/wait for the same termination promise.
    void stopWorker().catch(() => {});
  }

  function dispatch() {
    if (state !== 'open' || current !== null || queued.length === 0) return;
    current = queued.shift();
    try {
      worker.postMessage({ type: 'activate', id: current.id, pointer: current.pointer });
    } catch (error) {
      fail(error);
    }
  }

  worker.on('message', message => {
    if (state === 'closed' || state === 'failed') return;
    try {
      if (state === 'starting') {
        if (message?.type === 'initialization-error') throw receivedError(message.error);
        if (message?.type !== 'ready') throw new Error('Invalid A1 worker initialization response');
        latest = copySnapshot(message.snapshot);
        state = 'open';
        readyResolve();
        return;
      }
      if (message?.type !== 'result' || current === null || message.id !== current.id ||
          typeof message.ok !== 'boolean') {
        throw new Error('Invalid A1 worker activation response');
      }
      // Even failed native actions consume an attempt and return the preceding
      // committed view. Publish that snapshot before settling the input promise.
      const snapshot = copySnapshot(message.snapshot);
      const result = message.ok ? copyAction(message.result) : receivedError(message.error);
      const request = current;
      current = null;
      latest = snapshot;
      if (message.ok) request.resolve(result);
      else request.reject(result);
      dispatch();
    } catch (error) {
      fail(error);
    }
  });
  worker.on('error', fail);
  worker.on('exit', code => {
    if (state !== 'closed' && state !== 'failed') {
      fail(new Error(`A1 session worker exited unexpectedly (${code})`));
    }
  });

  function snapshot() {
    // The last received view remains inspectable after closing or worker loss.
    // It is always copied and is never a promise for unfinished native work.
    return copySnapshot(latest);
  }

  function activate(pointer) {
    try {
      if (state !== 'open') throw terminalError;
      const x = pointer?.x;
      const y = pointer?.y;
      if (!Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
          x < 0 || y < 0 || x > 0xffffffff || y > 0xffffffff) {
        throw new TypeError('Invalid A1 session pointer');
      }
      if (latest.eventsProcessed >= options.maxEvents) {
        throw codedError('A1_EVENT_LIMIT', 'A1 session event limit exceeded', RangeError);
      }
      if (queued.length + (current === null ? 0 : 1) >= maxPending) {
        throw codedError('A1_QUEUE_FULL', 'A1 session input queue is full', RangeError);
      }
      return new Promise((resolve, reject) => {
        queued.push({ id: ++nextId, pointer: { x, y }, resolve, reject });
        dispatch();
      });
    } catch (error) {
      return Promise.reject(error);
    }
  }

  function close() {
    if (state !== 'closed' && state !== 'failed') {
      state = 'closed';
      terminalError = codedError('A1_SESSION_CLOSED', 'A1 session is closed');
      rejectPending(terminalError);
    }
    // Stop admission/reject pending work immediately. Worker termination can
    // wait for an already-running synchronous native child and its own timeout.
    return stopWorker();
  }

  try {
    await ready;
  } catch (error) {
    await stopWorker().catch(() => {});
    throw error;
  }
  return Object.freeze({ snapshot, activate, close });
}
