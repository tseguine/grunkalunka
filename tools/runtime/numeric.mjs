import { executeProfile } from './execution.mjs';
export { ExecutionError } from './execution.mjs';

/**
 * Execute one trusted, import-free numeric artifact in a fresh disposable worker.
 * Copies the selected bytes, never transfers the caller's buffer, and waits for
 * worker termination before settling. This is not a hostile-code memory sandbox.
 * @param {Uint8Array} wasm Includes Node Buffers; shared backing memory is rejected.
 * @param {number} a
 * @param {number} b
 * @param {{timeout?: number, signal?: AbortSignal}} [options]
 * @returns {Promise<number>}
 */
export async function runNumeric(wasm, a, b, options) {
  return executeProfile('numeric', wasm, a, b, options);
}
