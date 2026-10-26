import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
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

/**
 * Compile source text and return { wasm: Buffer, stdout, stderr }.
 * Uses temporary files, not an in-process compiler. The owned directory is
 * removed before settling; caller files and the temporary parent are retained.
 * @param {{compiler: string, source: string, language: 'js'|'ts',
 *   profile?: 'numeric'|'primitive', cwd?: string, tempDirectory?: string,
 *   signal?: AbortSignal, timeout?: number, maxBuffer?: number}} options
 */
export async function compileSource({
  source, language, cwd = process.cwd(), tempDirectory = tmpdir(), ...options
}) {
  if (typeof source !== 'string' || /[\uD800-\uDFFF]/u.test(source)) {
    throw new TypeError('source must be a string without unpaired UTF-16 surrogates');
  }
  if (language !== 'js' && language !== 'ts') {
    throw new TypeError('language must be js or ts');
  }
  pathString(cwd, 'cwd');
  pathString(tempDirectory, 'tempDirectory');
  const root = resolve(cwd);
  const directory = await fs.mkdtemp(join(resolve(root, tempDirectory), 'grunkalunka-source-'));
  let failure;
  try {
    const input = join(directory, `input.${language}`);
    const output = join(directory, 'output.wasm');
    await fs.writeFile(input, source, 'utf8');
    const { stdout, stderr } = await compile({
      ...options, cwd: root, source: input, destination: output,
    });
    const wasm = await fs.readFile(output);
    return { wasm, stdout, stderr };
  } catch (error) {
    failure = error;
    throw error;
  } finally {
    try {
      await fs.rm(directory, { recursive: true, force: true });
    } catch (cleanupError) {
      if (failure !== undefined) {
        throw new AggregateError([failure, cleanupError], 'Compilation and temporary cleanup failed', {
          cause: failure,
        });
      }
      throw cleanupError;
    }
  }
}
