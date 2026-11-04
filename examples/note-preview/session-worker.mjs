// Fixed internal worker entry: native calls and retained state never execute on
// the caller's event loop. It adds scheduling, not a new component authority.
import { parentPort, workerData } from 'node:worker_threads';
import { createNoteSession } from './session.mjs';
import { loadNoteDocumentSync } from './run.mjs';

function describeError(error) {
  const described = { name: error.name, message: error.message };
  if (error instanceof RangeError && error.message === 'A1 session event limit exceeded') {
    described.code = 'A1_EVENT_LIMIT';
  } else if (error.code === 'A1_RELOAD_UNAVAILABLE') {
    described.code = error.code;
  }
  return described;
}

// The document module has an asynchronous ESM entry path. Finish module
// bootstrap before exposing ready/error: the parent may immediately terminate
// this worker on either response, including an initialization failure.
setImmediate(() => {
  let session;
  try {
    const reloadDocument = workerData.documentSource === undefined ? undefined :
      () => loadNoteDocumentSync(workerData.documentSource);
    session = createNoteSession({ ...workerData,
      ...(reloadDocument === undefined ? {} : { document: reloadDocument(), reloadDocument }) });
  } catch (error) {
    parentPort.postMessage({ type: 'initialization-error', error: describeError(error) });
    parentPort.close();
    return;
  }

  parentPort.on('message', message => {
    if (!['activate', 'scroll', 'reload'].includes(message?.type) ||
        !Number.isSafeInteger(message.id) || message.id < 1) {
      throw new Error('Invalid A1 worker input');
    }
    let result;
    let error;
    try {
      result = message.type === 'activate' ? session.activate(message.pointer) :
        message.type === 'scroll' ? session.scrollTo(message.y) : session.reload();
    } catch (cause) {
      error = describeError(cause);
    }
    parentPort.postMessage({ type: 'result', id: message.id, ok: error === undefined,
      result, error, snapshot: session.snapshot() });
  });
  parentPort.postMessage({ type: 'ready', snapshot: session.snapshot() });
});
