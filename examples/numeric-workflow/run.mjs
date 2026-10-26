import { readFile } from 'node:fs/promises';
import { extname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { compileSource } from '../../tools/build/compile.mjs';

const defaultSource = fileURLToPath(new URL('./gcd.js', import.meta.url));
const defaultCompiler = fileURLToPath(new URL(
  `../../target/debug/grunkalunka${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url,
));
const usage = 'usage: node examples/numeric-workflow/run.mjs [A B [SOURCE.js|SOURCE.ts]]';

export function parseArguments(args) {
  if (![0, 2, 3].includes(args.length)) throw new Error(usage);
  const values = args.length ? args.slice(0, 2) : ['84', '30'];
  const inputs = values.map(value => {
    if (!/^[+-]?\d+$/.test(value) || !Number.isSafeInteger(Number(value))) {
      throw new Error('A and B must be decimal safe integers');
    }
    return Number(value);
  });
  const sourcePath = args.length === 3 ? resolve(args[2]) : defaultSource;
  const language = extname(sourcePath).slice(1);
  if (language !== 'js' && language !== 'ts') throw new Error('source extension must be .js or .ts');
  return { inputs, sourcePath, language };
}

export async function runExample(args) {
  const { inputs, sourcePath, language } = parseArguments(args);
  try {
    // Decode strictly and retain a BOM: source bytes must not silently change.
    const source = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true })
      .decode(await readFile(sourcePath));
    const { wasm, stderr } = await compileSource({
      compiler: process.env.GRUNKALUNKA_COMPILER ?? defaultCompiler,
      source, language,
    });
    if (stderr) process.stderr.write(stderr);
    const { instance } = await WebAssembly.instantiate(wasm);
    if (typeof instance.exports.run !== 'function') throw new Error('module has no run function');
    return instance.exports.run(...inputs);
  } catch (cause) {
    throw new Error(`Cannot run ${sourcePath}: ${cause.stderr || cause.message}`, { cause });
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    console.log(await runExample(process.argv.slice(2)));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
