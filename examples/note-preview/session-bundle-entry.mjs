// Only built-in modules load before this bundle's exact payload is checked.
// Hashes provide a local integrity receipt, not signatures or an execution sandbox.
import { createHash } from 'node:crypto';
import { realpathSync } from 'node:fs';
import { lstat, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const componentPath = 'artifacts/action.component.wasm';
const guest = 'target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const bin = name => 'target/debug/examples/' + name +
  (process.platform === 'win32' ? '.exe' : '');
const expectedFiles = [
  'examples/note-preview/session-bundle-entry.mjs',
  'examples/note-preview/session-run.mjs',
  'examples/note-preview/session.mjs',
  'examples/note-preview/service-interaction.mjs',
  'examples/note-preview/run.mjs',
  'examples/note-preview/scene.mjs',
  'examples/note-preview/render.mjs',
  'tools/runtime/numeric.mjs',
  'tools/runtime/execution.mjs',
  'tools/runtime/execution-worker.mjs',
  'tools/runtime/values.mjs',
  guest, bin('a1_note_preview'), bin('a1_note_scene'), bin('a1_ui_action'), componentPath,
].sort();
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);

export async function verifySessionBundle() {
  const manifest = JSON.parse(await readFile(join(root, 'manifest.json'), 'utf8'));
  if (manifest?.schemaVersion !== 1 || manifest.kind !== 'a1-note-session-host-bundle' ||
      manifest.host?.platform !== process.platform || manifest.host?.arch !== process.arch ||
      manifest.host?.nodeMajor !== Number(process.versions.node.split('.')[0]) ||
      !Array.isArray(manifest.files) || manifest.files.some(file =>
        file === null || typeof file !== 'object' || typeof file.path !== 'string') ||
      JSON.stringify(manifest.files.map(file => file.path).sort()) !== JSON.stringify(expectedFiles)) {
    throw new Error('Incompatible A1 session host bundle or invalid manifest');
  }
  let componentWasm;
  for (const file of manifest.files) {
    const role = file.path === componentPath || file.path === guest ? 'portable-wasm' :
      file.path.startsWith('target/') ? 'native-host' : 'node-host';
    if (file.role !== role || !Number.isSafeInteger(file.bytes) || file.bytes < 1 ||
        !/^[a-f0-9]{64}$/.test(file.sha256)) {
      throw new Error('Invalid A1 session bundle file record: ' + file.path);
    }
    const path = join(root, file.path);
    if (!(await lstat(path)).isFile()) throw new Error('Non-regular session bundle input: ' + file.path);
    const bytes = await readFile(path);
    if (bytes.byteLength !== file.bytes || sha256(bytes) !== file.sha256) {
      throw new Error('A1 session bundle checksum mismatch: ' + file.path);
    }
    if (file.path === componentPath) componentWasm = bytes;
  }
  // A component is not a core Wasm module accepted by WebAssembly.validate.
  // Its actual linking/execution checks occur in the native component host.
  if (componentWasm.length < COMPONENT_HEADER.length || componentWasm.length > 4 * 1024 * 1024 ||
      !componentWasm.subarray(0, COMPONENT_HEADER.length).equals(COMPONENT_HEADER)) {
    throw new Error('Invalid packaged A1 component header or size');
  }
  return { componentWasm, manifest };
}

export async function runPackagedSession(args) {
  const { componentWasm } = await verifySessionBundle();
  const { parseSessionArguments, runNoteSession } = await import('./session-run.mjs');
  const options = parseSessionArguments(args);
  if (options.componentPath !== undefined) {
    throw new Error('Packaged sessions use their retained component');
  }
  return runNoteSession({ ...options, componentWasm, binaryTarget: join(root, 'target') });
}

if (process.argv[1] &&
    realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))) {
  try {
    console.log(JSON.stringify(await runPackagedSession(process.argv.slice(2))));
  } catch (error) {
    console.error('A1 session bundle: ' + error.message);
    process.exitCode = 1;
  }
}
