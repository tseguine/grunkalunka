// Runnable, single-activation A1 application. The source-imported service owns
// the state change; the host supplies explicit document authority and input.
import { realpathSync } from 'node:fs';
import { lstat, mkdtemp, open, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadNoteDocument } from './run.mjs';
import { activateNoteHeadingWithService, compileUiService } from './service-interaction.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const native = name => join(target, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const usage = 'usage: node examples/note-preview/interact.mjs ' +
  '--profile granted|denied --out DIR --x X --y Y [--note NOTE.md] ' +
  '[--ui-profile granted|denied] [--component HANDLER.wasm] [--collapsed] [--debug]';

export function parseInteractionArguments(args) {
  const options = { profile: null, out: null, note: null, x: null, y: null,
    uiProfile: 'granted', expanded: 1, debug: false };
  const fields = { '--profile': 'profile', '--out': 'out', '--note': 'note',
    '--x': 'x', '--y': 'y', '--ui-profile': 'uiProfile', '--component': 'componentPath' };
  const seen = new Set();
  for (let index = 0; index < args.length; index++) {
    const flag = args[index];
    if (seen.has(flag)) throw new Error(usage);
    seen.add(flag);
    if (flag === '--collapsed') options.expanded = 0;
    else if (flag === '--debug') options.debug = true;
    else {
      if (!Object.hasOwn(fields, flag)) throw new Error(usage);
      const value = args[++index];
      if (!value || value.startsWith('--') || value.includes('\0')) throw new Error(usage);
      if (flag === '--x' || flag === '--y') {
        if (!/^(0|[1-9][0-9]*)$/.test(value) || Number(value) > 0xffffffff) {
          throw new Error(usage);
        }
        options[fields[flag]] = Number(value);
      } else options[fields[flag]] = value;
    }
  }
  if (!['granted', 'denied'].includes(options.profile) ||
      !['granted', 'denied'].includes(options.uiProfile) || !options.out ||
      options.x === null || options.y === null ||
      (options.profile === 'granted' && !options.note)) throw new Error(usage);
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

async function readComponent(path) {
  const handle = await open(path, 'r');
  try {
    const bytes = Buffer.alloc(4 * 1024 * 1024 + 1);
    let length = 0;
    while (length < bytes.length) {
      const read = await handle.read(bytes, length, bytes.length - length, length);
      if (read.bytesRead === 0) break;
      length += read.bytesRead;
    }
    if (length === bytes.length) throw new RangeError('A1 component exceeds 4 MiB');
    return bytes.subarray(0, length);
  } finally {
    await handle.close();
  }
}

export async function runInteraction({
  profile, out, note, x, y, uiProfile = 'granted', expanded = 1, debug = false,
  componentPath, componentWasm,
}) {
  if (!['granted', 'denied'].includes(profile) || !['granted', 'denied'].includes(uiProfile) ||
      typeof out !== 'string' || !out || out.includes('\0') ||
      (profile === 'granted' && (typeof note !== 'string' || !note)) ||
      !Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
      x < 0 || y < 0 || x > 0xffffffff || y > 0xffffffff ||
      ![0, 1].includes(expanded) || typeof debug !== 'boolean' ||
      (componentPath !== undefined && (typeof componentPath !== 'string' || !componentPath)) ||
      (componentWasm !== undefined && !(componentWasm instanceof Uint8Array)) ||
      (componentPath !== undefined && componentWasm !== undefined)) {
    throw new TypeError('Invalid A1 interaction arguments');
  }
  // Own caller-supplied bytes before any await so the saved component is the
  // same component whose behavior produced this receipt and frame.
  let component = componentWasm === undefined ? undefined : Buffer.from(componentWasm);
  const destination = resolve(out);
  await refuseExisting(destination);
  if (component === undefined) {
    component = componentPath === undefined ? compileUiService({
      executable: native('a1_ui_action'),
      source: await readFile(new URL('./activate.js', import.meta.url), 'utf8'),
    }) : await readComponent(componentPath);
  }
  const document = await loadNoteDocument({ profile, note });
  const action = activateNoteHeadingWithService({
    executable: native('a1_note_scene'), serviceExecutable: native('a1_ui_action'),
    component, document, expanded, x, y, uiProfile,
  });
  const receipt = {
    schemaVersion: 1, kind: 'a1-source-interaction', profile, uiProfile,
    pointer: { x, y }, heading: action.heading,
    activated: action.activated, changed: action.changed,
    attemptedReads: document.attemptedReads, backingReads: document.backingReads,
    cleanups: document.cleanups, headingCount: document.headings.length,
    beforeVisible: action.before.visibleCount, visible: action.after.visibleCount,
    service: action.service,
  };
  // Publish a complete experiment only after source, authority, action and
  // native frame checks succeed. This never rewrites the selected note.
  const stage = await mkdtemp(join(dirname(destination), '.grunkalunka-a1-action-'));
  try {
    await writeFile(join(stage, 'index.html'), action.after.html, 'utf8');
    await writeFile(join(stage, 'before.ppm'), action.previousPixels);
    await writeFile(join(stage, 'preview.ppm'), action.pixels);
    await writeFile(join(stage, 'action.component.wasm'), component);
    await writeFile(join(stage, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
    if (debug) {
      await writeFile(join(stage, 'debug.json'), JSON.stringify({
        schemaVersion: 1,
        beforeScene: action.before.input.toString('utf8'),
        afterScene: action.after.input.toString('utf8'),
      }, null, 2) + '\n');
    }
    await refuseExisting(destination);
    await rename(stage, destination);
  } catch (error) {
    await rm(stage, { recursive: true, force: true });
    throw error;
  }
  return { output: destination, ...receipt };
}

if (process.argv[1] &&
    realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))) {
  try {
    console.log(JSON.stringify(await runInteraction(parseInteractionArguments(process.argv.slice(2)))));
  } catch (error) {
    console.error('A1 interaction: ' + error.message);
    process.exitCode = 1;
  }
}
