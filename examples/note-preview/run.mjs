// A1-only runnable host. Source and capabilities remain explicit; the guest
// receives only in-memory bytes, never a filesystem or ambient Node binding.
import { spawnSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { open, lstat, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareNoteScene } from './scene.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? 'target');
const native = (name, binaryTarget = target) => join(binaryTarget, 'debug', 'examples',
  name + (process.platform === 'win32' ? '.exe' : ''));
const compiler = join(target, 'debug',
  'grunkalunka' + (process.platform === 'win32' ? '.exe' : ''));
const guest = join(target, 'wasm32-unknown-unknown', 'debug', 'examples', 'outline_guest.wasm');
const MAX_NOTE = 4096;
const MAX_REPORT = 128 * 1024;
const PPM_HEADER = Buffer.from('P6\n320 160\n255\n');
const usage = 'usage: node examples/note-preview/run.mjs ' +
  '--profile granted|denied --out DIR [--note NOTE.md] [--collapsed] [--debug]';

export function parseArguments(args) {
  const options = { profile: null, out: null, note: null, expanded: 1, debug: false };
  const seen = new Set();
  for (let i = 0; i < args.length; i++) {
    const name = args[i];
    if (!['--profile', '--out', '--note', '--collapsed', '--debug'].includes(name) ||
      seen.has(name)) throw new Error(usage);
    seen.add(name);
    if (name === '--collapsed' || name === '--debug') {
      if (name === '--collapsed') options.expanded = 0;
      else options.debug = true;
    } else {
      const value = args[++i];
      if (!value || value.startsWith('--') || value.includes('\0')) throw new Error(usage);
      if (name === '--profile') options.profile = value;
      else if (name === '--out') options.out = value;
      else options.note = value;
    }
  }
  if (!['granted', 'denied'].includes(options.profile) || !options.out ||
    (options.profile === 'granted' && !options.note)) throw new Error(usage);
  return options;
}

async function boundedNote(path, profile) {
  if (profile === 'denied') return Buffer.alloc(0); // No filesystem metadata/access at all.
  const handle = await open(path, 'r');
  try {
    const buffer = Buffer.alloc(MAX_NOTE + 1);
    let length = 0;
    while (length < buffer.length) {
      const result = await handle.read(buffer, length, buffer.length - length, length);
      if (result.bytesRead === 0) break;
      length += result.bytesRead;
    }
    if (length > MAX_NOTE) throw new RangeError('Markdown note exceeds 4096 UTF-8 bytes');
    const bytes = buffer.subarray(0, length);
    new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return bytes;
  } finally {
    await handle.close();
  }
}

function nativeCall(command, args, input, maxBuffer = MAX_REPORT) {
  const result = spawnSync(command, args, {
    cwd: root, input, shell: false, encoding: null, timeout: 60_000,
    maxBuffer, killSignal: 'SIGKILL',
  });
  if (result.error || result.signal || result.status !== 0) {
    const detail = result.stderr?.toString('utf8').trim();
    throw new Error('A1 native execution failed: ' +
      (detail || result.error?.message || result.signal || result.status));
  }
  return result.stdout;
}

function decodedHex(value, label) {
  if (typeof value !== 'string' || value.length % 2 ||
    !/^(?:[0-9a-f]{2})*$/.test(value)) throw new Error('Invalid ' + label);
  return Buffer.from(value, 'hex');
}

function liftedResult(report, expectedProfile, bytes) {
  if (report.schemaVersion !== 1 || !Array.isArray(report.profiles) ||
    report.profiles.length !== 1) throw new Error('Expected one WIT profile, not a dual grant');
  const profile = report.profiles[0];
  if (profile?.profile !== expectedProfile || profile.attempts !== 1 ||
    profile.cleanups !== 1 || profile.backingReads !== (expectedProfile === 'granted' ? 1 : 0) ||
    !Array.isArray(profile.headings)) {
    throw new Error('Invalid WIT authority/cleanup result');
  }
  if (expectedProfile === 'denied') {
    if (profile.headings.length || Object.hasOwn(profile, 'sourceHex')) {
      throw new Error('Denied profile disclosed protected bytes');
    }
    return { profile: 'denied', sourceBytes: Buffer.alloc(0),
      headings: [], backingReads: profile.backingReads,
      attemptedReads: profile.attempts, cleanups: profile.cleanups };
  }
  const recovered = decodedHex(profile.sourceHex, 'source bytes');
  if (!recovered.equals(bytes)) throw new Error('WIT granted source differs from supplied bytes');
  const headings = profile.headings.map(record => ({
    level: record.level, start: record.start, end: record.end,
    sectionEnd: record.sectionEnd, parent: record.parent,
    text: new TextDecoder('utf-8', { fatal: true })
      .decode(decodedHex(record.textHex, 'WIT heading')),
  }));
  return { profile: 'granted', sourceBytes: bytes,
    headings, backingReads: profile.backingReads,
    attemptedReads: profile.attempts, cleanups: profile.cleanups };
}

function checkedPpm(bytes) {
  if (!bytes.subarray(0, PPM_HEADER.length).equals(PPM_HEADER) ||
    bytes.length !== PPM_HEADER.length + 320 * 160 * 3) {
    throw new Error('Invalid native RGB PPM result');
  }
  return bytes;
}

export async function runPreview({
  profile, out, note, expanded = 1, debug = false, selectorWasm,
}) {
  if (!['granted', 'denied'].includes(profile) || typeof out !== 'string' || !out ||
    (profile === 'granted' && (typeof note !== 'string' || !note))) {
    throw new TypeError('Invalid note-preview application arguments');
  }
  if (![0, 1].includes(expanded) || typeof debug !== 'boolean') {
    throw new TypeError('Invalid note-preview visibility/debug setting');
  }
  if (selectorWasm !== undefined && !(selectorWasm instanceof Uint8Array)) {
    throw new TypeError('Packaged selector must be an owned Wasm byte array');
  }
  // A packaged runner uses its own copied target tree regardless of the
  // developer machine's CARGO_TARGET_DIR. The normal source workflow keeps
  // its existing explicit target-directory override.
  const binaryTarget = selectorWasm === undefined ? target : resolve(root, 'target');
  const destination = resolve(out);
  // No overwrite of a previous successful app output.
  try {
    await lstat(destination);
    throw new Error('Output directory already exists');
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  const bytes = await boundedNote(note, profile);
  const selectedGuest = selectorWasm === undefined ? guest : join(
    binaryTarget, 'wasm32-unknown-unknown', 'debug', 'examples', 'outline_guest.wasm');
  const raw = nativeCall(native('a1_note_preview', binaryTarget),
    [selectedGuest, '--profile', profile], bytes);
  const lifted = liftedResult(JSON.parse(raw.toString('utf8')), profile, bytes);
  let wasm = selectorWasm;
  if (wasm === undefined) {
    const toggle = await readFile(new URL('./toggle.js', import.meta.url), 'utf8');
    const { compileSource } = await import('../../tools/build/compile.mjs');
    ({ wasm } = await compileSource({ compiler, source: toggle, language: 'js', cwd: root }));
  }
  if (!WebAssembly.validate(wasm)) throw new Error('Compiler did not emit valid selector Wasm');
  const scene = await prepareNoteScene({ wasm, expanded, ...lifted });
  const frame = nativeCall(native('a1_note_scene', binaryTarget),
    ['--report'], scene.input).toString('utf8');
  if (!frame.startsWith('A1FRAME1\t' + profile + '\t')) {
    throw new Error('Native frame profile differs from authorized note');
  }
  const ppm = checkedPpm(nativeCall(native('a1_note_scene', binaryTarget),
    ['--ppm'], scene.input, 512 * 1024));

  // Only publish complete, checked outputs; failures leave the requested
  // destination absent. This is an output directory, not a persistent vault.
  const staging = await mkdtemp(join(dirname(destination), '.grunkalunka-a1-'));
  try {
    await writeFile(join(staging, 'index.html'), scene.html, 'utf8');
    await writeFile(join(staging, 'preview.ppm'), ppm);
    if (debug) {
      await writeFile(join(staging, 'debug.json'), JSON.stringify({
        schemaVersion: 1, profile, expanded: Boolean(expanded),
        attemptedReads: lifted.attemptedReads, backingReads: lifted.backingReads,
        cleanups: lifted.cleanups,
        headingCount: lifted.headings.length, visibleHeadingCount: scene.visibleCount,
        nativeFrame: frame,
      }, null, 2) + '\n');
    }
    // Original Markdown bytes and input path are not copied into output
    // files. Granted heading projections are intentionally visible.
    await rename(staging, destination);
  } catch (error) {
    await rm(staging, { recursive: true, force: true });
    throw error;
  }
  return { profile, output: destination, headings: lifted.headings.length,
    visible: scene.visibleCount };
}

// Node canonicalizes the loaded module URL, but argv may retain a symlinked path
// (notably macOS /var -> /private/var). Compare file identities, not URLs.
if (process.argv[1] &&
    realpathSync(process.argv[1]) === realpathSync(fileURLToPath(import.meta.url))) {
  try {
    console.log(JSON.stringify(await runPreview(parseArguments(process.argv.slice(2)))));
  } catch (error) {
    console.error('A1 note preview: ' + error.message);
    process.exitCode = 1;
  }
}
