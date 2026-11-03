// Private A1 input -> source-compiled component -> Rust/WIT state -> native frame.
import { spawnSync } from 'node:child_process';
import { prepareSelectedNoteScene } from './scene.mjs';

const MAX_COMPONENT = 4 * 1024 * 1024;
const COMPONENT_HEADER = Buffer.from([0, 97, 115, 109, 13, 0, 1, 0]);
const PPM_HEADER = Buffer.from('P6\n320 160\n255\n');
const PPM_SIZE = PPM_HEADER.length + 320 * 160 * 3;
const statuses = new Set(['ok', 'denied', 'invalid-count', 'invalid-expanded', 'invalid-result', 'call-limit']);

function nativeOutput(executable, args, input, maxBuffer) {
  if (typeof executable !== 'string' || !executable) {
    throw new TypeError('Expected a native A1 executable');
  }
  const result = spawnSync(executable, args, {
    input, shell: false, encoding: null, timeout: 60_000,
    killSignal: 'SIGKILL', maxBuffer,
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error('A1 native execution failed: ' +
      (result.stderr?.toString('utf8').trim() || result.error?.message ||
        result.signal || result.status));
  }
  return result.stdout;
}

function checkedComponent(component) {
  if (!(component instanceof Uint8Array) ||
      component.byteLength < COMPONENT_HEADER.length || component.byteLength > MAX_COMPONENT ||
      !Buffer.from(component.subarray(0, COMPONENT_HEADER.length)).equals(COMPONENT_HEADER)) {
    throw new TypeError('Expected bounded A1 component bytes');
  }
  return component;
}

export function compileUiService({ executable, source, language = 'js' }) {
  if (typeof source !== 'string' || !source || Buffer.byteLength(source, 'utf8') > 64 * 1024 ||
      !['js', 'ts'].includes(language)) {
    throw new TypeError('Invalid A1 service source');
  }
  return checkedComponent(nativeOutput(executable, ['--compile', language],
    Buffer.from(source, 'utf8'), MAX_COMPONENT + 1024));
}

export function invokeUiService({ executable, component, profile, count, expanded }) {
  checkedComponent(component);
  if (!['granted', 'denied'].includes(profile) ||
      !Number.isSafeInteger(count) || count < 0 || count > 64 || ![0, 1].includes(expanded)) {
    throw new TypeError('Invalid A1 service request');
  }
  const raw = nativeOutput(executable,
    ['--run', profile, String(count), String(expanded)], component, 16 * 1024);
  const receipt = JSON.parse(raw.toString('utf8'));
  if (receipt?.schemaVersion !== 1 || receipt.profile !== profile ||
      receipt.permittedCount !== count || !statuses.has(receipt.status) ||
      !Number.isSafeInteger(receipt.attempts) || receipt.attempts < 0 ||
      !Number.isSafeInteger(receipt.transitions) || receipt.transitions < 0 ||
      receipt.transitions > receipt.attempts || ![0, count].includes(receipt.visible) ||
      (receipt.status === 'ok' ? !Number.isFinite(receipt.result) :
        receipt.result !== null || receipt.transitions !== 0 || receipt.visible !== 0) ||
      (profile === 'denied' && receipt.transitions !== 0)) {
    throw new Error('Invalid A1 service state receipt');
  }
  return receipt;
}

export function paintNoteScene(executable, scene) {
  const pixels = nativeOutput(executable, ['--ppm'], scene.input, PPM_SIZE + 1024);
  if (pixels.length !== PPM_SIZE ||
      !pixels.subarray(0, PPM_HEADER.length).equals(PPM_HEADER)) {
    throw new Error('Invalid native A1 frame');
  }
  return pixels;
}

export function activateNoteHeadingWithService({
  executable, serviceExecutable, component, document, expanded, x, y, uiProfile = 'granted',
}) {
  checkedComponent(component);
  if (![0, 1].includes(expanded) || !['granted', 'denied'].includes(uiProfile) ||
      !Number.isSafeInteger(x) || !Number.isSafeInteger(y) ||
      x < 0 || y < 0 || x > 0xffffffff || y > 0xffffffff) {
    throw new TypeError('Invalid A1 service activation');
  }
  const before = prepareSelectedNoteScene({ ...document,
    visibleCount: expanded ? document.headings.length : 0 });
  const previousPixels = paintNoteScene(executable, before);
  const idle = { activated: false, changed: false, heading: null, before, after: before,
    previousPixels, pixels: previousPixels, service: null };
  const raw = nativeOutput(executable, ['--hit', String(x), String(y)],
    before.input, 4096).toString('utf8');
  if (raw === 'A1HIT1\tnone\n') return idle;
  const match = /^A1HIT1\t(0|[1-9][0-9]*)\t(0|[1-9][0-9]*)\t(0|[1-9][0-9]*)\t([1-6])\n$/.exec(raw);
  if (!match) throw new Error('Malformed native A1 hit response');
  const [index, start, end, level] = match.slice(1).map(Number);
  const heading = document.headings?.[index];
  if (document.profile !== 'granted' || index >= before.visibleCount ||
      !heading || heading.start !== start || heading.end !== end || heading.level !== level) {
    throw new Error('Native A1 hit disagrees with authorized WIT headings');
  }
  const service = invokeUiService({ executable: serviceExecutable, component,
    profile: uiProfile, count: document.headings.length, expanded });
  const selected = { ...idle, heading: { index, start, end, level }, service };
  // A failed or unused service cannot replace the current view with its initial
  // private state. Document authority and permission to change the view differ.
  if (service.status !== 'ok' || service.transitions === 0) return selected;
  const after = prepareSelectedNoteScene({ ...document, visibleCount: service.visible });
  return { ...selected, activated: true, changed: after.visibleCount !== before.visibleCount,
    after, pixels: paintNoteScene(executable, after) };
}
