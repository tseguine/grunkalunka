// A1 local host-specific packaging experiment. Copies only an explicit set of
// host JS/native artifacts and two independently identified portable Wasm
// artifacts. No network, Cargo, signing, generic installer or release format.
import { createHash } from 'node:crypto';
import { chmod, lstat, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { compileSource } from '../../tools/build/compile.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const buildTarget = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const compiler = join(buildTarget, 'debug',
  'grunkalunka' + (process.platform === 'win32' ? '.exe' : ''));
const noteSource = new URL('./toggle.js', import.meta.url);
const selector = 'artifacts/toggle.wasm';
const guest = 'target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const bin = name => 'target/debug/examples/' +
  name + (process.platform === 'win32' ? '.exe' : '');

export const packageInputs = Object.freeze([
  'examples/note-preview/bundle-entry.mjs',
  'examples/note-preview/run.mjs',
  'examples/note-preview/scene.mjs',
  'examples/note-preview/render.mjs',
  'tools/runtime/numeric.mjs',
  'tools/runtime/execution.mjs',
  'tools/runtime/execution-worker.mjs',
  'tools/runtime/values.mjs',
  guest,
  bin('a1_note_preview'),
  bin('a1_note_scene'),
]);

export function parsePackageArguments(args) {
  if (args.length !== 2 || args[0] !== '--out' || !args[1] ||
    args[1].includes('\0')) {
    throw new Error('usage: node examples/note-preview/package.mjs --out NEW-DIR');
  }
  return { out: args[1] };
}

const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

async function refuseExisting(destination) {
  try {
    await lstat(destination);
  } catch (error) {
    if (error.code === 'ENOENT') return;
    throw error;
  }
  throw new Error('Output directory already exists');
}

export async function packageNotePreview({ out }) {
  if (typeof out !== 'string' || !out || out.includes('\0')) {
    throw new TypeError('Expected an explicit package directory');
  }
  const destination = resolve(out);
  await refuseExisting(destination);

  // Package a real compiler-emitted numeric core Wasm module instead of a
  // host-reimplemented toggle. No compiler binary is shipped.
  const source = await readFile(noteSource, 'utf8');
  const { wasm } = await compileSource({ compiler, source, language: 'js', cwd: root });
  if (!WebAssembly.validate(wasm)) throw new Error('Toggle is not valid Wasm');
  const module = new WebAssembly.Module(wasm);
  if (WebAssembly.Module.imports(module).length !== 0 ||
    !WebAssembly.Module.exports(module).some(item => item.name === 'run' && item.kind === 'function')) {
    throw new Error('Toggle must export run without imports');
  }

  const stage = await mkdtemp(join(dirname(destination), '.grunkalunka-a1-bundle-'));
  try {
    const files = [];
    for (const path of [...packageInputs, selector]) {
      let bytes;
      let mode;
      if (path === selector) {
        bytes = wasm;
      } else {
        const location = path.startsWith('target/') ?
          join(buildTarget, path.slice('target/'.length)) : join(root, path);
        const file = await lstat(location);
        if (!file.isFile()) throw new Error('Missing regular package input: ' + path);
        bytes = await readFile(location);
        mode = file.mode & 0o777;
      }
      const dest = join(stage, path);
      await mkdir(dirname(dest), { recursive: true });
      await writeFile(dest, bytes);
      if (mode !== undefined && process.platform !== 'win32') {
        await chmod(dest, mode);
      }
      const role = path === guest || path === selector ? 'portable-wasm' :
        path.startsWith('target/') ? 'native-host' : 'node-host';
      const saved = await readFile(dest);
      if (!saved.equals(bytes)) throw new Error('Package copy differs: ' + path);
      files.push({ path, role, bytes: saved.byteLength, sha256: sha256(saved) });
    }

    const manifest = {
      schemaVersion: 1,
      kind: 'a1-note-preview-host-bundle',
      host: {
        platform: process.platform,
        arch: process.arch,
        nodeMajor: Number(process.versions.node.split('.')[0]),
      },
      files: files.sort((a, b) => a.path.localeCompare(b.path, 'en')),
    };
    await writeFile(join(stage, 'manifest.json'),
      JSON.stringify(manifest, null, 2) + '\n', 'utf8');
    await refuseExisting(destination);
    await rename(stage, destination);
    return { destination, manifest };
  } catch (error) {
    await rm(stage, { recursive: true, force: true });
    throw error;
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const { out } = parsePackageArguments(process.argv.slice(2));
    const { destination, manifest } = await packageNotePreview({ out });
    console.log(JSON.stringify({
      destination, portableWasm: manifest.files.filter(file => file.role === 'portable-wasm')
        .map(file => ({ path: file.path, sha256: file.sha256 })),
      host: manifest.host,
    }));
  } catch (error) {
    console.error('A1 package: ' + error.message);
    process.exitCode = 1;
  }
}
