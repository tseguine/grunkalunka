// Fixed internal worker entry: native calls and retained state never execute on
// the caller's event loop. It adds scheduling, not a new component authority.
import { parentPort, workerData } from 'node:worker_threads';
import { createNoteSession } from './session.mjs';

function describeError(error) {
  const described = { name: error.name, message: error.message };
  if (error instanceof RangeError && error.message === 'A1 session event limit exceeded') {
    described.code = 'A1_EVENT_LIMIT';
  }
  return described;
}

let session;
try {
  session = createNoteSession(workerData);
  parentPort.postMessage({ type: 'ready', snapshot: session.snapshot() });
} catch (error) {
  parentPort.postMessage({ type: 'initialization-error', error: describeError(error) });
  parentPort.close();
}

if (session !== undefined) parentPort.on('message', message => {
  if (message?.type !== 'activate' || !Number.isSafeInteger(message.id) || message.id < 1) {
    throw new Error('Invalid A1 worker input');
  }
  let result;
  let error;
  try {
    result = session.activate(message.pointer);
  } catch (cause) {
    error = describeError(cause);
  }
  parentPort.postMessage({ type: 'result', id: message.id, ok: error === undefined,
    result, error, snapshot: session.snapshot() });
});
