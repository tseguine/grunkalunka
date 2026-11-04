// Fixed internal worker entry: native calls and retained state never execute on
// the caller's event loop. It adds scheduling, not a new component authority.
import { parentPort, workerData } from 'node:worker_threads';
import { createNoteSession } from './session.mjs';
import { loadNoteDocumentSync } from './run.mjs';

function describeError(error) {
  const described = { name: error.name, message: error.message };
  if (error instanceof RangeError && error.message === 'A1 session event limit exceeded') {
    described.code = 'A1_EVENT_LIMIT';
  } else if (['A1_RELOAD_UNAVAILABLE', 'A1_SELECTION_UNAVAILABLE'].includes(error.code)) {
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
    const source = workerData.documentSource;
    let options = workerData;
    if (source?.profile === 'granted') {
      const load = id => loadNoteDocumentSync({ profile: source.profile,
        note: source.notes[id].note, binaryTarget: source.binaryTarget });
      options = { ...workerData, document: load(0), noteSelection: {
        labels: source.notes.map(note => note.label), selectedId: 0, load,
      } };
    } else if (source !== undefined) {
      const reloadDocument = () => loadNoteDocumentSync(source);
      options = { ...workerData, document: reloadDocument(), reloadDocument };
    }
    session = createNoteSession(options);
  } catch (error) {
    parentPort.postMessage({ type: 'initialization-error', error: describeError(error) });
    parentPort.close();
    return;
  }

  parentPort.on('message', message => {
    if (!['activate', 'scroll', 'reload', 'select'].includes(message?.type) ||
        !Number.isSafeInteger(message.id) || message.id < 1) {
      throw new Error('Invalid A1 worker input');
    }
    let result;
    let error;
    try {
      result = message.type === 'activate' ? session.activate(message.pointer) :
        message.type === 'scroll' ? session.scrollTo(message.y) :
        message.type === 'select' ? session.selectNote(message.noteId) : session.reload();
    } catch (cause) {
      error = describeError(cause);
    }
    parentPort.postMessage({ type: 'result', id: message.id, ok: error === undefined,
      result, error, snapshot: session.snapshot() });
  });
  parentPort.postMessage({ type: 'ready', snapshot: session.snapshot() });
});
