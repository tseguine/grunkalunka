import { resolve } from 'node:path';
import { invoke } from './process.mjs';

export { CompilerError } from './process.mjs';

function pathString(value, name) {
  if (typeof value !== 'string' || value.length === 0 || value.includes('\0')) {
    throw new TypeError(`${name} must be a nonempty path string without NUL bytes`);
  }
}

/**
 * Compile one JS/TS source with an explicitly selected native compiler.
 * Paths resolve against cwd; there is no PATH search, shell or implicit build.
 * Success returns { destination, stdout, stderr }. Failures reject; files are
 * never removed or overwritten by this adapter, including after cancellation.
 * @param {{compiler: string, source: string, destination: string,
 *   profile?: 'numeric'|'primitive', cwd?: string, signal?: AbortSignal,
 *   timeout?: number, maxBuffer?: number}} options
 */
export async function compile({
  compiler, source, destination, profile = 'numeric', cwd = process.cwd(),
  signal, timeout = 30_000, maxBuffer = 1024 * 1024,
}) {
  for (const [name, value] of Object.entries({ compiler, source, destination, cwd })) {
    pathString(value, name);
  }
  if (profile !== 'numeric' && profile !== 'primitive') {
    throw new TypeError('profile must be numeric or primitive');
  }
  for (const [name, value] of Object.entries({ timeout, maxBuffer })) {
    if (!Number.isInteger(value) || value < 1 || value > 2 ** 31 - 1) {
      throw new RangeError(`${name} must be an integer from 1 through 2147483647`);
    }
  }
  if (signal !== undefined && !(signal instanceof AbortSignal)) {
    throw new TypeError('signal must be an AbortSignal');
  }
  const root = resolve(cwd);
  const output = resolve(root, destination);
  const result = await invoke(resolve(root, compiler), [
    profile === 'primitive' ? 'compile-value' : 'compile',
    resolve(root, source), output,
  ], { cwd: root, signal, timeout, maxBuffer });
  return { destination: output, ...result };
}
