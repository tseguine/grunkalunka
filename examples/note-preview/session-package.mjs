// Detached A1 session bundle: retained source-compiled component, Markdown guest,
// native hosts and their explicit Node adapters. No build is needed at run time.
import { createHash } from 'node:crypto';
import { realpathSync } from 'node:fs';
import { chmod, lstat, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compileUiService } from './service-interaction.mjs';
import { readBoundedSessionInput } from './session-run.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const buildTarget = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const componentPath = 'artifacts/action.component.wasm';
const guest = 'target/wasm32-unknown-unknown/debug/examples/outline_guest.wasm';
const bin = name => 'target/debug/examples/' + name +
  (process.platform === 'win32' ? '.exe' : '');
const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const MAX_COMPONENT = 4 * 1024 * 1024;
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const usage = 'usage: node examples/note-preview/session-package.mjs ' +
  '--out NEW-DIR [--component HANDLER.wasm] [--live]';

export const sessionPackageInputs = Object.freeze([
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
  guest,
  bin('a1_note_preview'),
  bin('a1_note_scene'),
  bin('a1_ui_action'),
]);

export const liveSessionPackageInputs = Object.freeze([
  ...sessionPackageInputs,
  'examples/note-preview/async-session.mjs',
  'examples/note-preview/session-worker.mjs',
  'examples/note-preview/live-run.mjs',
  'examples/note-preview/live-client.mjs',
  'examples/note-preview/live-page.mjs',
]);

export function parseSessionPackageArguments(args) {
  const options = {};
  const seen = new Set();
  for (let index = 0; index < args.length; index++) {
    const flag = args[index];
    if (!['--out', '--component', '--live'].includes(flag) || seen.has(flag)) {
      throw new Error(usage);
    }
    seen.add(flag);
    if (flag === '--live') {
      options.live = true;
      continue;
    }
    const value = args[++index];
    if (typeof value !== 'string' || !value || value.startsWith('--') || value.includes('\0')) {
      throw new Error(usage);
    }
    options[flag === '--out' ? 'out' : 'componentFile'] = value;
  }
  if (!options.out) throw new Error(usage);
  return options;
}

async function refuseExisting(destination) {
  try {
    await lstat(destination);
  } catch (error) {
    if (error.code === 'ENOENT') return;
    throw error;
  }
  throw new Error('Output directory already exists');
}

export async function packageNoteSession({ out, componentFile, componentWasm, live = false }) {
  if (typeof out !== 'string' || !out || out.includes('\0') || typeof live !== 'boolean' ||
      (componentFile !== undefined && (typeof componentFile !== 'string' ||
        !componentFile || componentFile.includes('\0'))) ||
      (componentWasm !== undefined && (!(componentWasm instanceof Uint8Array) ||
        componentWasm.byteLength > MAX_COMPONENT)) ||
      (componentFile !== undefined && componentWasm !== undefined)) {
    throw new TypeError('Invalid A1 session package arguments');
  }
  let component = componentWasm === undefined ? undefined : Buffer.from(componentWasm);
  const destination = resolve(out);
  await refuseExisting(destination);
  if (component === undefined) {
    component = componentFile === undefined ? compileUiService({
      executable: join(buildTarget, bin('a1_ui_action').slice('target/'.length)),
      source: await readFile(new URL('./activate.js', import.meta.url), 'utf8'),
    }) : await readBoundedSessionInput(componentFile, MAX_COMPONENT, 'A1 component');
  }
  if (component.length < COMPONENT_HEADER.length || component.length > MAX_COMPONENT ||
      !component.subarray(0, COMPONENT_HEADER.length).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }

  const stage = await mkdtemp(join(dirname(destination), '.grunkalunka-a1-session-bundle-'));
  try {
    const files = [];
    const inputs = live ? liveSessionPackageInputs : sessionPackageInputs;
    for (const path of [...inputs, componentPath]) {
      let bytes;
      let mode;
      if (path === componentPath) {
        bytes = component;
      } else {
        const location = path.startsWith('target/') ?
          join(buildTarget, path.slice('target/'.length)) : join(root, path);
        const file = await lstat(location);
        if (!file.isFile()) throw new Error('Missing regular session package input: ' + path);
        bytes = await readFile(location);
        mode = file.mode & 0o777;
      }
      const output = join(stage, path);
      await mkdir(dirname(output), { recursive: true });
      await writeFile(output, bytes);
      if (mode !== undefined && process.platform !== 'win32') await chmod(output, mode);
      const saved = await readFile(output);
      if (!saved.equals(bytes)) throw new Error('Session package copy differs: ' + path);
      files.push({
        path,
        role: path === guest || path === componentPath ? 'portable-wasm' :
          path.startsWith('target/') ? 'native-host' : 'node-host',
        bytes: saved.byteLength, sha256: sha256(saved),
      });
    }
    const manifest = {
      schemaVersion: 1,
      kind: live ? 'a1-note-live-host-bundle' : 'a1-note-session-host-bundle',
      host: {
        platform: process.platform, arch: process.arch,
        nodeMajor: Number(process.versions.node.split('.')[0]),
      },
      files: files.sort((left, right) => left.path.localeCompare(right.path, 'en')),
    };
    await writeFile(join(stage, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
    await refuseExisting(destination);
    await rename(stage, destination);
    return { destination, manifest };
  } catch (error) {
    await rm(stage, { recursive: true, force: true });
    throw error;
  }
}

if (process.argv[1] &&
    realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))) {
  try {
    const { destination, manifest } = await packageNoteSession(
      parseSessionPackageArguments(process.argv.slice(2)));
    console.log(JSON.stringify({
      destination, host: manifest.host,
      portableWasm: manifest.files.filter(file => file.role === 'portable-wasm')
        .map(file => ({ path: file.path, sha256: file.sha256 })),
    }));
  } catch (error) {
    console.error('A1 session package: ' + error.message);
    process.exitCode = 1;
  }
}
