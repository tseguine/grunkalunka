// Bounded input replay over one owned A1 document/view session. The retained
// component executes the source handler; the host records native frame output.
import { createHash } from 'node:crypto';
import { constants, realpathSync } from 'node:fs';
import { lstat, mkdir, mkdtemp, open, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadNoteDocument } from './run.mjs';
import { compileUiService } from './service-interaction.mjs';
import { createNoteSession } from './session.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const MAX_EVENTS = 256;
const MAX_EVENT_BYTES = 64 * 1024;
const MAX_COMPONENT_BYTES = 4 * 1024 * 1024;
const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const native = (directory, name) => join(directory, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const usage = 'usage: node examples/note-preview/session-run.mjs ' +
  '--profile granted|denied --out DIR --events INPUT.json [--note NOTE.md] ' +
  '[--ui-profile granted|denied] [--component HANDLER.wasm] [--collapsed] [--debug]';
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const framePath = sequence => 'frames/' + String(sequence).padStart(4, '0') + '.ppm';

export function parseSessionArguments(args) {
  const options = { profile: null, out: null, note: null, eventsPath: null,
    uiProfile: 'granted', expanded: 1, debug: false };
  const fields = { '--profile': 'profile', '--out': 'out', '--note': 'note',
    '--events': 'eventsPath', '--ui-profile': 'uiProfile', '--component': 'componentPath' };
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
      if (typeof value !== 'string' || !value || value.startsWith('--') || value.includes('\0')) {
        throw new Error(usage);
      }
      options[fields[flag]] = value;
    }
  }
  if (!['granted', 'denied'].includes(options.profile) ||
      !['granted', 'denied'].includes(options.uiProfile) || !options.out || !options.eventsPath ||
      (options.profile === 'granted' && !options.note)) throw new Error(usage);
  return options;
}

function ownedEvents(events) {
  if (!Array.isArray(events) || events.length > MAX_EVENTS) {
    throw new RangeError('A1 replay requires an array of at most 256 pointer events');
  }
  return Array.from(events, event => {
    if (event === null || typeof event !== 'object' || Array.isArray(event) ||
        Object.keys(event).sort().join(',') !== 'x,y' ||
        !Number.isSafeInteger(event.x) || !Number.isSafeInteger(event.y) ||
        event.x < 0 || event.y < 0 || event.x > 0xffffffff || event.y > 0xffffffff) {
      throw new TypeError('Each A1 event must contain only unsigned integer x and y');
    }
    return { x: event.x, y: event.y };
  });
}

export async function readBoundedSessionInput(path, maximum, label) {
  // On POSIX a FIFO can block inside open before fstat can reject it.
  const handle = await open(path, constants.O_RDONLY | (constants.O_NONBLOCK ?? 0));
  try {
    if (!(await handle.stat()).isFile()) throw new TypeError(label + ' must be a regular file');
    const bytes = Buffer.alloc(maximum + 1);
    let length = 0;
    while (length < bytes.length) {
      const result = await handle.read(bytes, length, bytes.length - length, length);
      if (result.bytesRead === 0) break;
      length += result.bytesRead;
    }
    if (length > maximum) throw new RangeError(label + ' exceeds its byte limit');
    return bytes.subarray(0, length);
  } finally {
    await handle.close();
  }
}

function checkComponent(bytes) {
  if (bytes.byteLength < COMPONENT_HEADER.length || bytes.byteLength > MAX_COMPONENT_BYTES ||
      !bytes.subarray(0, COMPONENT_HEADER.length).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
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

export async function runNoteSession({
  profile, out, note, events, eventsPath, uiProfile = 'granted', expanded = 1,
  debug = false, componentPath, componentWasm, binaryTarget = target,
}) {
  if (!['granted', 'denied'].includes(profile) || !['granted', 'denied'].includes(uiProfile) ||
      typeof out !== 'string' || !out || out.includes('\0') ||
      (profile === 'granted' && (typeof note !== 'string' || !note || note.includes('\0'))) ||
      ![0, 1].includes(expanded) || typeof debug !== 'boolean' ||
      typeof binaryTarget !== 'string' || !binaryTarget || binaryTarget.includes('\0') ||
      (eventsPath !== undefined && (typeof eventsPath !== 'string' || !eventsPath ||
        eventsPath.includes('\0'))) ||
      (events === undefined) === (eventsPath === undefined) ||
      (componentPath !== undefined && (typeof componentPath !== 'string' || !componentPath ||
        componentPath.includes('\0'))) ||
      (componentWasm !== undefined && (!(componentWasm instanceof Uint8Array) ||
        componentWasm.byteLength > MAX_COMPONENT_BYTES)) ||
      (componentPath !== undefined && componentWasm !== undefined)) {
    throw new TypeError('Invalid A1 session arguments');
  }
  // Copy all caller-owned mutable inputs before the first await.
  let inputEvents = events === undefined ? undefined : ownedEvents(events);
  let component = componentWasm === undefined ? undefined : Buffer.from(componentWasm);
  if (component !== undefined) checkComponent(component);
  const selectedTarget = resolve(binaryTarget);
  const destination = resolve(out);
  await refuseExisting(destination);
  if (inputEvents === undefined) {
    const bytes = await readBoundedSessionInput(eventsPath, MAX_EVENT_BYTES, 'A1 event input');
    inputEvents = ownedEvents(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)));
  }
  const serviceExecutable = native(selectedTarget, 'a1_ui_action');
  if (component === undefined) {
    component = componentPath === undefined ? compileUiService({
      executable: serviceExecutable,
      source: await readFile(new URL('./activate.js', import.meta.url), 'utf8'),
    }) : await readBoundedSessionInput(componentPath, MAX_COMPONENT_BYTES, 'A1 component');
    checkComponent(component);
  }
  const document = await loadNoteDocument({ profile, note, binaryTarget: selectedTarget });
  const session = createNoteSession({
    executable: native(selectedTarget, 'a1_note_scene'), serviceExecutable,
    component, document, expanded, uiProfile,
  });
  const initial = session.snapshot();
  const frames = [initial.pixels];
  const recorded = [];
  const debugScenes = debug ? [initial.scene.input.toString('utf8')] : undefined;
  for (const pointer of inputEvents) {
    const action = session.activate(pointer);
    frames.push(action.pixels);
    recorded.push({
      sequence: action.sequence, pointer, target: action.target,
      activated: action.activated, changed: action.changed,
      beforeExpanded: action.beforeExpanded, expanded: action.expanded,
      beforeVisible: action.before.visibleCount, visible: action.after.visibleCount,
      service: action.service, frame: framePath(action.sequence),
      frameSha256: sha256(action.pixels),
    });
    if (debug) debugScenes.push(action.after.input.toString('utf8'));
  }
  const final = session.snapshot();
  const receipt = {
    schemaVersion: 1, kind: 'a1-source-session', profile, uiProfile,
    attemptedReads: document.attemptedReads, backingReads: document.backingReads,
    cleanups: document.cleanups, headingCount: document.headings.length,
    eventsProcessed: final.eventsProcessed,
    initialExpanded: initial.expanded, expanded: final.expanded,
    initialVisible: initial.visibleCount, visible: final.visibleCount,
    initialFrame: framePath(0), initialFrameSha256: sha256(initial.pixels),
    finalFrame: framePath(final.eventsProcessed), finalFrameSha256: sha256(final.pixels),
    events: recorded,
  };
  // No output is published if any event traps or its native repaint fails.
  const stage = await mkdtemp(join(dirname(destination), '.grunkalunka-a1-session-'));
  try {
    await mkdir(join(stage, 'frames'));
    for (const [sequence, pixels] of frames.entries()) {
      await writeFile(join(stage, framePath(sequence)), pixels);
    }
    await writeFile(join(stage, 'before.ppm'), initial.pixels);
    await writeFile(join(stage, 'preview.ppm'), final.pixels);
    await writeFile(join(stage, 'index.html'), final.scene.html, 'utf8');
    await writeFile(join(stage, 'action.component.wasm'), component);
    await writeFile(join(stage, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
    if (debug) {
      await writeFile(join(stage, 'debug.json'), JSON.stringify({
        schemaVersion: 1, scenes: debugScenes,
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
    console.log(JSON.stringify(await runNoteSession(parseSessionArguments(process.argv.slice(2)))));
  } catch (error) {
    console.error('A1 session: ' + error.message);
    process.exitCode = 1;
  }
}
