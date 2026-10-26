import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { compileSource } from '../../tools/build/compile.mjs';
import { runNumeric } from '../../tools/runtime/numeric.mjs';
import { parseArguments } from './run.mjs';
import { followSource } from './follow.mjs';

const defaultCompiler = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));

// The command's driver is exported for integration tests, not a general watch API.
export async function watchExample(args, {
  signal, interval = 250, compiler = process.env.GRUNKALUNKA_COMPILER ?? defaultCompiler,
  report,
} = {}) {
  const { inputs, sourcePath, language } = parseArguments(args);
  const publish = report ?? (outcome => {
    if (Object.hasOwn(outcome, 'error')) {
      const error = outcome.error;
      console.error(`Cannot run ${sourcePath}: ${error?.stderr || error?.message || String(error)}`);
    } else {
      if (outcome.value.stderr) process.stderr.write(outcome.value.stderr);
      console.log('RESULT', outcome.value.result);
    }
  });
  await followSource(sourcePath, async (bytes, signal) => {
    const source = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes);
    const { wasm, stderr } = await compileSource({ compiler, source, language, signal });
    const result = await runNumeric(wasm, ...inputs, { signal });
    return { result, stderr };
  }, publish, { signal, interval });
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const controller = new AbortController();
  const interrupt = () => { process.exitCode = 130; controller.abort(); };
  const terminate = () => { process.exitCode = 143; controller.abort(); };
  process.once('SIGINT', interrupt);
  process.once('SIGTERM', terminate);
  try { await watchExample(process.argv.slice(2), { signal: controller.signal }); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
  finally {
    process.removeListener('SIGINT', interrupt);
    process.removeListener('SIGTERM', terminate);
  }
}
