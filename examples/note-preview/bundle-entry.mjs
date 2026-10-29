// Installed A1 bundle entry. Only standard Node modules load before the
// local manifest and every declared payload is checked. This is a reproducible
// local integrity receipt, NOT code signing or an untrusted-module sandbox.
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const executable = name => 'target/debug/examples/' +
  name + (process.platform === 'win32' ? '.exe' : '');
const expectedFiles = [
  'examples/note-preview/bundle-entry.mjs',
  'examples/note-preview/run.mjs',
  'examples/note-preview/scene.mjs',
  'examples/note-preview/render.mjs',
  'tools/runtime/numeric.mjs',
  'tools/runtime/execution.mjs',
  'tools/runtime/execution-worker.mjs',
  'tools/runtime/values.mjs',
  'target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm',
  executable('a1_note_preview'),
  executable('a1_note_scene'),
  'artifacts/toggle.wasm',
].sort();
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

export async function verifyBundle() {
  const manifest = JSON.parse(await readFile(join(root, 'manifest.json'), 'utf8'));
  if (manifest?.schemaVersion !== 1 || manifest.kind !== 'a1-note-preview-host-bundle' ||
    manifest.host?.platform !== process.platform ||
    manifest.host?.arch !== process.arch ||
    manifest.host?.nodeMajor !== Number(process.versions.node.split('.')[0]) ||
    !Array.isArray(manifest.files)) {
    throw new Error('Incompatible A1 host bundle or invalid manifest');
  }
  const paths = manifest.files.map(file => file.path);
  if (JSON.stringify([...paths].sort()) !== JSON.stringify(expectedFiles)) {
    throw new Error('Incomplete or unexpected A1 bundle payload');
  }
  for (const file of manifest.files) {
    const role = file.path === 'artifacts/toggle.wasm' ||
      file.path.endsWith('/outline_guest.wasm') ? 'portable-wasm' :
      file.path.startsWith('target/') ? 'native-host' : 'node-host';
    if (file.role !== role || !Number.isSafeInteger(file.bytes) || file.bytes < 1 ||
      !/^[a-f0-9]{64}$/.test(file.sha256)) {
      throw new Error('Invalid A1 bundle file record: ' + file.path);
    }
    const bytes = await readFile(join(root, file.path));
    if (bytes.byteLength !== file.bytes || sha256(bytes) !== file.sha256) {
      throw new Error('A1 bundle checksum mismatch: ' + file.path);
    }
  }
  const wasm = await readFile(join(root, 'artifacts/toggle.wasm'));
  if (!WebAssembly.validate(wasm)) throw new Error('Packaged toggle Wasm failed validation');
  const module = new WebAssembly.Module(wasm);
  if (WebAssembly.Module.imports(module).length !== 0 ||
    !WebAssembly.Module.exports(module).some(item =>
      item.name === 'run' && item.kind === 'function')) {
    throw new Error('Packaged selector has unexpected ABI or imports');
  }
  return { wasm, manifest };
}

export async function runPackaged(args) {
  const { wasm } = await verifyBundle();
  // Runtime modules are loaded *after* integrity checks, not before.
  const { parseArguments, runPreview } = await import('./run.mjs');
  return runPreview({ ...parseArguments(args), selectorWasm: wasm });
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    console.log(JSON.stringify(await runPackaged(process.argv.slice(2))));
  } catch (error) {
    console.error('A1 bundle: ' + error.message);
    process.exitCode = 1;
  }
}
