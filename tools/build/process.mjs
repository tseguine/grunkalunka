// Internal subprocess adapter. The public build entry point is compile.mjs.
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);

export class CompilerError extends Error {
  constructor(cause) {
    super(cause.message, { cause });
    this.name = 'CompilerError';
    this.code = cause.code ?? null;
    this.signal = cause.signal ?? null;
    this.killed = cause.killed ?? false;
    this.stdout = cause.stdout ?? '';
    this.stderr = cause.stderr ?? '';
  }
}

export async function invoke(executable, args, { cwd, signal, timeout, maxBuffer }) {
  try {
    // Node can spawn before handling an already-aborted signal. Avoid any launch.
    if (signal?.aborted) {
      throw Object.assign(new Error('Compilation aborted', { cause: signal.reason }), {
        code: 'ABORT_ERR',
      });
    }
    const pending = execute(executable, args, {
      cwd, signal, timeout, maxBuffer, encoding: 'utf8',
      shell: false, windowsHide: true, killSignal: 'SIGKILL',
    });
    // An abort can reject before close. Do not return with the direct child alive.
    const closed = pending.child
      ? new Promise(resolve => pending.child.once('close', resolve))
      : Promise.resolve();
    try {
      return await pending;
    } finally {
      await closed;
    }
  } catch (cause) {
    throw new CompilerError(cause);
  }
}
