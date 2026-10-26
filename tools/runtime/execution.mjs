// Internal shared worker lifecycle for the numeric and primitive entry points.
import { Worker } from 'node:worker_threads';
import { decodePrimitiveResult } from './values.mjs';

export class ExecutionError extends Error {
  constructor(code, message, cause) {
    super(message, { cause });
    this.name = 'ExecutionError';
    this.code = code;
  }
}

export async function executeProfile(profile, wasm, a, b, { timeout = 5000, signal } = {}) {
  if (!(wasm instanceof Uint8Array) || !(wasm.buffer instanceof ArrayBuffer)) {
    throw new TypeError('wasm must be a Uint8Array with non-shared backing memory');
  }
  if (typeof a !== 'number' || typeof b !== 'number') throw new TypeError('a and b must be Numbers');
  if (!Number.isInteger(timeout) || timeout < 1 || timeout > 2 ** 31 - 1) {
    throw new RangeError('timeout must be an integer from 1 through 2147483647');
  }
  if (signal !== undefined && !(signal instanceof AbortSignal)) throw new TypeError('signal must be an AbortSignal');
  const label = profile === 'primitive' ? 'Primitive' : 'Numeric';
  const aborted = () => new ExecutionError('ABORT_ERR', `${label} execution aborted`, signal?.reason);
  if (signal?.aborted) throw aborted();
  const bytes = new Uint8Array(wasm); // Snapshot only the view, not an entire pooled Buffer.
  const worker = new Worker(new URL('./execution-worker.mjs', import.meta.url), {
    workerData: { wasm: bytes, a, b, profile }, transferList: [bytes.buffer], execArgv: [],
  });
  let timer;
  let onAbort;
  try {
    return await new Promise((resolve, reject) => {
      worker.once('message', value => {
        try {
          if (profile === 'primitive') resolve(decodePrimitiveResult(value));
          else if (typeof value === 'number') resolve(value);
          else reject(new ExecutionError('ERR_WASM_EXECUTION', 'Worker returned a nonnumeric result'));
        } catch (cause) {
          reject(new ExecutionError('ERR_WASM_EXECUTION', cause.message, cause));
        }
      });
      worker.once('error', cause => reject(new ExecutionError('ERR_WASM_EXECUTION', cause.message, cause)));
      worker.once('exit', code => reject(new ExecutionError('ERR_WASM_EXECUTION', `Worker exited without a result (${code})`)));
      timer = setTimeout(() => reject(new ExecutionError('ERR_WASM_TIMEOUT', `${label} execution timed out`)), timeout);
      onAbort = () => reject(aborted());
      signal?.addEventListener('abort', onAbort, { once: true });
      if (signal?.aborted) onAbort();
    });
  } finally {
    clearTimeout(timer);
    if (onAbort) signal?.removeEventListener('abort', onAbort);
    await worker.terminate();
  }
}
