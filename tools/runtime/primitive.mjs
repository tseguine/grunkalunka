import { executeProfile } from './execution.mjs';
export { ExecutionError } from './execution.mjs';

/**
 * Execute one trusted primitive-return-v1 artifact in a disposable worker.
 * Returns the decoded JS primitive; malformed wire results reject, not coerce.
 * Shares runNumeric's input snapshot, import denial and worker lifetime rules.
 * @param {Uint8Array} wasm Includes Node Buffers; shared backing memory is rejected.
 * @param {number} a
 * @param {number} b
 * @param {{timeout?: number, signal?: AbortSignal}} [options]
 * @returns {Promise<undefined|null|boolean|number>}
 */
export async function runPrimitive(wasm, a, b, options) {
  return executeProfile('primitive', wasm, a, b, options);
}
